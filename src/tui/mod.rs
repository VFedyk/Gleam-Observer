pub mod events;
pub mod theme;
pub mod ui;
pub mod widgets;
pub mod dialogs;

use crate::app::App;
use crate::error::Result;
use crossterm::{
    event::DisableMouseCapture,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

pub fn run(mut app: App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    let tick_rate = Duration::from_millis(app.config.refresh.interval_ms);
    let event_handler = events::EventHandler::new(tick_rate);
    
    let result = run_app(&mut terminal, &mut app, &event_handler);
    
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    result
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    event_handler: &events::EventHandler,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;
        
        match event_handler.next()? {
            events::AppEvent::Key(key) => {
                use crate::app::DialogMode;
                
                // Handle filter input mode first
                if app.filter_mode {
                    match key.code {
                        crossterm::event::KeyCode::Esc => {
                            app.exit_filter_mode();
                        }
                        crossterm::event::KeyCode::Enter => {
                            // Exit filter mode and focus first result
                            app.filter_mode = false;
                            app.selected_process_index = 0;
                            app.clamp_process_selection();
                        }
                        crossterm::event::KeyCode::Backspace => {
                            app.filter_backspace();
                        }
                        crossterm::event::KeyCode::Char(c) => {
                            app.filter_input_char(c);
                        }
                        _ => {}
                    }
                    continue;
                }
                
                // Handle dialog input next
                match app.dialog_mode {
                    DialogMode::ConfirmKill | DialogMode::ConfirmTerminate => {
                        if events::is_yes(&key) {
                            if app.dialog_mode == DialogMode::ConfirmKill {
                                let _ = app.kill_selected_process();
                            } else {
                                let _ = app.terminate_selected_process();
                            }
                        } else if events::is_no(&key) || events::is_escape(&key) {
                            app.close_dialog();
                        }
                        continue;
                    }
                    DialogMode::ProcessInfo => {
                        if events::is_escape(&key) || events::is_enter(&key) {
                            app.close_dialog();
                        }
                        continue;
                    }
                    DialogMode::None => {}
                }
                
                // Regular key handling
                if events::should_quit(&key) {
                    app.quit();
                    break;
                } else if events::should_pause(&key) {
                    app.toggle_pause();
                } else if events::should_toggle_view(&key) {
                    app.toggle_view();
                } else if events::should_enter_history(&key) {
                    if app.view_mode == crate::app::ViewMode::History {
                        app.exit_history_mode();
                    } else {
                        app.enter_history_mode();
                    }
                } else if events::is_arrow_left(&key) {
                    if app.view_mode == crate::app::ViewMode::History {
                        app.playback_step_backward();
                    } else if app.tree_mode {
                        app.toggle_collapse_selected();
                    }
                } else if events::is_arrow_right(&key) {
                    if app.view_mode == crate::app::ViewMode::History {
                        app.playback_step_forward();
                    } else if app.tree_mode {
                        app.toggle_collapse_selected();
                    }
                } else if events::should_toggle_tree(&key) {
                    app.toggle_tree_mode();
                } else if events::should_enter_filter(&key) {
                    app.enter_filter_mode();
                } else if events::should_cycle_sort(&key) {
                    app.cycle_sort();
                } else if events::is_arrow_up(&key) {
                    app.move_selection_up();
                } else if events::is_arrow_down(&key) {
                    app.move_selection_down();
                } else if events::should_show_kill_dialog(&key) {
                    app.show_kill_dialog();
                } else if events::should_show_terminate_dialog(&key) {
                    app.show_terminate_dialog();
                } else if events::should_show_info(&key) {
                    app.show_info_dialog();
                }
            }
            events::AppEvent::Tick => {
                app.update()?;
            }
            events::AppEvent::Resize(_, _) => {}
            events::AppEvent::Ignored => {
                // Mouse events - redraw but don't update metrics
            }
        }
        
        if !app.running {
            break;
        }
    }
    
    Ok(())
}
