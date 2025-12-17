use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::Modifier,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, BorderType},
    Frame,
};
use crate::app::{App, ViewMode, ProcessSortMode, DialogMode};
use crate::alerts::AlertLevel;
use super::theme::CatppuccinTheme as Theme;

pub fn draw(f: &mut Frame, app: &App) {
    match app.view_mode {
        ViewMode::Dashboard => draw_dashboard(f, app),
        ViewMode::Processes => draw_processes_view(f, app),
        ViewMode::History => draw_history_view(f, app),
    }
    
    // Draw dialogs on top
    match app.dialog_mode {
        DialogMode::ConfirmKill => draw_confirm_dialog(f, "Kill Process", "Send SIGKILL?", app),
        DialogMode::ConfirmTerminate => draw_confirm_dialog(f, "Terminate Process", "Send SIGTERM?", app),
        DialogMode::ProcessInfo => draw_info_dialog(f, app),
        DialogMode::None => {}
    }
}

fn draw_dashboard(f: &mut Frame, app: &App) {
    let has_alerts = app.has_alerts();
    
    let constraints = if has_alerts {
        vec![
            Constraint::Length(3),  // Header
            Constraint::Length(5),  // Alerts
            Constraint::Min(10),    // Main content
            Constraint::Length(3),  // Footer
        ]
    } else {
        vec![
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Main content
            Constraint::Length(3),  // Footer
        ]
    };
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    let mut idx = 0;
    draw_header(f, chunks[idx], app);
    idx += 1;
    
    if has_alerts {
        draw_alerts_panel(f, chunks[idx], app);
        idx += 1;
    }
    
    draw_main_content(f, chunks[idx], app);
    idx += 1;
    
    draw_footer(f, chunks[idx], app);
}

fn draw_header(f: &mut Frame, area: Rect, _app: &App) {
    let title = vec![
        Line::from(vec![
            Span::raw("  "),
            Span::styled("◆", Style::default().fg(Theme::MAUVE)),
            Span::raw(" "),
            Span::styled("GleamObserver", Style::default()
                .fg(Theme::LAVENDER)
                .add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled("◆", Style::default().fg(Theme::MAUVE)),
            Span::raw("  "),
            Span::styled("Universal Hardware Monitor", Style::default()
                .fg(Theme::SUBTEXT0)
                .add_modifier(Modifier::ITALIC)),
        ]),
    ];
    
    let header = Paragraph::new(title)
        .alignment(Alignment::Center)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::MAUVE))
            .style(Style::default().bg(Theme::MANTLE)));
    
    f.render_widget(header, area);
}

fn draw_main_content(f: &mut Frame, area: Rect, app: &App) {
    let has_gpu = app.gpu.is_some();
    // Always show trends panel if enabled, even when empty
    let show_trends = app.config.trends.enabled;
    
    let constraints = if has_gpu && show_trends {
        vec![
            Constraint::Percentage(22),  // CPU
            Constraint::Percentage(20),  // Memory
            Constraint::Percentage(30),  // GPU + Info combined
            Constraint::Percentage(28),  // Trends
        ]
    } else if has_gpu {
        vec![
            Constraint::Percentage(25),  // CPU
            Constraint::Percentage(25),  // Memory
            Constraint::Percentage(50),  // GPU + Info combined
        ]
    } else if show_trends {
        vec![
            Constraint::Percentage(25),  // CPU
            Constraint::Percentage(25),  // Memory
            Constraint::Percentage(25),  // Info
            Constraint::Percentage(25),  // Trends
        ]
    } else {
        vec![
            Constraint::Percentage(35),  // CPU
            Constraint::Percentage(30),  // Memory
            Constraint::Percentage(35),  // Info
        ]
    };
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut idx = 0;
    draw_cpu_panel(f, chunks[idx], app);
    idx += 1;
    
    draw_memory_panel(f, chunks[idx], app);
    idx += 1;
    
    if has_gpu {
        // Split GPU + Info horizontally
        let gpu_info_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[idx]);
        
        draw_gpu_panel(f, gpu_info_chunks[0], app);
        draw_info_panel(f, gpu_info_chunks[1], app);
        idx += 1;
    } else {
        draw_info_panel(f, chunks[idx], app);
        idx += 1;
    }
    
    if show_trends {
        draw_trends_panel(f, chunks[idx], app);
    }
}

