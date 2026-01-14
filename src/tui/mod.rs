// TUI module for rendering the terminal interface
pub mod colors;
pub mod helpers;
pub mod input;

// Re-exports
pub use colors::*;
pub use helpers::{calculate_progress, format_file_size};
pub use input::{handle_confirm_input, handle_key_event, KeyAction};

use crate::async_preview::{PreviewState, SyncPreviewManager};
use crate::domain::{AppState, DecisionStatistics};
use crate::preview;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, Paragraph, Wrap},
    Frame,
};

/// UI view state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewState {
    /// Main file browsing view
    Browsing,
    /// Help overlay visible
    Help,
    /// Summary screen at end
    Summary,
    /// Confirmation dialog for trash action
    ConfirmTrash,
    /// Welcome screen shown on first launch
    Welcome,
}

/// Renders the TUI (legacy, without async preview)
pub fn render(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header with progress
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    render_header_polished(frame, chunks[0], state);
    render_content(frame, chunks[1], state);
    render_footer_polished(frame, chunks[2]);
}

/// Renders the TUI with async preview support
pub fn render_with_preview(
    frame: &mut Frame,
    state: &AppState,
    preview_manager: &mut SyncPreviewManager,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header with progress
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    render_header_polished(frame, chunks[0], state);
    render_content_async(frame, chunks[1], state, preview_manager);
    render_footer_polished(frame, chunks[2]);
}

