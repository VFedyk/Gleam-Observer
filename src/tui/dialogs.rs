use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Clear, Paragraph},
    Frame,
};
use crate::app::App;
use super::theme::CatppuccinTheme as Theme;

pub fn draw_confirm_dialog(f: &mut Frame, title: &str, message: &str, app: &App) {
    let area = centered_rect(60, 30, f.area());
    
    let pid = app.get_selected_pid().unwrap_or(0);
    
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(message, Style::default()
            .fg(Theme::TEXT)
            .add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled(format!("PID: {}", pid), Style::default().fg(Theme::SUBTEXT1))),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Y]", Style::default()
                .fg(Theme::GREEN)
                .add_modifier(Modifier::BOLD)),
            Span::styled(" Yes  ", Style::default().fg(Theme::TEXT)),
            Span::styled("[N]", Style::default()
                .fg(Theme::RED)
                .add_modifier(Modifier::BOLD)),
            Span::styled(" No", Style::default().fg(Theme::TEXT)),
        ]),
    ];
    
    let block = Block::default()
        .title(vec![
            Span::styled("⚠ ", Style::default().fg(Theme::YELLOW)),
            Span::styled(title, Style::default()
                .fg(Theme::YELLOW)
                .add_modifier(Modifier::BOLD)),
            Span::styled(" ⚠", Style::default().fg(Theme::YELLOW)),
        ])
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::YELLOW))
        .style(Style::default().bg(Theme::CRUST));
    
    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);
    
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

pub fn draw_info_dialog(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 50, f.area());
    
    let processes = app.current_processes();
    let selected = processes.get(app.selected_process_index);
    
    let text = if let Some(proc) = selected {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Name: ", Style::default()
                    .fg(Theme::SUBTEXT1)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(&proc.name, Style::default().fg(Theme::TEXT)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("PID: ", Style::default()
                    .fg(Theme::SUBTEXT1)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(format!("{}", proc.pid), Style::default().fg(Theme::TEXT)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("CPU: ", Style::default()
                    .fg(Theme::SUBTEXT1)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(format!("{:.2}%", proc.cpu_usage), Style::default().fg(Theme::BLUE)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Memory: ", Style::default()
                    .fg(Theme::SUBTEXT1)
                    .add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{:.2} MB", proc.memory() as f64 / 1024.0 / 1024.0),
                    Style::default().fg(Theme::TEAL)
                ),
            ]),
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled("[ESC] Close", Style::default()
                .fg(Theme::SUBTEXT1)
                .add_modifier(Modifier::ITALIC))),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled("No process selected", Style::default().fg(Theme::RED))),
        ]
    };
    
    let block = Block::default()
        .title(vec![
            Span::styled("ℹ ", Style::default().fg(Theme::BLUE)),
            Span::styled("Process Info", Style::default()
                .fg(Theme::BLUE)
                .add_modifier(Modifier::BOLD)),
        ])
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Theme::BLUE))
        .style(Style::default().bg(Theme::CRUST));
    
    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);
    
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