fn draw_cpu_panel(f: &mut Frame, area: Rect, app: &App) {
    use ratatui::widgets::Sparkline;
    
    let cpu_usage = app.metrics.global_cpu_usage();
    let cpu_color = Theme::cpu_color(cpu_usage);
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);
    
    // Gauge
    let label = format!("{:.1}%", cpu_usage);
    let gauge = Gauge::default()
        .block(Block::default()
            .title(vec![
                Span::styled("╭─ ", Style::default().fg(Theme::BLUE)),
                Span::styled("CPU", Style::default()
                    .fg(Theme::BLUE)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" ─╮", Style::default().fg(Theme::BLUE)),
            ])
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::BLUE))
            .style(Style::default().bg(Theme::MANTLE)))
        .gauge_style(Style::default().fg(cpu_color).bg(Theme::SURFACE0))
        .label(Span::styled(label, Style::default()
            .fg(Theme::TEXT)
            .add_modifier(Modifier::BOLD)))
        .percent(cpu_usage.min(100.0) as u16);
    
    f.render_widget(gauge, chunks[0]);
    
    // Sparkline
    let history_data: Vec<u64> = app.history.cpu_usage.get_values()
        .iter()
        .map(|v| *v as u64)
        .collect();
    
    if !history_data.is_empty() {
        let sparkline = Sparkline::default()
            .block(Block::default()
                .title("History")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Theme::BLUE))
                .style(Style::default().bg(Theme::MANTLE)))
            .data(&history_data)
            .style(Style::default().fg(Theme::BLUE));
        
        f.render_widget(sparkline, chunks[1]);
    }
}

fn draw_memory_panel(f: &mut Frame, area: Rect, app: &App) {
    use ratatui::widgets::Sparkline;
    let mem_percent = app.metrics.memory_usage_percent();
    let mem_used = app.metrics.memory_used() / 1024 / 1024;
    let mem_total = app.metrics.memory_total() / 1024 / 1024;
    let mem_color = Theme::memory_color(mem_percent);
    
    let swap_used = app.metrics.swap_used() / 1024 / 1024;
    let swap_total = app.metrics.swap_total() / 1024 / 1024;
    let swap_percent = app.metrics.swap_usage_percent();
    
    let has_history = !app.history.memory_usage.is_empty();
    
    let constraints = if has_history {
        vec![Constraint::Percentage(40), Constraint::Percentage(20), Constraint::Percentage(40)]
    } else {
        vec![Constraint::Percentage(50), Constraint::Percentage(50)]
    };
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    
    let mut idx = 0;
    
    // RAM gauge
    let _ram_label = format!("{} MB / {} MB", mem_used, mem_total);
    let ram_gauge = Gauge::default()
        .block(Block::default()
            .title(vec![
                Span::styled("╭─ ", Style::default().fg(Theme::TEAL)),
                Span::styled("RAM", Style::default()
                    .fg(Theme::TEAL)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" ─╮", Style::default().fg(Theme::TEAL)),
            ])
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::TEAL))
            .style(Style::default().bg(Theme::MANTLE)))
        .gauge_style(Style::default().fg(mem_color).bg(Theme::SURFACE0))
        .label(Span::styled(
            format!("{:.1}%", mem_percent),
            Style::default().fg(Theme::TEXT).add_modifier(Modifier::BOLD)
        ))
        .percent(mem_percent.min(100.0) as u16);
    
    f.render_widget(ram_gauge, chunks[idx]);
    idx += 1;
    
    // Memory history sparkline
    if has_history {
        let history_data: Vec<u64> = app.history.memory_usage.get_values()
            .iter()
            .map(|v| *v as u64)
            .collect();
        
        let sparkline = Sparkline::default()
            .block(Block::default()
                .title("Mem History")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Theme::TEAL))
                .style(Style::default().bg(Theme::MANTLE)))
            .data(&history_data)
            .style(Style::default().fg(Theme::TEAL));
        
        f.render_widget(sparkline, chunks[idx]);
        idx += 1;
    }
    
    // SWAP gauge
    if swap_total > 0 {
        let _swap_label = format!("{} MB / {} MB", swap_used, swap_total);
        let swap_gauge = Gauge::default()
            .block(Block::default()
                .title(vec![
                    Span::styled("╭─ ", Style::default().fg(Theme::SKY)),
                    Span::styled("SWAP", Style::default()
                        .fg(Theme::SKY)
                        .add_modifier(Modifier::BOLD)),
                    Span::styled(" ─╮", Style::default().fg(Theme::SKY)),
                ])
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Theme::SKY))
                .style(Style::default().bg(Theme::MANTLE)))
            .gauge_style(Style::default().fg(Theme::SAPPHIRE).bg(Theme::SURFACE0))
            .label(Span::styled(
                format!("{:.1}%", swap_percent),
                Style::default().fg(Theme::TEXT).add_modifier(Modifier::BOLD)
            ))
            .percent(swap_percent.min(100.0) as u16);
        
        f.render_widget(swap_gauge, chunks[idx]);
    }
}

