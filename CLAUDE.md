# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

fswp is a terminal-based file decluttering application built in Rust. It presents a swipe-style interface where users can quickly review files one-by-one and decide to keep or trash them using keyboard shortcuts. The project is feature-complete with all core functionality implemented and a modular architecture.

## Development Commands

### Testing
```bash
# Run all tests
cargo test

# Run tests in watch mode (requires cargo-watch)
cargo watch -x test

# Run tests for a specific module
cargo test domain
cargo test cli
cargo test tui
cargo test preview
cargo test async_preview
cargo test config
cargo test file_opener
```

### Building
```bash
# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run the application
cargo run

# Run with CLI arguments
cargo run -- --help
cargo run -- ~/Downloads --type image --sort size --reverse
cargo run -- . --dry-run --min-size 1MB
cargo run -- ~/Downloads -y  # Skip confirmation prompts
```

### Code Quality
```bash
# Format code
cargo fmt

# Check formatting without modifying files
cargo fmt --check

# Run Clippy linter
cargo clippy

# Run Clippy with warnings as errors (CI mode)
cargo clippy -- -D warnings
```

### CI Pipeline
GitHub Actions automatically runs on every push/PR:
- Format checking (`cargo fmt --check`)
- Linting (`cargo clippy -- -D warnings`)
- Tests (`cargo test`)

## Architecture

### Module Structure

The codebase follows a modular architecture with clear separation of concerns:

```
src/
├── lib.rs              # Library entry point with public API
├── main.rs             # Binary entry point and main event loop
├── error.rs            # Custom error types (thiserror)
├── cli.rs              # CLI argument parsing and configuration
├── config.rs           # User configuration and preferences
├── file_opener.rs      # File opening in external editors
├── preview.rs          # File preview generation
├── async_preview.rs    # Async preview loading with caching
├── domain/
│   ├── mod.rs          # Module exports and re-exports
│   ├── file_type.rs    # FileType enum
│   ├── file_entry.rs   # FileEntry struct
│   ├── decision.rs     # Decision enum + DecisionStatistics
│   ├── app_state.rs    # AppState struct
│   ├── discovery.rs    # File discovery + SortBy + DiscoveryOptions
│   └── decision_engine.rs  # DecisionEngine
└── tui/
    ├── mod.rs          # ViewState + main render functions
    ├── colors.rs       # Color theme constants
    ├── helpers.rs      # format_file_size, calculate_progress
    └── input.rs        # KeyAction + handle_key_event
```

### Library API (`src/lib.rs`)

The project exposes a library API for programmatic use:

```rust
pub use domain::{
    discover_files, discover_files_with_options, AppState, Decision,
    DecisionEngine, DecisionStatistics, DiscoveryOptions, FileEntry,
    FileType, SortBy,
};
pub use error::{FileTinderError, Result};
```

### Error Handling (`src/error.rs`)

Custom error types using `thiserror`:

- `FileTinderError::Io` — Wrapped I/O errors
- `FileTinderError::DirectoryNotFound` — Invalid directory path
- `FileTinderError::FileNotFound` — Missing file
- `FileTinderError::InvalidIndex` — Out-of-bounds file index
- `FileTinderError::NothingToUndo` — Empty undo stack
- `FileTinderError::PreviewError` — Preview generation failure
- `FileTinderError::TrashError` — Trash operation failure
- `FileTinderError::ConfigError` — Configuration issues
- `FileTinderError::OpenFileError` — File opening failure

### Core Domain Model (`src/domain/`)

The domain module is split into focused submodules:

**`file_type.rs`**: `FileType` enum categorizing files into Text, Image, Pdf, or Binary. The `from_extension()` method handles case-insensitive extension matching.

**`file_entry.rs`**: `FileEntry` struct representing a single file with metadata (path, name, size, modification date, file type). Created via `from_path()` which extracts metadata from the filesystem.

**`decision.rs`**: `Decision` enum (Keep/Trash) and `DecisionStatistics` struct for session summaries.

**`app_state.rs`**: `AppState` struct for central state management:
- `files`: Vec of all FileEntry objects
- `current_index`: Tracks which file is currently being viewed
- `decisions_stack`: Stack of (index, Decision) tuples for undo functionality
- Methods: `next()`, `previous()`, `current_file()`, `record_decision()`, `undo()`

