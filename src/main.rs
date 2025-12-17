use clap::Parser;
use fs2::FileExt;
use gleam_observer::{App, Config, Result};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "gleam")]
#[command(author, version, about = "Universal Hardware Monitor", long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        help = "Refresh interval in milliseconds",
        default_value = "1000"
    )]
    refresh_rate: u64,

    #[arg(short, long, help = "Path to custom config file")]
    config: Option<String>,

    #[arg(long, help = "Headless mode (JSON output)")]
    headless: bool,

    #[arg(long, help = "Export format (csv, json)", value_name = "FORMAT")]
    export: Option<String>,

    #[arg(long, help = "Disable GPU monitoring")]
    no_gpu: bool,

    #[arg(short, long, help = "Verbose logging")]
    verbose: bool,

    #[arg(
        long,
        help = "Export history to file (csv or json)",
        value_name = "FILE"
    )]
    export_history: Option<String>,

    #[arg(long, help = "Run as daemon with system tray")]
    tray: bool,

    #[arg(long, help = "Run UI only (no daemon)")]
    ui_only: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    log::info!("Starting GleamObserver v{}", env!("CARGO_PKG_VERSION"));

    let mut config = if let Some(config_path) = args.config {
        log::info!("Loading config from: {}", config_path);
        Config::load()?
    } else {
        Config::load().unwrap_or_default()
    };

    if args.refresh_rate != 1000 {
        config.refresh.interval_ms = args.refresh_rate;
    }

    // Prevent multiple interactive sessions from piling up. Hold the lock for the
    // lifetime of the process to keep exclusivity.
    let _tui_lock = if !args.headless && !args.tray {
        Some(ensure_single_tui_instance()?)
    }
    else {
        None
    };

    let enable_gpu = !args.no_gpu;

    if args.tray {
        #[cfg(all(unix, feature = "systray"))]
        {
            log::info!("Starting in daemon/tray mode");
            return gleam_observer::daemon::DaemonContext::start(config);
        }

        #[cfg(not(all(unix, feature = "systray")))]
        {
            log::error!("Tray mode requires --features systray on Unix systems");
            return Err(gleam_observer::error::Error::Daemon(
                "Systray feature not enabled".to_string(),
            ));
        }
    } else if args.headless {
        run_headless(config, args.export, enable_gpu)?;
    } else {
        run_tui(config, enable_gpu)?;
    }

    Ok(())
}

fn ensure_single_tui_instance() -> Result<std::fs::File> {
    let lock_path = lock_file_path();

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&lock_path)
        .map_err(|e| gleam_observer::error::Error::Tui(format!("Failed to open lock file {:?}: {}", lock_path, e)))?;

    // Try to acquire an exclusive lock; if it fails, report the existing PID.
    if let Err(e) = file.try_lock_exclusive() {
        let mut contents = String::new();
        let _ = std::fs::File::open(&lock_path).and_then(|mut f| f.read_to_string(&mut contents));
        let pid_info = contents.trim();
        return Err(gleam_observer::error::Error::Tui(format!(
            "Another GleamObserver session is already running (pid {}). Close it with 'q' before starting a new one. ({})",
            if pid_info.is_empty() { "unknown" } else { pid_info },
            e
        )));
    }

    // Record our PID for visibility.
    file.set_len(0).ok();
    let _ = write!(std::io::BufWriter::new(&file), "{}", std::process::id());

    Ok(file)
}

fn lock_file_path() -> PathBuf {
    std::env::temp_dir().join("gleam_observer_tui.lock")
}