fn draw_gpu_panel(f: &mut Frame, area: Rect, app: &App) {
    if app.gpu.is_some() {
        let gpu_info = &app.gpu_info_cache; // Use cached data!
        
        if gpu_info.is_empty() {
            let text = Paragraph::new(vec![
                Line::from(Span::styled("No GPU data available", 
                    Style::default().fg(Theme::SUBTEXT0).add_modifier(Modifier::ITALIC))),
            ])
            .alignment(Alignment::Center)
            .block(Block::default()
                .title(vec![
                    Span::styled("╭─ ", Style::default().fg(Theme::PEACH)),
                    Span::styled("GPU", Style::default()
                        .fg(Theme::PEACH)
                        .add_modifier(Modifier::BOLD)),
                    Span::styled(" ─╮", Style::default().fg(Theme::PEACH)),
                ])
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Theme::PEACH))
                .style(Style::default().bg(Theme::MANTLE)));
            f.render_widget(text, area);
            return;
        }

        let lines: Vec<Line> = gpu_info.iter().enumerate().flat_map(|(i, gpu)| {
            let temp = gpu.temperature.unwrap_or(0.0);
            let temp_color = Theme::gpu_temp_color(temp);
            let temp_str = if gpu.temperature.is_some() {
                format!("{:.1}°C", temp)
            } else {
                "N/A".to_string()
            };
            
            let util_str = gpu.utilization.map(|u| format!("{:.1}%", u)).unwrap_or_else(|| "N/A".to_string());
            let mem_percent = gpu.memory_usage_percent().unwrap_or(0.0);
            let mem_str = match (gpu.memory_used, gpu.memory_total) {
                (Some(used), Some(total)) => format!("{} MB / {} MB ({:.1}%)", 
                    used / 1024 / 1024, 
                    total / 1024 / 1024,
                    mem_percent),
                _ => "N/A".to_string(),
            };

            let mut lines = vec![
                Line::from(vec![
                    Span::styled(format!(" ◆ GPU {} ", i), Style::default()
                        .fg(Theme::MAUVE)
                        .add_modifier(Modifier::BOLD)),
                    Span::styled(&gpu.vendor, Style::default().fg(Theme::LAVENDER)),
                    Span::raw(" - "),
                    Span::styled(&gpu.name, Style::default().fg(Theme::TEXT)),
                ]),
                Line::from(vec![
                    Span::raw("   ├─ "),
                    Span::styled("Temp: ", Style::default().fg(Theme::SUBTEXT0)),
                    Span::styled(temp_str, Style::default().fg(temp_color).add_modifier(Modifier::BOLD)),
                    Span::raw("  "),
                    Span::styled("Usage: ", Style::default().fg(Theme::SUBTEXT0)),
                    Span::styled(util_str, Style::default().fg(Theme::GREEN).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::raw("   ├─ "),
                    Span::styled("Memory: ", Style::default().fg(Theme::SUBTEXT0)),
                    Span::styled(mem_str, Style::default().fg(Theme::PINK)),
                ]),
            ];
            
            // Add power info if available
            if gpu.power_draw.is_some() || gpu.power_limit.is_some() {
                let mut power_parts = vec![
                    Span::raw("   ├─ "),
                    Span::styled("Power: ", Style::default().fg(Theme::SUBTEXT0)),
                ];
                
                if let Some(power) = gpu.power_draw {
                    power_parts.push(Span::styled(format!("{:.1}W", power), 
                        Style::default().fg(Theme::YELLOW).add_modifier(Modifier::BOLD)));
                    
                    if let Some(limit) = gpu.power_limit {
                        power_parts.push(Span::styled(format!("/{:.0}W", limit), 
                            Style::default().fg(Theme::SUBTEXT1)));
                    }
                    
                    if let Some(eff) = gpu.power_efficiency {
                        power_parts.push(Span::raw("  "));
                        power_parts.push(Span::styled("Eff: ", Style::default().fg(Theme::SUBTEXT0)));
                        power_parts.push(Span::styled(format!("{:.2}%%/W", eff), 
                            Style::default().fg(Theme::GREEN)));
                    }
                }
                
                lines.push(Line::from(power_parts));
            }
            
            // Add clocks and fan
            let has_clocks = gpu.clock_speed.is_some() || gpu.memory_clock.is_some() || gpu.fan_speed.is_some();
            if has_clocks {
                let mut clock_parts = vec![
                    Span::raw("   ├─ "),
                ];
                
                if let Some(core) = gpu.clock_speed {
                    clock_parts.push(Span::styled("Core: ", Style::default().fg(Theme::SUBTEXT0)));
                    clock_parts.push(Span::styled(format!("{}MHz", core), 
                        Style::default().fg(Theme::BLUE)));
                    clock_parts.push(Span::raw("  "));
                }
                
                if let Some(mem_clock) = gpu.memory_clock {
                    clock_parts.push(Span::styled("Mem: ", Style::default().fg(Theme::SUBTEXT0)));
                    clock_parts.push(Span::styled(format!("{}MHz", mem_clock), 
                        Style::default().fg(Theme::TEAL)));
                    clock_parts.push(Span::raw("  "));
                }
                
                if let Some(fan) = gpu.fan_speed {
                    clock_parts.push(Span::styled("Fan: ", Style::default().fg(Theme::SUBTEXT0)));
                    clock_parts.push(Span::styled(format!("{}%", fan), 
                        Style::default().fg(Theme::SKY)));
                }
                
                lines.push(Line::from(clock_parts));
            }
            
            // Add process count
            if !gpu.processes.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("   └─ "),
                    Span::styled(format!("Processes: {} active", gpu.processes.len()), 
                        Style::default().fg(Theme::LAVENDER).add_modifier(Modifier::BOLD)),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("   └─ "),
                    Span::styled("No GPU processes", Style::default().fg(Theme::OVERLAY0)),
                ]));
            }
            
            lines
        }).collect();

        let paragraph = Paragraph::new(lines)
            .block(Block::default()
                .title(vec![
                    Span::styled("╭─ ", Style::default().fg(Theme::PEACH)),
                    Span::styled("GPU", Style::default()
                        .fg(Theme::PEACH)
                        .add_modifier(Modifier::BOLD)),
                    Span::styled(" ─╮", Style::default().fg(Theme::PEACH)),
                ])
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Theme::PEACH))
                .style(Style::default().bg(Theme::MANTLE)));
        
        f.render_widget(paragraph, area);
    }
}

