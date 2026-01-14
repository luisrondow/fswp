use fswp::async_preview::SyncPreviewManager;
use fswp::cli::{AppConfig, Args, SortOrder};
use fswp::config::UserConfig;
use fswp::domain::{
    discover_files_with_options, AppState, Decision, DecisionEngine, DiscoveryOptions, SortBy,
};
use fswp::open_file;
use fswp::tui::{
    handle_confirm_input, handle_key_event, render_confirm_trash_overlay, render_help_overlay,
    render_summary, render_welcome_overlay, render_with_preview, KeyAction, ViewState,
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

    // Load user configuration
    let mut user_config = UserConfig::load().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load user config: {}", e);
        UserConfig::default()
    });

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
        config,
        &mut user_config,
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

/// Suspends the TUI terminal to allow external programs to run
fn suspend_terminal<B: ratatui::backend::Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Resumes the TUI terminal after external program exits
fn resume_terminal<B: ratatui::backend::Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
) -> io::Result<()> {
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.hide_cursor()?;
    terminal.clear()?;
    Ok(())
}

/// Main application loop
fn run_loop<B: ratatui::backend::Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
    app_state: &mut AppState,
    decision_engine: &mut DecisionEngine,
    preview_manager: &mut SyncPreviewManager,
    config: &AppConfig,
    user_config: &mut UserConfig,
) -> io::Result<()> {
    // Show welcome on first launch or if --welcome flag is set
    let should_show_welcome = config.show_welcome || !user_config.welcome_shown;
    let mut view_state = if should_show_welcome {
        ViewState::Welcome
    } else {
        ViewState::Browsing
    };

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
                ViewState::ConfirmTrash => {
                    if let Some(file) = app_state.current_file() {
                        render_confirm_trash_overlay(frame, file);
                    }
                }
                ViewState::Welcome => render_welcome_overlay(frame),
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
                    ViewState::ConfirmTrash => {
                        let action = handle_confirm_input(key);
                        match action {
                            KeyAction::ConfirmTrash => {
                                // Execute trash decision
                                if decision_engine
                                    .record_decision(app_state.current_index, Decision::Trash)
                                    .is_ok()
                                {
                                    app_state.record_decision(Decision::Trash);
                                    app_state.next();
                                    preview_manager.reset();

                                    if is_all_files_processed(app_state, decision_engine) {
                                        view_state = ViewState::Summary;
                                    } else {
                                        view_state = ViewState::Browsing;
                                    }
                                } else {
                                    view_state = ViewState::Browsing;
                                }
                            }
                            KeyAction::CancelTrash => {
                                view_state = ViewState::Browsing;
                            }
                            _ => {}
                        }
                        continue;
                    }
                    ViewState::Welcome => {
                        // Any key dismisses welcome and starts browsing
                        view_state = ViewState::Browsing;

                        // Mark welcome as shown and persist
                        user_config.welcome_shown = true;
                        if let Err(e) = user_config.save() {
                            eprintln!("Warning: Failed to save user config: {}", e);
                        }
                        continue;
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
                        // Skip confirmation if flag set or dry-run mode
                        if config.skip_confirm || decision_engine.is_dry_run() {
                            // Execute trash immediately
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
                        } else {
                            // Show confirmation dialog
                            view_state = ViewState::ConfirmTrash;
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
                    KeyAction::Open => {
                        if let Some(file) = app_state.current_file() {
                            // Suspend terminal before opening external program
                            if let Err(e) = suspend_terminal(terminal) {
                                eprintln!("Failed to suspend terminal: {}", e);
                                continue;
                            }

                            // Open the file (blocking call)
                            let open_result = open_file(&file.path);

                            // Resume terminal after external program exits
                            if let Err(e) = resume_terminal(terminal) {
                                eprintln!("Failed to resume terminal: {}", e);
                                return Err(e);
                            }

                            // Handle any errors from opening the file
                            if let Err(e) = open_result {
                                eprintln!("Failed to open file: {}", e);
                            }
                        }
                    }
                    KeyAction::ConfirmTrash | KeyAction::CancelTrash => {
                        // These actions are only handled in ConfirmTrash state
                        // Ignore them here
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
