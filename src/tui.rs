// TUI module for rendering the terminal interface
#![allow(dead_code)]

use crate::domain::AppState;
use crate::preview;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Represents the result of handling a key event
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    /// Quit the application
    Quit,
    /// Mark current file to keep
    Keep,
    /// Mark current file to trash
    Trash,
    /// Move to next file
    Next,
    /// Move to previous file
    Previous,
    /// Undo last decision
    Undo,
    /// No action
    None,
}

/// Maps keyboard events to actions
pub fn handle_key_event(key: KeyEvent) -> KeyAction {
    match (key.code, key.modifiers) {
        // Quit: q or Ctrl+C
        (KeyCode::Char('q'), KeyModifiers::NONE) => KeyAction::Quit,
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => KeyAction::Quit,
        (KeyCode::Esc, KeyModifiers::NONE) => KeyAction::Quit,

        // Keep: Right arrow or k
        (KeyCode::Right, KeyModifiers::NONE) => KeyAction::Keep,
        (KeyCode::Char('k'), KeyModifiers::NONE) => KeyAction::Keep,

        // Trash: Left arrow or t
        (KeyCode::Left, KeyModifiers::NONE) => KeyAction::Trash,
        (KeyCode::Char('t'), KeyModifiers::NONE) => KeyAction::Trash,

        // Navigation
        (KeyCode::Down, KeyModifiers::NONE) => KeyAction::Next,
        (KeyCode::Up, KeyModifiers::NONE) => KeyAction::Previous,
        (KeyCode::Char('j'), KeyModifiers::NONE) => KeyAction::Next,
        (KeyCode::Char('i'), KeyModifiers::NONE) => KeyAction::Previous,

        // Undo: u or Ctrl+Z
        (KeyCode::Char('u'), KeyModifiers::NONE) => KeyAction::Undo,
        (KeyCode::Char('z'), KeyModifiers::CONTROL) => KeyAction::Undo,

        _ => KeyAction::None,
    }
}

/// Renders the TUI
pub fn render(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    render_header(frame, chunks[0], state);
    render_content(frame, chunks[1], state);
    render_footer(frame, chunks[2]);
}

/// Renders the header with file info
fn render_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let header_text = if let Some(file) = state.current_file() {
        format!(
            "File {}/{}: {}",
            state.current_index + 1,
            state.files.len(),
            file.name
        )
    } else {
        "No files to review".to_string()
    };

    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).title("File Tinder"))
        .style(Style::default().fg(Color::Cyan));

    frame.render_widget(header, area);
}

/// Renders the main content area
fn render_content(frame: &mut Frame, area: Rect, state: &AppState) {
    let content = if let Some(file) = state.current_file() {
        // Generate file preview
        let preview_lines = match preview::generate_preview(file) {
            Ok(lines) => lines,
            Err(e) => vec![
                format!("Error generating preview: {}", e),
                String::new(),
                format!("File: {}", file.name),
                format!("Path: {}", file.path.display()),
                format!("Size: {} bytes", file.size),
                format!("Type: {:?}", file.file_type),
            ],
        };

        // Convert strings to Lines
        let lines: Vec<Line> = preview_lines.into_iter().map(Line::from).collect();

        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Preview: {}", file.name)),
            )
            .wrap(Wrap { trim: false })
    } else {
        Paragraph::new("No files to display")
            .block(Block::default().borders(Borders::ALL).title("Content"))
    };

    frame.render_widget(content, area);
}