fn draw_info_panel(f: &mut Frame, area: Rect, app: &App) {
    use sysinfo::System;
    
    // Get system information
    let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());
    let os_name = System::long_os_version().unwrap_or_else(|| "Unknown OS".to_string());
    let kernel = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
    let uptime = System::uptime();
    
    // Format uptime
    let uptime_str = if uptime < 60 {
        format!("{}s", uptime)
    } else if uptime < 3600 {
        format!("{}m", uptime / 60)
    } else if uptime < 86400 {
        format!("{}h {}m", uptime / 3600, (uptime % 3600) / 60)
    } else {
        format!("{}d {}h", uptime / 86400, (uptime % 86400) / 3600)
    };
    
    // ASCII Art Logo - Cyberpunk Style
    let logo_lines = vec![
        "  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓",
        "  ▓▒  ◆ Gleam0bserver  ▒▓",
        "  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓",
    ];
    
    let mut lines = Vec::new();
    
    // Add logo
    for logo_line in logo_lines {
        lines.push(Line::from(Span::styled(
            logo_line,
            Style::default().fg(Theme::MAUVE).add_modifier(Modifier::BOLD)
        )));
    }
    
    lines.push(Line::from(""));
    
    // System info
    lines.push(Line::from(vec![
        Span::styled("🖥️  ", Style::default().fg(Theme::LAVENDER)),
        Span::styled(hostname, Style::default().fg(Theme::TEXT).add_modifier(Modifier::BOLD)),
    ]));
    
    lines.push(Line::from(vec![
        Span::styled("🐧 ", Style::default().fg(Theme::BLUE)),
        Span::styled(&os_name, Style::default().fg(Theme::SUBTEXT1)),
    ]));
    
    lines.push(Line::from(vec![
        Span::styled("⚙️  ", Style::default().fg(Theme::TEAL)),
        Span::styled(format!("Kernel {}", kernel), Style::default().fg(Theme::SUBTEXT1)),
    ]));
    
    lines.push(Line::from(""));
    
    lines.push(Line::from(vec![
        Span::styled("📊 ", Style::default().fg(Theme::PEACH)),
        Span::styled("CPU Cores: ", Style::default().fg(Theme::SUBTEXT0)),
        Span::styled(format!("{}", app.metrics.cpu_count()), 
            Style::default().fg(Theme::BLUE).add_modifier(Modifier::BOLD)),
    ]));
    
    lines.push(Line::from(vec![
        Span::styled("🔢 ", Style::default().fg(Theme::GREEN)),
        Span::styled("Processes: ", Style::default().fg(Theme::SUBTEXT0)),
        Span::styled(format!("{}", app.metrics.process_count()), 
            Style::default().fg(Theme::TEAL).add_modifier(Modifier::BOLD)),
    ]));
    
    lines.push(Line::from(vec![
        Span::styled("⏱️  ", Style::default().fg(Theme::YELLOW)),
        Span::styled("Uptime: ", Style::default().fg(Theme::SUBTEXT0)),
        Span::styled(uptime_str, 
            Style::default().fg(Theme::YELLOW).add_modifier(Modifier::BOLD)),
    ]));
    
    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default()
            .title(vec![
                Span::styled("╭─ ", Style::default().fg(Theme::GREEN)),
                Span::styled("System Info", Style::default()
                    .fg(Theme::GREEN)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" ─╮", Style::default().fg(Theme::GREEN)),
            ])
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::GREEN))
            .style(Style::default().bg(Theme::MANTLE)));
    
    f.render_widget(paragraph, area);
}

