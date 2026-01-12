# Refactoring Plan: Modularity and Rust Best Practices

## Overview

Minimal-scope refactoring focused on:
- Splitting large files (domain.rs: 1,191 lines, tui.rs: 894 lines)
- Adding `thiserror` for custom error types
- Creating `lib.rs` for reusable library API
- Moving integration tests to `tests/` directory

## Target Structure

```
src/
├── lib.rs                    # Library entry point with public API
├── error.rs                  # Custom error types (thiserror)
├── main.rs                   # Binary entry point
├── cli.rs                    # CLI parsing (unchanged)
├── domain/
│   ├── mod.rs               # Module exports
│   ├── file_type.rs         # FileType enum
│   ├── file_entry.rs        # FileEntry struct
│   ├── decision.rs          # Decision enum + DecisionStatistics
│   ├── app_state.rs         # AppState struct
│   ├── discovery.rs         # File discovery functions + SortBy + DiscoveryOptions
│   └── decision_engine.rs   # DecisionEngine
├── tui/
│   ├── mod.rs               # Module exports + ViewState
│   ├── colors.rs            # Color theme constants
│   ├── input.rs             # KeyAction + handle_key_event
│   ├── helpers.rs           # format_file_size, calculate_progress
│   ├── render.rs            # Main render functions
│   ├── overlays.rs          # render_help_overlay, render_summary
│   └── widgets.rs           # render_header_polished, render_content, render_footer
├── preview.rs               # Preview generation (unchanged)
└── async_preview.rs         # Async preview loading (unchanged)

tests/
├── file_discovery_tests.rs  # Integration tests for file discovery
├── decision_engine_tests.rs # Integration tests for trash operations
├── preview_integration.rs   # Preview generation with real files
└── cli_validation_tests.rs  # CLI validation with filesystem
```

---

## Implementation Phases

### Phase 1: Add thiserror and Create Error Types

**Files to modify:**
- `Cargo.toml` - Add `thiserror = "1"`
- Create `src/error.rs`

**Error types to define:**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FileTinderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Directory not found: {path}")]
    DirectoryNotFound { path: std::path::PathBuf },

    #[error("File not found: {path}")]
    FileNotFound { path: std::path::PathBuf },

    #[error("Invalid file index: {index} (max: {max})")]
    InvalidIndex { index: usize, max: usize },

    #[error("No decisions to undo")]
    NothingToUndo,

    #[error("Preview generation failed: {reason}")]
    PreviewError { reason: String },

    #[error("Trash operation failed: {0}")]
    TrashError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, FileTinderError>;
```

---

### Phase 2: Create lib.rs with Public API

**Files to create:**
- `src/lib.rs`

**Files to modify:**
- `Cargo.toml` - Add lib/bin configuration
- `src/main.rs` - Update imports

**Cargo.toml additions:**
```toml
[lib]
name = "file_tinder"
path = "src/lib.rs"

[[bin]]
name = "file-tinder"
path = "src/main.rs"
```

**lib.rs structure:**
```rust
//! Fswp - A terminal-based file decluttering library

pub mod domain;
pub mod error;
pub mod preview;
pub mod async_preview;
pub mod tui;
pub mod cli;

// Re-export primary types
pub use domain::{
    AppState, Decision, DecisionEngine, DecisionStatistics,
    DiscoveryOptions, FileEntry, FileType, SortBy,
    discover_files, discover_files_with_options,
};
pub use error::{FileTinderError, Result};
```

---

### Phase 3: Split domain.rs into Module Directory

**Extraction order (by dependency):**

1. `src/domain/file_type.rs` - FileType enum, from_extension() (~35 lines)
2. `src/domain/decision.rs` - Decision enum, DecisionStatistics (~45 lines)
3. `src/domain/file_entry.rs` - FileEntry struct, from_path() (~77 lines)
4. `src/domain/app_state.rs` - AppState struct (~35 lines code)
5. `src/domain/discovery.rs` - DiscoveryOptions, SortBy, discover_files (~130 lines)
6. `src/domain/decision_engine.rs` - DecisionEngine (~160 lines)
7. `src/domain/mod.rs` - Re-exports all types

**For each extraction:**
1. Create new file with type + its unit tests
2. Add `pub mod` to mod.rs
3. Update imports in other files
4. Run `cargo test` to verify
5. Delete from original domain.rs

---

### Phase 4: Split tui.rs into Module Directory

**Extraction order:**

1. `src/tui/colors.rs` - Color constants (ACCENT_PRIMARY, etc.) (~20 lines)
2. `src/tui/helpers.rs` - format_file_size, calculate_progress (~50 lines)
3. `src/tui/input.rs` - KeyAction enum, handle_key_event (~90 lines)
4. `src/tui/overlays.rs` - render_help_overlay, render_summary, centered_rect (~150 lines)
5. `src/tui/widgets.rs` - render_header, render_content, render_footer (~200 lines)
6. `src/tui/render.rs` - render, render_with_preview (~100 lines)
7. `src/tui/mod.rs` - ViewState enum, re-exports

---

### Phase 5: Move Integration Tests to tests/

**Tests to move (identified by TempDir usage or real filesystem):**

From `domain.rs`:
- `file_discovery_tests` module → `tests/file_discovery_tests.rs`
- `decision_engine_tests` module → `tests/decision_engine_tests.rs`

From `preview.rs`:
- PDF integration tests (skip when pdfium unavailable) → `tests/preview_integration.rs`
- Image loading tests with real files → `tests/preview_integration.rs`

From `cli.rs`:
- Validation tests with path checking → `tests/cli_validation_tests.rs`

**Tests to keep inline (pure unit tests):**
- `file_type_tests` - No I/O, just enum logic
- `file_entry_tests` - Uses tempfile for isolation
- `app_state_tests` - Pure state logic
- Key handling tests in tui
- Formatting tests in tui

---

## Critical Files

| File | Action | Risk |
|------|--------|------|
| `src/domain.rs` | Split into 6 modules | Medium |
| `src/tui.rs` | Split into 7 modules | Medium |
| `Cargo.toml` | Add thiserror, lib config | Low |
| `src/main.rs` | Update imports | Low |
| `src/error.rs` | Create new | Low |
| `src/lib.rs` | Create new | Low |

---

## Verification Steps

After each phase:
1. `cargo build` - Ensure compilation
2. `cargo test` - All tests pass
3. `cargo clippy -- -D warnings` - No new warnings
4. `cargo fmt --check` - Formatting maintained
5. `cargo run -- --help` - CLI still works

Final verification:
```bash
cargo test
cargo build --release
cargo run -- ~/Downloads --dry-run
```

---

## Commits Strategy

Create atomic commits for each extraction:
1. "Add thiserror dependency and create error module"
2. "Create lib.rs with public API"
3. "Extract FileType to domain/file_type.rs"
4. "Extract Decision to domain/decision.rs"
5. "Extract FileEntry to domain/file_entry.rs"
6. "Extract AppState to domain/app_state.rs"
7. "Extract discovery functions to domain/discovery.rs"
8. "Extract DecisionEngine to domain/decision_engine.rs"
9. "Remove original domain.rs"
10. "Extract TUI colors to tui/colors.rs"
11. ... (continue for each tui file)
12. "Move integration tests to tests/ directory"