fn run_headless(config: Config, export_format: Option<String>, enable_gpu: bool) -> Result<()> {
    log::info!("Running in headless mode");

    let mut app = App::new(config, enable_gpu)?;

    if let Some(gpu_manager) = &app.gpu {
        for (i, gpu_info) in gpu_manager.get_info().iter().enumerate() {
            log::info!("GPU {}: {} - {}", i, gpu_info.vendor, gpu_info.name);
        }
    }

    loop {
        app.update()?;

        match export_format.as_deref() {
            Some("json") => {
                let gpu_data = if let Some(gpu_manager) = &app.gpu {
                    let gpus: Vec<String> = gpu_manager.get_info().iter()
                        .map(|g| format!(
                            "{{\"name\": \"{}\", \"vendor\": \"{}\", \"temp\": {}, \"util\": {}, \"mem_used_mb\": {}, \"mem_total_mb\": {}}}",
                            g.name,
                            g.vendor,
                            g.temperature.map(|t| format!("{:.1}", t)).unwrap_or_else(|| "null".to_string()),
                            g.utilization.map(|u| format!("{:.1}", u)).unwrap_or_else(|| "null".to_string()),
                            g.memory_used.map(|m| (m / 1024 / 1024).to_string()).unwrap_or_else(|| "null".to_string()),
                            g.memory_total.map(|m| (m / 1024 / 1024).to_string()).unwrap_or_else(|| "null".to_string())
                        ))
                        .collect();
                    format!(", \"gpus\": [{}]", gpus.join(", "))
                } else {
                    String::new()
                };

                println!(
                    "{{\"cpu\": {:.2}, \"memory\": {:.2}{}}}",
                    app.metrics.global_cpu_usage(),
                    app.metrics.memory_usage_percent(),
                    gpu_data
                );
            }
            Some("csv") => {
                let gpu_csv = if let Some(gpu_manager) = &app.gpu {
                    let gpu_vals: Vec<String> = gpu_manager
                        .get_info()
                        .iter()
                        .map(|g| {
                            format!(
                                "{},{}",
                                g.temperature
                                    .map(|t| format!("{:.1}", t))
                                    .unwrap_or_else(|| "N/A".to_string()),
                                g.utilization
                                    .map(|u| format!("{:.1}", u))
                                    .unwrap_or_else(|| "N/A".to_string())
                            )
                        })
                        .collect();
                    if gpu_vals.is_empty() {
                        String::new()
                    } else {
                        format!(",{}", gpu_vals.join(","))
                    }
                } else {
                    String::new()
                };

                println!(
                    "{:.2},{:.2}{}",
                    app.metrics.global_cpu_usage(),
                    app.metrics.memory_usage_percent(),
                    gpu_csv
                );
            }
            _ => {
                print!(
                    "CPU: {:.2}% | Memory: {:.2}%",
                    app.metrics.global_cpu_usage(),
                    app.metrics.memory_usage_percent()
                );

                if let Some(gpu_manager) = &app.gpu {
                    for (i, gpu_info) in gpu_manager.get_info().iter().enumerate() {
                        print!(
                            " | GPU{}: {:.1}°C {:.1}%",
                            i,
                            gpu_info.temperature.unwrap_or(0.0),
                            gpu_info.utilization.unwrap_or(0.0)
                        );
                    }
                }
                println!();
            }
        }

        std::thread::sleep(Duration::from_millis(app.config.refresh.interval_ms));
    }
}

fn run_tui(config: Config, enable_gpu: bool) -> Result<()> {
    log::info!("Starting TUI mode");

    let app = App::new(config, enable_gpu)?;

    log::info!("CPU cores detected: {}", app.metrics.cpu_count());
    log::info!(
        "Total memory: {} MB",
        app.metrics.memory_total() / 1024 / 1024
    );

    if let Some(gpu_manager) = &app.gpu {
        log::info!("GPUs detected: {}", gpu_manager.gpu_count());
        for (i, gpu_info) in gpu_manager.get_info().iter().enumerate() {
            log::info!("  GPU {}: {} - {}", i, gpu_info.vendor, gpu_info.name);
        }
    } else {
        log::info!("No GPUs detected or GPU monitoring disabled");
    }

    gleam_observer::tui::run(app)?;

    Ok(())
}