fn draw_alerts_panel(f: &mut Frame, area: Rect, app: &App) {
    let critical_count = app.critical_alert_count();
    let warning_count = app.warning_alert_count();
    
    let mut lines = Vec::new();
    
    if critical_count > 0 {
        lines.push(Line::from(vec![
            Span::styled(" ⚠ ", Style::default()
                .fg(Theme::CRUST)
                .bg(Theme::RED)
                .add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(format!("{} CRITICAL ALERT{}", critical_count, if critical_count > 1 { "S" } else { "" }),
                Style::default().fg(Theme::RED).add_modifier(Modifier::BOLD)),
        ]));
    }
    
    if warning_count > 0 {
        lines.push(Line::from(vec![
            Span::styled(" ⚠ ", Style::default()
                .fg(Theme::CRUST)
                .bg(Theme::PEACH)
                .add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(format!("{} WARNING{}", warning_count, if warning_count > 1 { "S" } else { "" }),
                Style::default().fg(Theme::PEACH).add_modifier(Modifier::BOLD)),
        ]));
    }
    
    // Show first alert message
    if !app.active_alerts.is_empty() {
        let first_alert = &app.active_alerts[0];
        let color = match first_alert.level {
            AlertLevel::Critical => Theme::RED,
            AlertLevel::Warning => Theme::PEACH,
            AlertLevel::Info => Theme::YELLOW,
        };
        
        lines.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(&first_alert.message, Style::default().fg(color)),
        ]));
    }
    
    let paragraph = Paragraph::new(lines)
        .block(Block::default()
            .title(vec![
                Span::styled("⚠ ", Style::default().fg(Theme::RED)),
                Span::styled("ALERTS", Style::default()
                    .fg(Theme::RED)
                    .add_modifier(Modifier::BOLD)),
            ])
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::RED))
            .style(Style::default().bg(Theme::MANTLE)));
    
    f.render_widget(paragraph, area);
}

fn draw_trends_panel(f: &mut Frame, area: Rect, app: &App) {
    use crate::trends::{TrendDirection, TrendSeverity};
    
    if app.active_trends.is_empty() {
        let text = Paragraph::new(vec![
            Line::from(Span::styled("No significant trends detected", 
                Style::default().fg(Theme::SUBTEXT0).add_modifier(Modifier::ITALIC))),
        ])
        .alignment(Alignment::Center)
        .block(Block::default()
            .title(vec![
                Span::styled("╭─ ", Style::default().fg(Theme::MAUVE)),
                Span::styled("Trends", Style::default()
                    .fg(Theme::MAUVE)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" ─╮", Style::default().fg(Theme::MAUVE)),
            ])
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::MAUVE))
            .style(Style::default().bg(Theme::MANTLE)));
        f.render_widget(text, area);
        return;
    }

    let mut sorted_trends = app.active_trends.clone();
    sorted_trends.sort_by(|a, b| b.severity.partial_cmp(&a.severity).unwrap());

    let mut lines: Vec<Line> = Vec::new();

    for (i, trend) in sorted_trends.iter().take(3).enumerate() {
        if i > 0 {
            lines.push(Line::from(""));
        }

        let icon = match trend.direction {
            TrendDirection::Increasing => "↗",
            TrendDirection::Decreasing => "↘",
            TrendDirection::Stable => "→",
            TrendDirection::Volatile => "〰",
        };

        let color = match trend.severity {
            TrendSeverity::Critical => Theme::RED,
            TrendSeverity::Warning => Theme::YELLOW,
            TrendSeverity::Info => Theme::BLUE,
        };

        let metric_name = format!("{}", trend.metric);
        let rate_str = if trend.rate_per_minute >= 0.0 {
            format!("+{:.1}%/min", trend.rate_per_minute)
        } else {
            format!("{:.1}%/min", trend.rate_per_minute)
        };

        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", icon), 
                Style::default().fg(color).add_modifier(Modifier::BOLD)),
            Span::styled(metric_name, Style::default().fg(Theme::TEXT)),
            Span::raw(" "),
            Span::styled(rate_str, Style::default().fg(color)),
        ]));

        if let Some(time) = trend.time_to_threshold {
            let time_str = if time < 60 {
                format!("{}s", time)
            } else if time < 3600 {
                format!("{}m", time / 60)
            } else {
                format!("{}h", time / 3600)
            };

            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled("⏰ ", Style::default().fg(Theme::PEACH)),
                Span::styled(format!("Threshold in {}", time_str),
                    Style::default().fg(Theme::SUBTEXT1)),
            ]));
        }

        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled("🔮 ", Style::default().fg(Theme::MAUVE)),
            Span::styled(format!("5min: {:.1}%", trend.predicted_value_5min),
                Style::default().fg(Theme::SUBTEXT1)),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default()
            .title(vec![
                Span::styled("╭─ ", Style::default().fg(Theme::MAUVE)),
                Span::styled("Trends", Style::default()
                    .fg(Theme::MAUVE)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" ─╮", Style::default().fg(Theme::MAUVE)),
            ])
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::MAUVE))
            .style(Style::default().bg(Theme::MANTLE)));

    f.render_widget(paragraph, area);
}