/// Renders the footer with controls
fn render_footer(frame: &mut Frame, area: Rect) {
    let footer_text = "←/t: Trash | →/k: Keep | ↑↓/i/j: Navigate | u: Undo | q: Quit";
    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));

    frame.render_widget(footer, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FileEntry, FileType};
    use chrono::Utc;
    use crossterm::event::KeyModifiers;
    use ratatui::{backend::TestBackend, Terminal};
    use std::path::PathBuf;

    fn create_test_entry(name: &str) -> FileEntry {
        FileEntry {
            path: PathBuf::from(name),
            name: name.to_string(),
            size: 1024,
            modified_date: Utc::now(),
            file_type: FileType::Text,
        }
    }

    mod key_handling_tests {
        use super::*;

        #[test]
        fn test_key_quit() {
            let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
            assert_eq!(handle_key_event(key), KeyAction::Quit);

            let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
            assert_eq!(handle_key_event(key), KeyAction::Quit);

            let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
            assert_eq!(handle_key_event(key), KeyAction::Quit);
        }

        #[test]
        fn test_key_keep() {
            let key = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
            assert_eq!(handle_key_event(key), KeyAction::Keep);

            let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
            assert_eq!(handle_key_event(key), KeyAction::Keep);
        }

        #[test]
        fn test_key_trash() {
            let key = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
            assert_eq!(handle_key_event(key), KeyAction::Trash);

            let key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
            assert_eq!(handle_key_event(key), KeyAction::Trash);
        }

        #[test]
        fn test_key_navigation() {
            let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
            assert_eq!(handle_key_event(key), KeyAction::Next);

            let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
            assert_eq!(handle_key_event(key), KeyAction::Previous);

            let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
            assert_eq!(handle_key_event(key), KeyAction::Next);

            let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
            assert_eq!(handle_key_event(key), KeyAction::Previous);
        }

        #[test]
        fn test_key_undo() {
            let key = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE);
            assert_eq!(handle_key_event(key), KeyAction::Undo);

            let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL);
            assert_eq!(handle_key_event(key), KeyAction::Undo);
        }

        #[test]
        fn test_key_none() {
            let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
            assert_eq!(handle_key_event(key), KeyAction::None);

            let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
            assert_eq!(handle_key_event(key), KeyAction::None);
        }
    }

    mod layout_tests {
        use super::*;

        #[test]
        fn test_render_empty_state() {
            let state = AppState::new(vec![]);
            let backend = TestBackend::new(80, 24);
            let mut terminal = Terminal::new(backend).unwrap();

            terminal
                .draw(|frame| {
                    render(frame, &state);
                })
                .unwrap();

            // Verify no panics and rendering succeeds
            let buffer = terminal.backend().buffer().clone();
            let content = buffer.content();

            // Check for "No files" message in content
            let buffer_str: String = content.iter().map(|c| c.symbol()).collect();
            assert!(buffer_str.contains("No files"));
        }

        #[test]
        fn test_render_with_files() {
            let files = vec![
                create_test_entry("file1.txt"),
                create_test_entry("file2.rs"),
            ];
            let state = AppState::new(files);
            let backend = TestBackend::new(80, 24);
            let mut terminal = Terminal::new(backend).unwrap();

            terminal
                .draw(|frame| {
                    render(frame, &state);
                })
                .unwrap();

            let buffer = terminal.backend().buffer().clone();
            let content = buffer.content();
            let buffer_str: String = content.iter().map(|c| c.symbol()).collect();

            // Check for file info in buffer
            assert!(buffer_str.contains("file1.txt"));
            assert!(buffer_str.contains("File 1/2"));
        }

        #[test]
        fn test_render_footer() {
            let state = AppState::new(vec![create_test_entry("test.txt")]);
            let backend = TestBackend::new(80, 24);
            let mut terminal = Terminal::new(backend).unwrap();

            terminal
                .draw(|frame| {
                    render(frame, &state);
                })
                .unwrap();

            let buffer = terminal.backend().buffer().clone();
            let content = buffer.content();
            let buffer_str: String = content.iter().map(|c| c.symbol()).collect();

            // Check for controls in footer
            assert!(buffer_str.contains("Trash"));
            assert!(buffer_str.contains("Keep"));
            assert!(buffer_str.contains("Quit"));
        }

        #[test]
        fn test_render_header_progress() {
            let files = vec![
                create_test_entry("file1.txt"),
                create_test_entry("file2.txt"),
                create_test_entry("file3.txt"),
            ];
            let mut state = AppState::new(files);
            state.next();

            let backend = TestBackend::new(80, 24);
            let mut terminal = Terminal::new(backend).unwrap();

            terminal
                .draw(|frame| {
                    render(frame, &state);
                })
                .unwrap();

            let buffer = terminal.backend().buffer().clone();
            let content = buffer.content();
            let buffer_str: String = content.iter().map(|c| c.symbol()).collect();

            // Check that we're on file 2 of 3
            assert!(buffer_str.contains("File 2/3"));
            assert!(buffer_str.contains("file2.txt"));
        }
    }
}
