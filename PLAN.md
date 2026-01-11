# File Tinder - TDD Implementation Plan

## Overview
A terminal-based file decluttering app built in Rust using TDD methodology. Each phase creates a feature branch, pushes to remote, and waits for user approval before proceeding.

## Phase 0: Repository & CI Setup
**Branch:** `main`

### Tasks
1. Initialize git repository: `git init`
2. Create GitHub repository: `gh repo create file-tinder --public --source=.`
3. Initialize Cargo project: `cargo init --name file-tinder`
4. Create `.github/workflows/ci.yml` with:
   - Run tests on push/PR
   - Clippy linting
   - Rustfmt format checking
5. Add `.gitignore` for Rust projects
6. Initial commit and push to main

### GitHub Actions CI Workflow
```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo test
```

---

## Phase 1: Core Domain Model
**Branch:** `feature/01-core-domain`

### Tasks
1. Add core dependencies to Cargo.toml:
   - `ratatui`
   - `crossterm`
   - `trash`
   - `tokio`
4. Create domain types with tests:
   - `FileEntry` struct (path, name, size, modified_date, file_type)
   - `FileType` enum (Text, Image, Pdf, Binary)
   - `Decision` enum (Keep, Trash)
   - `AppState` struct (files, current_index, decisions_stack)

### TDD Approach
- Write tests for `FileEntry` creation and metadata extraction
- Write tests for `FileType` detection from file extension/mime
- Write tests for `AppState` navigation (next, previous)

---

## Phase 2: File Discovery & Listing
**Branch:** `feature/02-file-discovery`

### Tasks
1. Implement directory scanner to list files
2. Filter out hidden files and directories
3. Sort files by modification date
4. Handle permission errors gracefully

### TDD Approach
- Write tests with temporary directories containing mock files
- Test filtering logic for hidden files
- Test sorting behavior
- Test error handling for inaccessible files

---

## Phase 3: Decision Engine & Trash Integration
**Branch:** `feature/03-decision-engine`

### Tasks
1. Implement `trash` crate integration
2. Create `DecisionEngine` to track keep/trash decisions
3. Implement undo functionality with decision stack
4. Handle trash operation errors

### TDD Approach
- Write tests for decision recording
- Write tests for undo stack operations
- Write integration tests with actual trash operations (using temp files)
- Test error scenarios (permission denied, file not found)

---

## Phase 4: Basic TUI Framework
**Branch:** `feature/04-basic-tui`

### Tasks
1. Set up `ratatui` with `crossterm` backend
2. Create basic layout (header, content, footer)
3. Implement keyboard event handling
4. Create main application loop

### TDD Approach
- Write tests for layout rendering (using ratatui's test backend)
- Write tests for key event mapping
- Write tests for state transitions based on input

---

## Phase 5: Text/Code Preview
**Branch:** `feature/05-text-preview`

### Tasks
1. Add `syntect` dependency for syntax highlighting
2. Implement text file reading (first ~50 lines)
3. Detect programming language from extension
4. Render highlighted code in TUI

### TDD Approach
- Write tests for language detection
- Write tests for line limiting logic
- Write tests for syntax highlighting output
- Test with various file encodings

---

## Phase 6: Image Preview
**Branch:** `feature/06-image-preview`

### Tasks
1. Add `ratatui-image` dependency
2. Implement image loading and resizing
3. Handle terminal protocol detection (Sixel, Kitty, fallback)
4. Integrate with main preview area

### TDD Approach
- Write tests for image loading from various formats (PNG, JPG, GIF)
- Write tests for resize calculations
- Test fallback behavior when protocols unavailable

---

## Phase 7: PDF Preview
**Branch:** `feature/07-pdf-preview`

### Tasks
1. Add `pdfium-render` with static binding
2. Implement first-page extraction and rendering
3. Convert PDF page to image for display
4. Handle corrupted/password-protected PDFs

### TDD Approach
- Write tests with sample PDF files
- Test page rendering to image buffer
- Test error handling for invalid PDFs
- Test memory management for large PDFs

---

## Phase 8: Async Preview Loading
**Branch:** `feature/08-async-preview`

### Tasks
1. Implement background preview loading with Tokio
2. Show loading indicator while preview renders
3. Cancel pending previews on rapid navigation
4. Cache recently viewed previews

### TDD Approach
- Write tests for async task spawning
- Test cancellation behavior
- Test cache hit/miss scenarios
- Test UI responsiveness during loading

---

## Phase 9: Polish & UX Enhancements
**Branch:** `feature/09-polish`

### Tasks
1. Add nice borders and color scheme
2. Implement progress indicator (X of Y files)
3. Add summary screen at end (kept: X, trashed: Y)
4. Add help overlay (?)
5. Handle edge cases (empty directories, single file)

### TDD Approach
- Write tests for progress calculations
- Write tests for summary statistics
- Test edge case handling
- Write integration tests for full user flows

---

## Phase 10: CLI Arguments & Configuration
**Branch:** `feature/10-cli-config`

### Tasks
1. Add `clap` for CLI argument parsing
2. Support target directory argument
3. Add flags for filtering by file type
4. Add dry-run mode option

### TDD Approach
- Write tests for argument parsing
- Test default values
- Test invalid argument handling

---

## Verification Strategy

### Unit Tests
Run after each phase: `cargo test`

### Integration Tests
- Create test fixtures with sample files
- Test complete workflows from directory scan to trash

### Manual Testing
- Test in different terminal emulators (iTerm2, Terminal.app, Kitty)
- Test with various file types and sizes
- Test keyboard responsiveness

### CI Pipeline
- GitHub Actions runs automatically on every push/PR
- Clippy linting catches common mistakes
- Rustfmt ensures consistent code style

---

## Dependencies Summary

```toml
[dependencies]
ratatui = "0.28"
crossterm = "0.28"
trash = "4.0"
tokio = { version = "1", features = ["full"] }
syntect = "5"
ratatui-image = "1"
pdfium-render = "0.8"
clap = { version = "4", features = ["derive"] }

[dev-dependencies]
tempfile = "3"
```

---

## Workflow Per Phase

1. Create feature branch: `git checkout -b feature/XX-name`
2. Write failing tests first (Red)
3. Implement minimum code to pass tests (Green)
4. Refactor while keeping tests green (Refactor)
5. Push to remote: `git push -u origin feature/XX-name`
6. Create the PR: `gh pr create --title "xxxx" --body "xxxxx"`
7. Wait for user confirmation
8. Merge to main and continue to next phase