fn draw_processes_view(f: &mut Frame, app: &App) {
    use ratatui::widgets::{Table, Row, Cell};
    
    let show_filter_bar = app.filter_mode || !app.filter_input.is_empty();

    let constraints = if show_filter_bar {
        vec![
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Process table
            Constraint::Length(3),  // Filter/search bar (taller for visibility)
            Constraint::Length(3),  // Footer
        ]
    } else {
        vec![
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Process table
            Constraint::Length(3),  // Footer
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    draw_header(f, chunks[0], app);
    
    // Process table
    let sort_indicator = match app.process_sort {
        ProcessSortMode::Cpu => "CPU ▼",
        ProcessSortMode::Memory => "MEM ▼",
        ProcessSortMode::Name => "NAME ▼",
        ProcessSortMode::Pid => "PID ▼",
    };
    
    let processes = app.current_processes();
    let total_processes = processes.len();
    let selected = if total_processes == 0 {
        0
    } else {
        app.selected_process_index.min(total_processes.saturating_sub(1))
    };

    // Determine how many rows can be shown inside the table area.
    let table_height = chunks[1].height as usize;
    let visible_rows = table_height.saturating_sub(4).max(1); // borders + header + margin

    // Calculate the first row to display so the selection stays visible.
    let start = if total_processes > visible_rows {
        let max_start = total_processes - visible_rows;
        let preferred_start = selected.saturating_sub(visible_rows.saturating_sub(1));
        preferred_start.min(max_start)
    } else {
        0
    };

    let rows: Vec<Row> = processes
        .iter()
        .enumerate()
        .skip(start)
        .take(visible_rows)
        .map(|(idx, p)| {
            let style = if idx == selected {
                Style::default()
                    .fg(Theme::CRUST)
                    .bg(Theme::PINK)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::TEXT)
            };
            
            Row::new(vec![
                Cell::from(format!("{}", p.pid)),
                Cell::from(p.cmd.clone()),  // Show full command instead of just name
                Cell::from(format!("{:.1}%", p.cpu_usage)),
                Cell::from(format!("{:.1} MB", p.memory() as f64 / 1024.0 / 1024.0)),
            ])
            .style(style)
        })
        .collect();
    
    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Length(8),
            ratatui::layout::Constraint::Min(30),
            ratatui::layout::Constraint::Length(10),
            ratatui::layout::Constraint::Length(12),
        ]
    )
    .header(Row::new(vec!["PID", "Command", sort_indicator, "Memory"])
        .style(Style::default()
            .fg(Theme::LAVENDER)
            .add_modifier(Modifier::BOLD))
        .bottom_margin(1))
    .block(Block::default()
        .title(vec![
            Span::styled("╭─ ", Style::default().fg(Theme::PINK)),
            Span::styled("PROCESSES", Style::default()
                .fg(Theme::PINK)
                .add_modifier(Modifier::BOLD)),
            Span::styled(" ─╮", Style::default().fg(Theme::PINK)),
        ])
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::PINK))
        .style(Style::default().bg(Theme::MANTLE)));
    
    f.render_widget(table, chunks[1]);

    let mut footer_idx = 2;
    if show_filter_bar {
        draw_filter_bar(f, chunks[2], app);
        footer_idx = 3;
    }

    draw_footer(f, chunks[footer_idx], app);
}

