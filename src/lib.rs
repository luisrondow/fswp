//! Fswp - A terminal-based file decluttering library
//!
//! This crate provides the core functionality for the Fswp application,
//! enabling programmatic file review and organization workflows.

pub mod async_preview;
pub mod cli;
pub mod config;
pub mod domain;
pub mod error;
pub mod file_opener;
pub mod preview;
pub mod tui;

// Re-export primary types for convenience
pub use config::UserConfig;
pub use domain::{
    discover_files, discover_files_with_options, AppState, Decision, DecisionEngine,
    DecisionStatistics, DiscoveryOptions, FileEntry, FileType, SortBy,
};
pub use error::{FileTinderError, Result};
pub use file_opener::open_file;