**`decision_engine.rs`**: `DecisionEngine` struct managing file decisions and trash operations:
- `dry_run`: Disables actual file moves for preview mode
- `staging_dir`: Temporary staging before final trash
- Methods: `record_decision()`, `undo()`, `get_statistics()`, `commit_trash_decisions()`

**`discovery.rs`**: File discovery functions and configuration:
- `SortBy` enum: Date, Name, Size, Type
- `DiscoveryOptions` struct with filters (file_types, show_hidden, min_size, max_size, sort_by, reverse)
- `discover_files(dir_path)` — Default file discovery
- `discover_files_with_options()` — Advanced discovery with filtering/sorting

### CLI Module (`src/cli.rs`)

**Args struct**: Clap-derived argument parser with:
- `directory`: Target directory (default: ".")
- `file_types`: Type filters (--type text,image,pdf,binary)
- `dry_run`: Preview mode without file moves
- `sort_by`: Sort criteria (date, name, size, type)
- `reverse`: Reverse sort order
- `show_hidden`: Include hidden files
- `min_size/max_size`: Size filters (supports "5MB", "1GB" format)
- `yes`: Skip confirmation prompts for trash actions
- `welcome`: Force show welcome dialog on startup

**AppConfig struct**: Validated configuration derived from Args with `skip_confirm` and `show_welcome` fields.

### Config Module (`src/config.rs`)

User configuration persistence:

**UserConfig struct**: Serializable config with:
- `welcome_shown`: Tracks if welcome dialog has been displayed

**Methods**:
- `config_path()` — Returns `~/.config/fswp/config.json`
- `load()` — Load config or return default
- `save()` — Persist config to disk

### File Opener Module (`src/file_opener.rs`)

Opens files in external applications:

**`open_file(path)`**: Opens file with precedence:
1. `$EDITOR` or `$VISUAL` environment variable (via `edit` crate)
2. System default application (via `open` crate)

Blocks until the editor/application closes.

### TUI Module (`src/tui/`)

The TUI module is split into focused submodules:

**`colors.rs`**: Color theme constants — coral red (`ACCENT_PRIMARY` for trash), mint green (`ACCENT_SECONDARY` for keep), golden yellow (`ACCENT_HIGHLIGHT`), and neutral tones for text/borders.

**`helpers.rs`**: Utility functions:
- `format_file_size()` — Human-readable file sizes
- `calculate_progress()` — Progress bar calculations

**`input.rs`**: Input handling:
- `KeyAction` enum: Quit, Keep, Trash, ConfirmTrash, CancelTrash, Next, Previous, Undo, Help, Open, None
- `handle_key_event()` — Converts crossterm events to KeyActions (browsing mode)
- `handle_confirm_input()` — Converts events to KeyActions (confirmation dialog)

**`mod.rs`**: Main rendering logic:
- `ViewState` enum: Browsing, Help, Summary, ConfirmTrash, Welcome
- `render_with_preview()` — Main UI with async preview
- `render_summary()` — Session summary screen
- `render_help_overlay()` — Help modal
- `render_confirm_trash_overlay()` — Confirmation dialog
- `render_welcome_overlay()` — First-launch welcome screen

**Keyboard Bindings**:
- `→` / `k` — Keep
- `←` / `t` — Trash
- `↑` / `i` — Previous
- `↓` / `j` — Next
- `o` — Open file in editor
- `u` / `Ctrl+Z` — Undo
- `?` — Help
- `q` / `Esc` / `Ctrl+C` — Quit

**Confirmation Dialog Keys**:
- `y` / `Enter` — Confirm trash
- `n` / `Esc` — Cancel

### Preview Module (`src/preview.rs`)

Generates previews based on file type:

**Text/Code**: Syntax highlighting via `syntect`, showing first 50 lines.

**Images**: Half-block character rendering with true color support (max 80x40 chars).

**PDFs**: Text extraction from first page via `pdfium-render`.

**Key Functions**:
- `generate_preview()` — Dispatches to appropriate handler
- `generate_text_preview()` — Syntax-highlighted text
- `generate_image_preview()` — Image to half-block rendering
- `generate_pdf_preview()` — PDF text extraction

### Async Preview Module (`src/async_preview.rs`)

Non-blocking preview loading with caching:

**PreviewState enum**: Loading, Ready(Vec<String>), Error(String)

**PreviewLoader struct**: Async message-based loader with tokio background worker.