fn draw_filter_bar(f: &mut Frame, area: Rect, app: &App) {
    let prompt = if app.filter_mode {
        "[/ search] Type to filter • Enter: apply & select first • Esc: clear"
    } else {
        "[/ search]"
    };

    let matches = app.current_processes().len();
    let cursor = if app.filter_mode { " █" } else { "" };

    let content = vec![
        Line::from(vec![
            Span::styled(prompt, Style::default().fg(Theme::SUBTEXT1)),
            Span::raw("  "),
            Span::styled(
                format!("Query: {}{}", app.filter_input, cursor),
                Style::default()
                    .fg(Theme::ROSEWATER)
                    .bg(Theme::CRUST)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("({} match{})", matches, if matches == 1 { "" } else { "es" }),
                Style::default().fg(Theme::SUBTEXT1),
            ),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Filter")
                .border_style(Style::default().fg(Theme::PINK))
                .style(Style::default().bg(Theme::MANTLE)),
        )
        .style(Style::default().fg(Theme::TEXT));

    // Clear the area to avoid artifacts from previous frames.
    f.render_widget(ratatui::widgets::Clear, area);
    f.render_widget(paragraph, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let status_text = if app.paused { 
        Span::styled(" ⏸ PAUSED ", Style::default()
            .fg(Theme::CRUST)
            .bg(Theme::YELLOW)
            .add_modifier(Modifier::BOLD))
    } else { 
        Span::styled(" ▶ RUNNING ", Style::default()
            .fg(Theme::CRUST)
            .bg(Theme::GREEN)
            .add_modifier(Modifier::BOLD))
    };
    
    let view_text = match app.view_mode {
        ViewMode::Dashboard => "Processes",
        ViewMode::Processes => "Dashboard",
        ViewMode::History => "Dashboard",
    };
    
    // Context-aware footer based on current view
    let footer_text = match app.view_mode {
        ViewMode::Dashboard => vec![
            Line::from(vec![
                Span::raw("  "),
                Span::styled("[q]", Style::default()
                    .fg(Theme::MAUVE)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" Quit", Style::default().fg(Theme::TEXT)),
                Span::raw("  │  "),
                Span::styled("[Tab]", Style::default()
                    .fg(Theme::PINK)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(format!(" {} ", view_text), Style::default().fg(Theme::TEXT)),
                Span::raw("│  "),
                Span::styled("[h]", Style::default()
                    .fg(Theme::LAVENDER)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" History", Style::default().fg(Theme::TEXT)),
                Span::raw("  │  "),
                Span::styled("[p]", Style::default()
                    .fg(Theme::BLUE)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" Pause", Style::default().fg(Theme::TEXT)),
                Span::raw("  │  "),
                status_text,
            ]),
        ],
        ViewMode::Processes => vec![
            Line::from(vec![
                Span::raw("  "),
                Span::styled("[q]", Style::default()
                    .fg(Theme::MAUVE)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" Quit", Style::default().fg(Theme::TEXT)),
                Span::raw("  │  "),
                Span::styled("[Tab]", Style::default()
                    .fg(Theme::PINK)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" Dashboard", Style::default().fg(Theme::TEXT)),
                Span::raw("  │  "),
                Span::styled("[s]", Style::default()
                    .fg(Theme::TEAL)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" Sort", Style::default().fg(Theme::TEXT)),
                Span::raw("  │  "),
                Span::styled("[/]", Style::default()
                    .fg(Theme::PINK)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" Search", Style::default().fg(Theme::TEXT)),
                Span::raw("  │  "),
                Span::styled("[↑↓]", Style::default()
                    .fg(Theme::LAVENDER)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" Select", Style::default().fg(Theme::TEXT)),
                Span::raw("  │  "),
                Span::styled("[k]", Style::default()
                    .fg(Theme::RED)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" Kill", Style::default().fg(Theme::TEXT)),
                Span::raw("  │  "),
                Span::styled("[t]", Style::default()
                    .fg(Theme::YELLOW)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" Term", Style::default().fg(Theme::TEXT)),
                Span::raw("  │  "),
                Span::styled("[i]", Style::default()
                    .fg(Theme::BLUE)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" Info", Style::default().fg(Theme::TEXT)),
                Span::raw("  │  "),
                status_text,
            ]),
        ],
        ViewMode::History => vec![
            Line::from(vec![
                Span::raw("  "),
                Span::styled("[h]", Style::default()
                    .fg(Theme::PINK)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" Exit History", Style::default().fg(Theme::TEXT)),
                Span::raw("  │  "),
                Span::styled("[← →]", Style::default()
                    .fg(Theme::LAVENDER)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(" Navigate", Style::default().fg(Theme::TEXT)),
                Span::raw("  │  "),
                Span::styled("⏸ PLAYBACK", Style::default()
                    .fg(Theme::CRUST)
                    .bg(Theme::YELLOW)
                    .add_modifier(Modifier::BOLD)),
            ]),
        ],
    };
    
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::OVERLAY0))
            .style(Style::default().bg(Theme::MANTLE)));
    
    f.render_widget(footer, area);
}

fn draw_confirm_dialog(f: &mut Frame, title: &str, message: &str, app: &App) {
    super::dialogs::draw_confirm_dialog(f, title, message, app);
}

fn draw_info_dialog(f: &mut Frame, app: &App) {
    super::dialogs::draw_info_dialog(f, app);
}

fn draw_history_view(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(5),  // Timeline
            Constraint::Min(10),    // Historical data
            Constraint::Length(3),  // Footer
        ])
        .split(f.area());

    // Header
    let header = Paragraph::new(vec![Line::from(vec![
        Span::styled("    ◆ ", Style::default().fg(Theme::PINK)),
        Span::styled("TIME-TRAVEL MODE", Style::default()
            .fg(Theme::PINK)
            .add_modifier(Modifier::BOLD)),
        Span::styled(" ◆  Rewind System State", Style::default().fg(Theme::SUBTEXT1)),
    ])])
    .alignment(Alignment::Center)
    .block(Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::PINK))
        .style(Style::default().bg(Theme::MANTLE)));
    f.render_widget(header, chunks[0]);

    // Timeline
    draw_timeline(f, chunks[1], app);

    // Historical metrics
    draw_historical_metrics(f, chunks[2], app);

    // Footer
    draw_history_footer(f, chunks[3], app);
}

