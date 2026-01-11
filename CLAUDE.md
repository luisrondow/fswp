# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

File Tinder is a terminal-based file decluttering application built in Rust. It presents a "Tinder-like" interface where users can quickly review files one-by-one and decide to keep or trash them using keyboard shortcuts.

## Development Commands

### Testing
```bash
# Run all tests
cargo test

# Run tests in watch mode (requires cargo-watch)
cargo watch -x test

# Run tests for a specific module
cargo test domain
```

### Building
```bash
# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run the application
cargo run
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

### TDD Workflow
This project follows Test-Driven Development methodology. The implementation is divided into phases (see PLAN.md), with each phase:
1. Creating a feature branch
2. Writing tests first (Red)
3. Implementing code to pass tests (Green)
4. Refactoring (Refactor)
5. Creating PR for review

### Core Domain Model (`src/domain.rs`)

The application is built around these core types:

**FileType enum**: Categorizes files into Text, Image, PDF, or Binary based on file extension. The `from_extension()` method handles case-insensitive extension matching for common file types.

**FileEntry struct**: Represents a single file with metadata including path, name, size, modification date, and file type. Created via `from_path()` which extracts metadata from the filesystem.

**Decision enum**: Represents user actions - Keep or Trash.

**AppState struct**: Central state management holding:
- `files`: Vec of all FileEntry objects
- `current_index`: Tracks which file is currently being viewed
- `decisions_stack`: Stack of (index, Decision) tuples for undo functionality

AppState provides navigation methods (`next()`, `previous()`), decision tracking (`record_decision()`), and undo capabilities (`undo()`).

### Planned Architecture (from SPEC.md)

**UI Layer**: Built with `ratatui` TUI framework using `crossterm` for terminal events. Single-card focus layout with header (file metadata), center (preview), and footer (controls).

**Preview System**: Different rendering strategies per file type:
- Text/Code: Syntax highlighting via `syntect`, showing ~50 lines
- Images: Terminal rendering with `ratatui-image` (supports Sixel/Kitty protocols with ASCII fallback)
- PDFs: First-page extraction via `pdfium-render` with static linking, rendered as image

**Async Processing**: Uses `tokio` runtime for non-blocking preview generation (especially for PDFs/images) to keep UI responsive.

**File Operations**: `trash` crate moves files to OS-specific Trash/Recycle Bin instead of permanent deletion.

### Key Design Decisions

**Survivor Mode**: "Keep" action leaves files untouched; only "Trash" modifies filesystem.

**Undo Stack**: Maintains history of decisions as (index, Decision) tuples to enable undo functionality.

**Safe Deletion**: Uses system trash instead of permanent deletion for safety.

**Static PDF Linking**: Bundles PDF engine to avoid external dependencies like poppler.

## Development Notes

### Current Phase
Phase 1 (Core Domain Model) is implemented. The domain types (FileType, FileEntry, Decision, AppState) are complete with comprehensive test coverage.

### Dead Code Warning
The `#![allow(dead_code)]` attribute in `src/domain.rs` is temporary since we're building incrementally with TDD. Types are defined before being used in later phases.

### Test Organization
Tests are organized in nested modules within `src/domain.rs`:
- `file_type_tests`: Extension detection and case handling
- `file_entry_tests`: File metadata extraction and error cases
- `app_state_tests`: Navigation, decision tracking, and undo logic

Tests use `tempfile` crate for creating temporary files during testing.

### Adding New File Type Support
To add support for a new file type category:
1. Add variant to `FileType` enum
2. Update `from_extension()` match statement with relevant extensions
3. Add test cases to `file_type_tests` module
4. Implement preview strategy in future preview system phase
