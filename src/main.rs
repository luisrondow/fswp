mod domain;
mod tui;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use domain::{discover_files, AppState, Decision, DecisionEngine};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, path::Path, time::Duration};
use tui::{handle_key_event, render, KeyAction};

fn main() -> io::Result<()> {
    println!("File Tinder - Terminal File Declutterer");
    Ok(())
}

/// Runs the TUI application
pub fn run_app(directory: &Path) -> io::Result<()> {
    // Discover files
    let files = discover_files(directory)?;
    if files.is_empty() {
        println!("No files found in directory");
        return Ok(());
    }

    // Initialize state
    let mut app_state = AppState::new(files.clone());
    let mut decision_engine = DecisionEngine::new(files);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let result = run_loop(&mut terminal, &mut app_state, &mut decision_engine);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

/// Main application loop
fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app_state: &mut AppState,
    decision_engine: &mut DecisionEngine,
) -> io::Result<()> {
    loop {
        // Render
        terminal.draw(|frame| render(frame, app_state))?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let action = handle_key_event(key);

                match action {
                    KeyAction::Quit => {
                        break;
                    }
                    KeyAction::Keep => {
                        decision_engine.record_decision(app_state.current_index, Decision::Keep)?;
                        app_state.record_decision(Decision::Keep);
                        app_state.next();
                    }
                    KeyAction::Trash => {
                        decision_engine
                            .record_decision(app_state.current_index, Decision::Trash)?;
                        app_state.record_decision(Decision::Trash);
                        app_state.next();
                    }
                    KeyAction::Next => {
                        app_state.next();
                    }
                    KeyAction::Previous => {
                        app_state.previous();
                    }
                    KeyAction::Undo => {
                        if decision_engine.undo().is_ok() {
                            app_state.undo();
                        }
                    }
                    KeyAction::None => {}
                }
            }
        }
    }

    Ok(())
}
