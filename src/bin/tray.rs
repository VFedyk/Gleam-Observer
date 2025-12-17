use gleam_observer::{Config, Result};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[cfg(all(unix, feature = "systray"))]
use tray_item::{IconSource, TrayItem};

const MONITORING_INTERVAL_MS: u64 = 1000;
const TRAY_UPDATE_INTERVAL_MS: u64 = 200;

#[derive(Clone)]
struct TrayState {
    alerts_paused: bool,
    cpu_percent: f32,
    mem_percent: f32,
}

impl TrayState {
    fn new() -> Self {
        Self {
            alerts_paused: false,
            cpu_percent: 0.0,
            mem_percent: 0.0,
        }
    }

    fn update_metrics(&mut self, cpu: f32, mem: f32) {
        self.cpu_percent = cpu;
        self.mem_percent = mem;
    }

    fn toggle_alerts(&mut self) {
        self.alerts_paused = !self.alerts_paused;
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("GleamObserver Daemon Starting (Systray Mode)");

    let config = Config::load().unwrap_or_default();

    #[cfg(all(unix, feature = "systray"))]
    {
        let state = Arc::new(Mutex::new(TrayState::new()));

        let state_monitor = state.clone();
        let config_clone = config.clone();
        thread::spawn(move || {
            if let Err(e) = run_monitoring_loop(state_monitor, config_clone) {
                log::error!("Monitoring loop error: {}", e);
            }
        });

        if let Err(e) = init_tray(state, config) {
            log::error!("Failed to initialize systray: {}", e);
            return Err(e);
        }
    }

    #[cfg(not(all(unix, feature = "systray")))]
    {
        log::error!("Systray feature not enabled. Please compile with --features systray");
        return Err(gleam_observer::error::Error::Daemon(
            "Systray feature not enabled".to_string(),
        ));
    }

    Ok(())
}

#[cfg(all(unix, feature = "systray"))]
fn run_monitoring_loop(state: Arc<Mutex<TrayState>>, config: Config) -> Result<()> {
    use gleam_observer::daemon::DaemonContext;

    let ctx = DaemonContext::new(config)?;

    loop {
        if gleam_observer::daemon::should_stop() {
            log::info!("Monitoring loop: stop signal received");
            break;
        }

        update_tray_state(&state, &ctx);

        thread::sleep(Duration::from_millis(MONITORING_INTERVAL_MS));
    }

    Ok(())
}

#[cfg(all(unix, feature = "systray"))]
fn update_tray_state(state: &Arc<Mutex<TrayState>>, ctx: &gleam_observer::daemon::DaemonContext) {
    if let Ok(mut s) = state.lock() {
        s.update_metrics(ctx.cpu_percent, ctx.mem_percent);
        s.alerts_paused = ctx.alerts_paused;
    }
}

#[cfg(all(unix, feature = "systray"))]
fn init_tray(state: Arc<Mutex<TrayState>>, _config: Config) -> Result<()> {
    let mut tray =
        TrayItem::new("GleamObserver", IconSource::Resource("gleamobserver")).map_err(|e| {
            gleam_observer::error::Error::Daemon(format!("Failed to create tray: {}", e))
        })?;

    let _state_dashboard = state.clone();
    tray.add_menu_item("Dashboard", move || {
        log::info!("User clicked: Dashboard");
        open_dashboard();
    })
    .map_err(|e| gleam_observer::error::Error::Daemon(format!("Failed to add menu item: {}", e)))?;

    tray.add_menu_item("", || {}).map_err(|e| {
        gleam_observer::error::Error::Daemon(format!("Failed to add separator: {}", e))
    })?;

    let state_status = state.clone();
    tray.add_menu_item("Show Status", move || {
        if let Ok(s) = state_status.lock() {
            gleam_observer::daemon::send_status_update(s.cpu_percent, s.mem_percent);
        }
    })
    .map_err(|e| gleam_observer::error::Error::Daemon(format!("Failed to add menu item: {}", e)))?;

    let state_pause = state.clone();
    tray.add_menu_item("Pause Alerts", move || {
        if let Ok(mut s) = state_pause.lock() {
            s.toggle_alerts();
            log::info!("Alerts paused: {}", s.alerts_paused);
        }
    })
    .map_err(|e| gleam_observer::error::Error::Daemon(format!("Failed to add menu item: {}", e)))?;

    tray.add_menu_item("Settings", || {
        log::info!("User clicked: Settings");
        open_settings();
    })
    .map_err(|e| gleam_observer::error::Error::Daemon(format!("Failed to add menu item: {}", e)))?;

    tray.add_menu_item("", || {}).map_err(|e| {
        gleam_observer::error::Error::Daemon(format!("Failed to add separator: {}", e))
    })?;

    tray.add_menu_item("Exit", || {
        log::info!("GleamObserver Daemon: User clicked Exit");
        gleam_observer::daemon::set_stop_flag();
        std::process::exit(0);
    })
    .map_err(|e| gleam_observer::error::Error::Daemon(format!("Failed to add menu item: {}", e)))?;

    log::info!("Systray initialized successfully");

    run_tray_event_loop()
}

#[cfg(all(unix, feature = "systray"))]
fn run_tray_event_loop() -> Result<()> {
    loop {
        thread::sleep(Duration::from_millis(TRAY_UPDATE_INTERVAL_MS));

        if gleam_observer::daemon::should_stop() {
            log::info!("Tray: stop signal received");
            break;
        }
    }

    Ok(())
}

#[cfg(all(unix, feature = "systray"))]
fn open_dashboard() {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home"));
    let binary_path = format!("{}/.local/bin/gleam", home);

    if let Err(e) = std::process::Command::new(binary_path).spawn() {
        log::error!("Failed to open dashboard: {}", e);
    }
}

#[cfg(all(unix, feature = "systray"))]
fn open_settings() {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home"));
    let config_path = format!("{}/.config/gleam-observer/config.toml", home);

    if let Err(e) = std::process::Command::new("xdg-open")
        .arg(config_path)
        .spawn()
    {
        log::error!("Failed to open settings: {}", e);
    }
}