/// Renders the summary screen at the end
pub fn render_summary(frame: &mut Frame, stats: &DecisionStatistics) {
    let area = frame.area();

    // Center the summary box
    let summary_area = centered_rect(60, 50, area);

    // Clear the background
    frame.render_widget(Clear, summary_area);

    let block = Block::default()
        .title(" Session Complete ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT_HIGHLIGHT))
        .style(Style::default().bg(BG_DARK));

    let inner = block.inner(summary_area);
    frame.render_widget(block, summary_area);

    // Build summary content
    let total = stats.total_files;
    let kept = stats.kept;
    let trashed = stats.trashed;
    let remaining = total.saturating_sub(kept + trashed);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Summary",
            Style::default()
                .fg(TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("   Total files:  "),
            Span::styled(
                format!("{}", total),
                Style::default()
                    .fg(ACCENT_HIGHLIGHT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("   ✓ ", Style::default().fg(ACCENT_SECONDARY)),
            Span::raw("Kept:     "),
            Span::styled(
                format!("{}", kept),
                Style::default()
                    .fg(ACCENT_SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("   ✗ ", Style::default().fg(ACCENT_PRIMARY)),
            Span::raw("Trashed:  "),
            Span::styled(
                format!("{}", trashed),
                Style::default()
                    .fg(ACCENT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("   ○ ", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("Skipped:  "),
            Span::styled(
                format!("{}", remaining),
                Style::default().fg(TEXT_SECONDARY),
            ),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to exit",
            Style::default().fg(TEXT_SECONDARY),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .style(Style::default().fg(TEXT_PRIMARY));

    frame.render_widget(paragraph, inner);
}

/// Renders the help overlay
pub fn render_help_overlay(frame: &mut Frame) {
    let area = frame.area();
    let help_area = centered_rect(50, 70, area);

    // Clear background
    frame.render_widget(Clear, help_area);

    let block = Block::default()
        .title(" Help ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT_HIGHLIGHT))
        .style(Style::default().bg(BG_DARK));

    let inner = block.inner(help_area);
    frame.render_widget(block, help_area);

    let help_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default()
                .fg(ACCENT_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  → ", Style::default().fg(ACCENT_SECONDARY)),
            Span::raw("or "),
            Span::styled("k", Style::default().fg(ACCENT_SECONDARY)),
            Span::raw("     Keep file"),
        ]),
        Line::from(vec![
            Span::styled("  ← ", Style::default().fg(ACCENT_PRIMARY)),
            Span::raw("or "),
            Span::styled("t", Style::default().fg(ACCENT_PRIMARY)),
            Span::raw("     Trash file"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ↑↓ ", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("or "),
            Span::styled("i/j", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("   Navigate"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  u ", Style::default().fg(ACCENT_HIGHLIGHT)),
            Span::raw("or "),
            Span::styled("Ctrl+Z", Style::default().fg(ACCENT_HIGHLIGHT)),
            Span::raw("  Undo"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  o ", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("          Open file in editor"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  q ", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("or "),
            Span::styled("Esc", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("     Quit"),
        ]),
        Line::from(vec![
            Span::styled("  ?", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("           Toggle help"),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "Press ? or Esc to close",
            Style::default().fg(TEXT_SECONDARY),
        )),
    ];

    let paragraph = Paragraph::new(help_lines)
        .alignment(Alignment::Center)
        .style(Style::default().fg(TEXT_PRIMARY));

    frame.render_widget(paragraph, inner);
}

/// Renders the welcome dialog overlay
pub fn render_welcome_overlay(frame: &mut Frame) {
    let area = centered_rect(85, 85, frame.area());

    // Clear background
    frame.render_widget(Clear, area);

    // Create welcome content
    let welcome_lines = vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Welcome to fswp!",
            Style::default()
                .fg(ACCENT_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::raw(
            "fswp helps you quickly review and organize files with a Tinder-like interface.",
        )]),
        Line::from(vec![Span::raw(
            "Swipe through files, decide what to keep or trash, and declutter your directories.",
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Quick Start:",
            Style::default()
                .fg(ACCENT_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  1. ", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("Review the current file (preview shown below)"),
        ]),
        Line::from(vec![
            Span::styled("  2. ", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("Press "),
            Span::styled(
                "→ (Right Arrow)",
                Style::default()
                    .fg(ACCENT_SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to "),
            Span::styled("keep", Style::default().fg(ACCENT_SECONDARY)),
            Span::raw(" or "),
            Span::styled(
                "← (Left Arrow)",
                Style::default()
                    .fg(ACCENT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to "),
            Span::styled("trash", Style::default().fg(ACCENT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("  3. ", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("Continue until you've reviewed all files"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Essential Keyboard Shortcuts:",
            Style::default()
                .fg(ACCENT_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  → / k  ", Style::default().fg(ACCENT_SECONDARY)),
            Span::raw("      Keep file"),
        ]),
        Line::from(vec![
            Span::styled("  ← / t  ", Style::default().fg(ACCENT_PRIMARY)),
            Span::raw("      Trash file"),
        ]),
        Line::from(vec![
            Span::styled("  ↑ / i  ", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("      Previous file"),
        ]),
        Line::from(vec![
            Span::styled("  ↓ / j  ", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("      Next file"),
        ]),
        Line::from(vec![
            Span::styled("  u      ", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("      Undo last decision"),
        ]),
        Line::from(vec![
            Span::styled("  o      ", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("      Open file in editor"),
        ]),
        Line::from(vec![
            Span::styled("  ?      ", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("      Show help (access this anytime)"),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc", Style::default().fg(TEXT_SECONDARY)),
            Span::raw("      Quit application"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Safety Features:",
            Style::default()
                .fg(ACCENT_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  •  ", Style::default().fg(ACCENT_SECONDARY)),
            Span::raw("Files are moved to your system "),
            Span::styled("trash", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" (not permanently deleted)"),
        ]),
        Line::from(vec![
            Span::styled("  •  ", Style::default().fg(ACCENT_SECONDARY)),
            Span::raw("Use "),
            Span::styled("'u'", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to undo any decision before quitting"),
        ]),
        Line::from(vec![
            Span::styled("  •  ", Style::default().fg(ACCENT_SECONDARY)),
            Span::raw("Run with "),
            Span::styled("--dry-run", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to preview without making changes"),
        ]),
        Line::from(vec![
            Span::styled("  •  ", Style::default().fg(ACCENT_SECONDARY)),
            Span::raw("Use "),
            Span::styled("--yes", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" to skip confirmation dialogs"),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press any key to start browsing...",
            Style::default()
                .fg(ACCENT_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    let welcome_text = Paragraph::new(welcome_lines)
        .block(
            Block::default()
                .title(Span::styled(
                    " Welcome to fswp ",
                    Style::default()
                        .fg(ACCENT_HIGHLIGHT)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(ACCENT_HIGHLIGHT))
                .style(Style::default().bg(BG_DARK)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });

    frame.render_widget(welcome_text, area);
}

/// Renders the confirmation dialog for trash action
pub fn render_confirm_trash_overlay(frame: &mut Frame, file: &crate::domain::FileEntry) {
    let area = frame.area();
    let confirm_area = centered_rect(50, 60, area);

    // Clear background
    frame.render_widget(Clear, confirm_area);

    let block = Block::default()
        .title(" ⚠ Confirm Trash ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT_PRIMARY))
        .style(Style::default().bg(BG_DARK));

    let inner = block.inner(confirm_area);
    frame.render_widget(block, confirm_area);

    let confirm_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Are you sure you want to trash this file?",
            Style::default()
                .fg(TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  File: ", Style::default().fg(TEXT_SECONDARY)),
            Span::styled(&file.name, Style::default().fg(TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("  Size: ", Style::default().fg(TEXT_SECONDARY)),
            Span::styled(
                format_file_size(file.size),
                Style::default().fg(TEXT_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Type: ", Style::default().fg(TEXT_SECONDARY)),
            Span::styled(
                format!("{:?}", file.file_type),
                Style::default().fg(TEXT_PRIMARY),
            ),
        ]),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "This file will be moved to trash.",
            Style::default().fg(TEXT_SECONDARY),
        )),
        Line::from(Span::styled(
            "You can undo this action with 'u'.",
            Style::default().fg(TEXT_SECONDARY),
        )),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Y]", Style::default().fg(ACCENT_SECONDARY)),
            Span::raw("es  "),
            Span::styled("[Enter]", Style::default().fg(ACCENT_SECONDARY)),
            Span::raw("     "),
            Span::styled("[N]", Style::default().fg(ACCENT_PRIMARY)),
            Span::raw("o  "),
            Span::styled("[Esc]", Style::default().fg(ACCENT_PRIMARY)),
        ]),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(confirm_lines)
        .alignment(Alignment::Center)
        .style(Style::default().fg(TEXT_PRIMARY));

    frame.render_widget(paragraph, inner);
}

/// Renders a loading overlay
pub fn render_loading_overlay(frame: &mut Frame, file: &crate::domain::FileEntry) {
    let area = frame.area();
    let loading_area = centered_rect(40, 30, area);

    // Clear background
    frame.render_widget(Clear, loading_area);

    let block = Block::default()
        .title(" Loading Preview ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT_HIGHLIGHT))
        .style(Style::default().bg(BG_DARK));

    let inner = block.inner(loading_area);
    frame.render_widget(block, loading_area);

    // Simple animation based on current time
    let spinners = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let spinner_idx = (now / 100) as usize % spinners.len();
    let spinner = spinners[spinner_idx];

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {} ", spinner),
                Style::default().fg(ACCENT_HIGHLIGHT),
            ),
            Span::styled("Processing file", Style::default().fg(TEXT_PRIMARY)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Name: ", Style::default().fg(TEXT_SECONDARY)),
            Span::styled(
                &file.name,
                Style::default()
                    .fg(TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Size: ", Style::default().fg(TEXT_SECONDARY)),
            Span::styled(
                format_file_size(file.size),
                Style::default().fg(TEXT_PRIMARY),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Type: ", Style::default().fg(TEXT_SECONDARY)),
            Span::styled(
                format!("{:?}", file.file_type),
                Style::default().fg(TEXT_PRIMARY),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Please wait...",
            Style::default()
                .fg(TEXT_SECONDARY)
                .add_modifier(Modifier::ITALIC),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .style(Style::default().fg(TEXT_PRIMARY));

    frame.render_widget(paragraph, inner);
}

/// Helper to create a centered rect
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

/// Renders the polished header with progress bar
fn render_header_polished(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Length(2)])
        .split(area);

    // Title and file info
    let (title_text, file_info) = if let Some(file) = state.current_file() {
        let size_str = format_file_size(file.size);
        let file_type = format!("{:?}", file.file_type);
        (
            format!(" File {}/{} ", state.current_index + 1, state.files.len()),
            vec![
                Span::styled(
                    &file.name,
                    Style::default()
                        .fg(TEXT_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("({} • {})", size_str, file_type),
                    Style::default().fg(TEXT_SECONDARY),
                ),
            ],
        )
    } else {
        (
            " Fswp ".to_string(),
            vec![Span::styled(
                "No files to review",
                Style::default().fg(TEXT_SECONDARY),
            )],
        )
    };

    let title_line = Line::from(vec![Span::styled(
        title_text,
        Style::default()
            .fg(ACCENT_HIGHLIGHT)
            .add_modifier(Modifier::BOLD),
    )]);

    let info_line = Line::from(file_info);

    let header = Paragraph::new(vec![title_line, info_line])
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER_COLOR)),
        )
        .alignment(Alignment::Left);

    frame.render_widget(header, chunks[0]);

    // Progress bar
    let total = state.files.len();
    let processed = state.decisions_stack.len();
    let progress = if total > 0 {
        processed as f64 / total as f64
    } else {
        0.0
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER_COLOR)),
        )
        .gauge_style(Style::default().fg(ACCENT_SECONDARY).bg(BG_DARK))
        .ratio(progress)
        .label(format!(
            "{}% ({}/{})",
            (progress * 100.0) as u16,
            processed,
            total
        ));

    frame.render_widget(gauge, chunks[1]);
}

/// Renders the main content area (synchronous version)
fn render_content(frame: &mut Frame, area: Rect, state: &AppState) {
    use crate::preview::PreviewContent;

    let content = if let Some(file) = state.current_file() {
        // Generate file preview
        let lines: Vec<Line> = match preview::generate_preview(file) {
            Ok(PreviewContent::Text(text_lines)) => {
                text_lines.into_iter().map(Line::from).collect()
            }
            Ok(PreviewContent::Styled(styled_lines)) => styled_lines,
            Err(e) => vec![
                Line::from(format!("Error generating preview: {}", e)),
                Line::from(""),
                Line::from(format!("File: {}", file.name)),
                Line::from(format!("Path: {}", file.path.display())),
                Line::from(format!("Size: {} bytes", file.size)),
                Line::from(format!("Type: {:?}", file.file_type)),
            ],
        };

        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(BORDER_COLOR))
                    .title(format!(" {} ", file.name)),
            )
            .style(Style::default().fg(TEXT_PRIMARY))
            .wrap(Wrap { trim: false })
    } else {
        render_empty_state_widget()
    };

    frame.render_widget(content, area);
}

/// Creates an empty state widget for when no files are present
fn render_empty_state_widget() -> Paragraph<'static> {
    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "No Files Found",
            Style::default()
                .fg(ACCENT_HIGHLIGHT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "The directory is empty or contains only hidden files.",
            Style::default().fg(TEXT_SECONDARY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Try a different directory with visible files.",
            Style::default().fg(TEXT_SECONDARY),
        )),
    ];

    Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER_COLOR))
                .title(" Content "),
        )
        .alignment(Alignment::Center)
}

/// Renders the main content area with async preview loading
fn render_content_async(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    preview_manager: &mut SyncPreviewManager,
) {
    use crate::preview::PreviewContent;

    if let Some(file) = state.current_file() {
        // Get preview state from manager
        let preview_state = preview_manager.request_preview(file);

        match preview_state {
            PreviewState::Loading => {
                // Render an empty block for content first
                let content_block = Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(BORDER_COLOR))
                    .title(format!(" {} ", file.name));
                frame.render_widget(content_block, area);

                // Then render the loading overlay
                render_loading_overlay(frame, file);
            }
            PreviewState::Ready(preview_content) => {
                let lines = match preview_content {
                    PreviewContent::Text(text_lines) => {
                        text_lines.iter().map(|s| Line::from(s.clone())).collect()
                    }
                    PreviewContent::Styled(styled_lines) => styled_lines.clone(),
                };

                let paragraph = Paragraph::new(lines)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(BORDER_COLOR))
                            .title(format!(" {} ", file.name)),
                    )
                    .style(Style::default().fg(TEXT_PRIMARY))
                    .wrap(Wrap { trim: false });
                frame.render_widget(paragraph, area);
            }
            PreviewState::Error(e) => {
                let error_lines: Vec<Line> = vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  [!] Error generating preview",
                        Style::default()
                            .fg(ACCENT_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(format!("  {}", e)),
                    Line::from(""),
                    Line::from(format!("  File: {}", file.name)),
                    Line::from(format!("  Path: {}", file.path.display())),
                    Line::from(format!("  Size: {}", format_file_size(file.size))),
                    Line::from(format!("  Type: {:?}", file.file_type)),
                ];

                let paragraph = Paragraph::new(error_lines)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Rounded)
                            .border_style(Style::default().fg(ACCENT_PRIMARY))
                            .title(format!(" {} [!] ", file.name)),
                    )
                    .style(Style::default().fg(TEXT_PRIMARY))
                    .wrap(Wrap { trim: false });
                frame.render_widget(paragraph, area);
            }
        }
    } else {
        frame.render_widget(render_empty_state_widget(), area);
    }
}

/// Renders the polished footer with styled controls
fn render_footer_polished(frame: &mut Frame, area: Rect) {
    let controls = Line::from(vec![
        Span::styled(
            " ← ",
            Style::default()
                .fg(ACCENT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Trash", Style::default().fg(TEXT_SECONDARY)),
        Span::raw("  │  "),
        Span::styled(
            "→ ",
            Style::default()
                .fg(ACCENT_SECONDARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Keep", Style::default().fg(TEXT_SECONDARY)),
        Span::raw("  │  "),
        Span::styled("↑↓ ", Style::default().fg(TEXT_SECONDARY)),
        Span::styled("Navigate", Style::default().fg(TEXT_SECONDARY)),
        Span::raw("  │  "),
        Span::styled("u ", Style::default().fg(ACCENT_HIGHLIGHT)),
        Span::styled("Undo", Style::default().fg(TEXT_SECONDARY)),
        Span::raw("  │  "),
        Span::styled("? ", Style::default().fg(TEXT_SECONDARY)),
        Span::styled("Help", Style::default().fg(TEXT_SECONDARY)),
        Span::raw("  │  "),
        Span::styled("q ", Style::default().fg(TEXT_SECONDARY)),
        Span::styled("Quit", Style::default().fg(TEXT_SECONDARY)),
    ]);

    let footer = Paragraph::new(controls)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER_COLOR)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(footer, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FileEntry, FileType};
    use chrono::Utc;
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

            // Check for empty state message in content
            let buffer_str: String = content.iter().map(|c| c.symbol()).collect();
            // The new empty state shows "No Files Found" or contains "empty"
            assert!(
                buffer_str.contains("No Files") || buffer_str.contains("empty"),
                "Expected empty state message, got: {}",
                buffer_str
            );
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

        #[test]
        fn test_render_help_overlay() {
            let backend = TestBackend::new(80, 30);
            let mut terminal = Terminal::new(backend).unwrap();

            terminal
                .draw(|frame| {
                    render_help_overlay(frame);
                })
                .unwrap();

            let buffer = terminal.backend().buffer().clone();
            let content = buffer.content();
            let buffer_str: String = content.iter().map(|c| c.symbol()).collect();

            // Check for help content
            assert!(buffer_str.contains("Help"));
            assert!(buffer_str.contains("Keep"));
            assert!(buffer_str.contains("Trash"));
        }

        #[test]
        fn test_render_summary() {
            let stats = DecisionStatistics {
                total_files: 10,
                kept: 6,
                trashed: 3,
            };

            let backend = TestBackend::new(80, 30);
            let mut terminal = Terminal::new(backend).unwrap();

            terminal
                .draw(|frame| {
                    render_summary(frame, &stats);
                })
                .unwrap();

            let buffer = terminal.backend().buffer().clone();
            let content = buffer.content();
            let buffer_str: String = content.iter().map(|c| c.symbol()).collect();

            // Check for summary content
            assert!(buffer_str.contains("Summary") || buffer_str.contains("Complete"));
        }

        #[test]
        fn test_render_loading_overlay() {
            let file = create_test_entry("test_image.png");
            let backend = TestBackend::new(80, 30);
            let mut terminal = Terminal::new(backend).unwrap();

            terminal
                .draw(|frame| {
                    render_loading_overlay(frame, &file);
                })
                .unwrap();

            let buffer = terminal.backend().buffer().clone();
            let content = buffer.content();
            let buffer_str: String = content.iter().map(|c| c.symbol()).collect();

            // Check for loading content
            assert!(buffer_str.contains("Loading"));
            assert!(buffer_str.contains("test_image.png"));
            assert!(buffer_str.contains("Processing"));
        }
    }
}
