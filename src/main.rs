use fswp::async_preview::SyncPreviewManager;
use fswp::cli::{AppConfig, Args, SortOrder};
use fswp::domain::{
    discover_files_with_options, AppState, Decision, DecisionEngine, DiscoveryOptions, SortBy,
};
use fswp::tui::{
    handle_key_event, render_help_overlay, render_summary, render_with_preview, KeyAction,
    ViewState,
};

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

fn main() -> io::Result<()> {
    // Parse command line arguments
    let args = Args::parse_args();

    // Validate arguments
    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // Convert to config
    let config: AppConfig = args.into();

    // Run the app
    run_app_with_config(&config)
}

/// Runs the TUI application with configuration
pub fn run_app_with_config(config: &AppConfig) -> io::Result<()> {
    // Convert config to discovery options
    let discovery_options = DiscoveryOptions {
        file_types: config.file_type_filters.clone(),
        show_hidden: config.show_hidden,
        min_size: config.min_size,
        max_size: config.max_size,
        sort_by: match config.sort_by {
            SortOrder::Date => SortBy::Date,
            SortOrder::Name => SortBy::Name,
            SortOrder::Size => SortBy::Size,
            SortOrder::Type => SortBy::Type,
        },
        reverse: config.reverse,
    };

    // Discover files with options
    let files = discover_files_with_options(&config.directory, &discovery_options)?;

    if files.is_empty() {
        println!(
            "No files found in directory: {}",
            config.directory.display()
        );
        if config.file_type_filters.is_some() {
            println!("(File type filters are active - try without filters)");
        }
        return Ok(());
    }

    // Print dry-run notice
    if config.dry_run {
        println!("[DRY RUN] No files will be moved to trash");
        println!("   Found {} files to review", files.len());
        println!("   Press Enter to continue...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
    }

    // Initialize state
    let mut app_state = AppState::new(files.clone());
    let mut decision_engine = DecisionEngine::new(files);
    decision_engine.set_dry_run(config.dry_run);
    let mut preview_manager = SyncPreviewManager::new();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let result = run_loop(
        &mut terminal,
        &mut app_state,
        &mut decision_engine,
        &mut preview_manager,
    );

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Print summary after exit
    if config.dry_run {
        let stats = decision_engine.get_statistics();
        println!("\n[DRY RUN] Complete");
        println!("   Would have kept: {} files", stats.kept);
        println!("   Would have trashed: {} files", stats.trashed);
    }

    result
}

/// Main application loop
fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app_state: &mut AppState,
    decision_engine: &mut DecisionEngine,
    preview_manager: &mut SyncPreviewManager,
) -> io::Result<()> {
    let mut view_state = ViewState::Browsing;

    loop {
        // Render based on current view state
        terminal.draw(|frame| {
            render_with_preview(frame, app_state, preview_manager);

            // Render overlays
            match view_state {
                ViewState::Help => render_help_overlay(frame),
                ViewState::Summary => {
                    let stats = decision_engine.get_statistics();
                    render_summary(frame, &stats);
                }
                ViewState::Browsing => {}
            }
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Handle overlay-specific input
                match view_state {
                    ViewState::Help => {
                        // Any key closes help (or toggle with ?)
                        let action = handle_key_event(key);
                        if matches!(action, KeyAction::Help | KeyAction::Quit | KeyAction::None) {
                            view_state = ViewState::Browsing;
                        }
                        continue;
                    }
                    ViewState::Summary => {
                        // Any key exits from summary
                        break;
                    }
                    ViewState::Browsing => {}
                }

                let action = handle_key_event(key);

                match action {
                    KeyAction::Quit => {
                        // Show summary before quitting if any decisions were made
                        let stats = decision_engine.get_statistics();
                        if stats.kept > 0 || stats.trashed > 0 {
                            view_state = ViewState::Summary;
                        } else {
                            break;
                        }
                    }
                    KeyAction::Keep => {
                        if decision_engine
                            .record_decision(app_state.current_index, Decision::Keep)
                            .is_ok()
                        {
                            app_state.record_decision(Decision::Keep);
                            app_state.next();
                            preview_manager.reset();

                            // Check if we've processed all files
                            if is_all_files_processed(app_state, decision_engine) {
                                view_state = ViewState::Summary;
                            }
                        }
                    }
                    KeyAction::Trash => {
                        if decision_engine
                            .record_decision(app_state.current_index, Decision::Trash)
                            .is_ok()
                        {
                            app_state.record_decision(Decision::Trash);
                            app_state.next();
                            preview_manager.reset();

                            // Check if we've processed all files
                            if is_all_files_processed(app_state, decision_engine) {
                                view_state = ViewState::Summary;
                            }
                        }
                    }
                    KeyAction::Next => {
                        app_state.next();
                        preview_manager.reset();
                    }
                    KeyAction::Previous => {
                        app_state.previous();
                        preview_manager.reset();
                    }
                    KeyAction::Undo => {
                        if decision_engine.undo().is_ok() {
                            app_state.undo();
                            preview_manager.reset();
                            // Return to browsing if we were in summary
                            if view_state == ViewState::Summary {
                                view_state = ViewState::Browsing;
                            }
                        }
                    }
                    KeyAction::Help => {
                        view_state = ViewState::Help;
                    }
                    KeyAction::None => {}
                }
            }
        }
    }

    Ok(())
}

/// Checks if all files have been processed
fn is_all_files_processed(app_state: &AppState, decision_engine: &DecisionEngine) -> bool {
    let stats = decision_engine.get_statistics();
    stats.kept + stats.trashed >= app_state.files.len()
}