fn draw_timeline(f: &mut Frame, area: Rect, app: &App) {
    let history_len = app.history.cpu_usage.len();
    let current_idx = app.playback_index.unwrap_or(0);
    
    let progress = if history_len > 0 {
        (current_idx as f64 / (history_len - 1).max(1) as f64 * 100.0) as u16
    } else {
        0
    };

    let timestamp = if let Some(idx) = app.playback_index {
        if let Some((ts, _)) = app.history.cpu_usage.get_at(idx) {
            let datetime = chrono::DateTime::from_timestamp(ts as i64, 0);
            if let Some(dt) = datetime {
                dt.format("%H:%M:%S").to_string()
            } else {
                "Unknown".to_string()
            }
        } else {
            "N/A".to_string()
        }
    } else {
        "LIVE".to_string()
    };

    let text = vec![
        Line::from(vec![
            Span::styled("◀ ", Style::default().fg(Theme::LAVENDER)),
            Span::styled(&timestamp, Style::default()
                .fg(Theme::YELLOW)
                .add_modifier(Modifier::BOLD)),
            Span::styled(" ▶", Style::default().fg(Theme::LAVENDER)),
            Span::styled(format!("  [{}/{}]", current_idx + 1, history_len), 
                Style::default().fg(Theme::SUBTEXT1)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("[{:█<50}]", "█".repeat((progress / 2) as usize)),
            Style::default().fg(Theme::PINK)
        )),
    ];

    let timeline = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(Block::default()
            .title("Timeline [← →]")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::LAVENDER))
            .style(Style::default().bg(Theme::MANTLE)));
    
    f.render_widget(timeline, area);
}

fn draw_historical_metrics(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // CPU at this timestamp
    if let Some(idx) = app.playback_index {
        if let Some((_, cpu_val)) = app.history.cpu_usage.get_at(idx) {
            let cpu_color = Theme::cpu_color(cpu_val);
            let label = format!("{:.1}%", cpu_val);
            
            let gauge = Gauge::default()
                .block(Block::default()
                    .title("CPU (Historical)")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Theme::BLUE))
                    .style(Style::default().bg(Theme::MANTLE)))
                .gauge_style(Style::default().fg(cpu_color).bg(Theme::SURFACE0))
                .label(Span::styled(label, Style::default()
                    .fg(Theme::TEXT)
                    .add_modifier(Modifier::BOLD)))
                .percent(cpu_val.min(100.0) as u16);
            
            f.render_widget(gauge, chunks[0]);
        }

        if let Some((_, mem_val)) = app.history.memory_usage.get_at(idx) {
            let mem_color = Theme::memory_color(mem_val);
            let label = format!("{:.1}%", mem_val);
            
            let gauge = Gauge::default()
                .block(Block::default()
                    .title("Memory (Historical)")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Theme::TEAL))
                    .style(Style::default().bg(Theme::MANTLE)))
                .gauge_style(Style::default().fg(mem_color).bg(Theme::SURFACE0))
                .label(Span::styled(label, Style::default()
                    .fg(Theme::TEXT)
                    .add_modifier(Modifier::BOLD)))
                .percent(mem_val.min(100.0) as u16);
            
            f.render_widget(gauge, chunks[1]);
        }
    }
}

fn draw_history_footer(f: &mut Frame, area: Rect, _app: &App) {
    let footer_text = vec![
        Line::from(vec![
            Span::raw("  "),
            Span::styled("[h]", Style::default()
                .fg(Theme::PINK)
                .add_modifier(Modifier::BOLD)),
            Span::styled(" Exit History", Style::default().fg(Theme::TEXT)),
            Span::raw("  │  "),
            Span::styled("[← →]", Style::default()
                .fg(Theme::LAVENDER)
                .add_modifier(Modifier::BOLD)),
            Span::styled(" Navigate Timeline", Style::default().fg(Theme::TEXT)),
            Span::raw("  │  "),
            Span::styled("⏸ PLAYBACK", Style::default()
                .fg(Theme::CRUST)
                .bg(Theme::YELLOW)
                .add_modifier(Modifier::BOLD)),
        ]),
    ];
    
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Theme::OVERLAY0))
            .style(Style::default().bg(Theme::MANTLE)));
    
    f.render_widget(footer, area);
}
