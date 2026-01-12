//! Fswp - A terminal-based file decluttering library
//!
//! This crate provides the core functionality for the Fswp application,
//! enabling programmatic file review and organization workflows.

pub mod async_preview;
pub mod cli;
pub mod domain;
pub mod error;
pub mod preview;
pub mod tui;

// Re-export primary types for convenience
pub use domain::{
    discover_files, discover_files_with_options, AppState, Decision, DecisionEngine,
    DecisionStatistics, DiscoveryOptions, FileEntry, FileType, SortBy,
};
pub use error::{FileTinderError, Result};