**SyncPreviewManager struct**: Synchronous wrapper for TUI integration:
- `request_preview()` — Start/get preview
- `poll_preview()` — Non-blocking check
- `reset()` — Clear for next file
- `cache_size()` — Cache statistics

**Architecture**: LRU cache (10 entries), 5-second timeout, cancellation support.

### Key Design Decisions

**Survivor Mode**: "Keep" action leaves files untouched; only "Trash" modifies filesystem.

**Safe Deletion**: Uses `trash` crate for OS-specific Trash/Recycle Bin instead of permanent deletion.

**Staging Directory**: DecisionEngine uses temp folder for staging before final trash (enables undo).

**Async Previews**: Background preview generation keeps UI responsive during heavy operations.

**Confirmation Dialogs**: Trash actions require confirmation by default (can be skipped with `-y` flag).

**Welcome Experience**: First-time users see a welcome dialog explaining the interface.

### Dependencies

**Core**:
- `ratatui` + `crossterm` — Terminal UI
- `tokio` — Async runtime
- `clap` — CLI argument parsing
- `trash` — Safe file deletion
- `chrono` — DateTime handling
- `thiserror` — Custom error types

**Preview**:
- `syntect` — Syntax highlighting
- `image` + `ratatui-image` — Image processing
- `pdfium-render` — PDF rendering

**File Operations**:
- `edit` — Editor integration ($EDITOR/$VISUAL)
- `open` — System default application

**Configuration**:
- `dirs` — Platform config directories
- `serde` + `serde_json` — Config serialization

**Testing**:
- `tempfile` — Temporary files
- `printpdf` — PDF creation for tests

## Development Notes

### TDD Workflow
This project follows Test-Driven Development methodology. Each feature:
1. Creates a feature branch
2. Writes tests first (Red)
3. Implements code to pass tests (Green)
4. Refactors (Refactor)
5. Creates PR for review

### Dead Code Warning
The `#![allow(dead_code)]` attribute in some modules is for incremental TDD development where types may be defined before full usage.

### Test Organization
Tests are organized in nested modules within each source file:
- `domain/file_type.rs`: file_type_tests
- `domain/file_entry.rs`: file_entry_tests
- `domain/app_state.rs`: app_state_tests
- `domain/discovery.rs`: discovery_tests
- `domain/decision_engine.rs`: decision_engine_tests
- `error.rs`: error display and conversion tests
- `cli.rs`: args_tests, config_tests
- `config.rs`: config tests
- `file_opener.rs`: file opener tests
- `tui/mod.rs`: layout_tests
- `tui/input.rs`: key_handling_tests
- `tui/helpers.rs`: formatting_tests
- `preview.rs`: syntax_tests, image_tests, pdf_tests
- `async_preview.rs`: cache_tests, async_loader_tests, sync_manager_tests

### Pdfium Library
PDF preview tests are skipped if the Pdfium library is unavailable. The `is_pdfium_available()` function checks for library presence.

### Adding New File Type Support
To add support for a new file type category:
1. Add variant to `FileType` enum in `domain/file_type.rs`
2. Update `from_extension()` match statement in `domain/file_type.rs`
3. Add test cases to `file_type_tests` module in `domain/file_type.rs`
4. Implement preview strategy in `preview.rs`
5. Add CLI filter option in `cli.rs`

## CLI Usage

```bash
fswp [OPTIONS] [DIRECTORY]

Arguments:
  [DIRECTORY]  Target directory [default: .]

Options:
  -t, --type <TYPE>       Filter by type (text, image, pdf, binary) [multiple allowed]
  -n, --dry-run           Preview without moving files
  -s, --sort <SORT>       Sort by (date, name, size, type) [default: date]
  -r, --reverse           Reverse sort order
      --hidden            Include hidden files
      --min-size <SIZE>   Minimum file size (e.g., "5MB", "1GB")
      --max-size <SIZE>   Maximum file size
  -y, --yes               Skip confirmation prompts for trash actions
      --welcome           Show welcome dialog on startup
  -h, --help              Print help
  -V, --version           Print version
```

### Examples
```bash
# Review files in Downloads, images only, largest first
fswp ~/Downloads --type image --sort size --reverse

# Dry run on current directory, files over 10MB
fswp . --dry-run --min-size 10MB

# Include hidden files, sort by name
fswp ~/Documents --hidden --sort name

# Skip confirmation prompts for faster workflow
fswp ~/Downloads -y
```
