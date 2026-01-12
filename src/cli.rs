// CLI module for argument parsing and configuration
#![allow(dead_code)]

use crate::domain::FileType;
use clap::{ArgAction, Parser, ValueEnum};
use std::path::PathBuf;

/// Fswp - A terminal-based file decluttering tool
///
/// Swipe through your files Tinder-style: keep what you love, trash what you don't.
#[derive(Parser, Debug, Clone)]
#[command(name = "fswp")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Directory to scan for files
    ///
    /// If not specified, defaults to the current directory.
    #[arg(default_value = ".")]
    pub directory: PathBuf,

    /// Filter by file type(s)
    ///
    /// Can be specified multiple times to include multiple types.
    /// Example: --type text --type image
    #[arg(short = 't', long = "type", value_enum)]
    pub file_types: Vec<FileTypeFilter>,

    /// Dry run mode - preview actions without actually moving files to trash
    ///
    /// In dry run mode, no files will be moved or deleted.
    /// Useful for testing or seeing what would happen.
    #[arg(short = 'n', long = "dry-run", action = ArgAction::SetTrue)]
    pub dry_run: bool,

    /// Sort files by specified criteria
    #[arg(short = 's', long = "sort", value_enum, default_value = "date")]
    pub sort_by: SortOrder,

    /// Reverse sort order
    #[arg(short = 'r', long = "reverse", action = ArgAction::SetTrue)]
    pub reverse: bool,

    /// Show hidden files (files starting with .)
    #[arg(long = "hidden", action = ArgAction::SetTrue)]
    pub show_hidden: bool,

    /// Minimum file size filter (e.g., "1KB", "5MB", "1GB")
    #[arg(long = "min-size")]
    pub min_size: Option<String>,

    /// Maximum file size filter (e.g., "100MB", "1GB")
    #[arg(long = "max-size")]
    pub max_size: Option<String>,
}

/// File type filter options
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FileTypeFilter {
    /// Text files (txt, md, rs, py, js, etc.)
    Text,
    /// Image files (png, jpg, gif, etc.)
    Image,
    /// PDF files
    Pdf,
    /// Binary/other files
    Binary,
}

impl From<FileTypeFilter> for FileType {
    fn from(filter: FileTypeFilter) -> Self {
        match filter {
            FileTypeFilter::Text => FileType::Text,
            FileTypeFilter::Image => FileType::Image,
            FileTypeFilter::Pdf => FileType::Pdf,
            FileTypeFilter::Binary => FileType::Binary,
        }
    }
}

/// Sort order options
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum SortOrder {
    /// Sort by modification date (oldest first)
    #[default]
    Date,
    /// Sort by file name (alphabetical)
    Name,
    /// Sort by file size (smallest first)
    Size,
    /// Sort by file type
    Type,
}

impl Args {
    /// Parse command line arguments
    pub fn parse_args() -> Self {
        Args::parse()
    }

    /// Get the file type filters as domain FileType values
    pub fn get_file_type_filters(&self) -> Option<Vec<FileType>> {
        if self.file_types.is_empty() {
            None
        } else {
            Some(self.file_types.iter().map(|&t| t.into()).collect())
        }
    }

    /// Parse a size string (e.g., "5MB", "100KB") into bytes
    pub fn parse_size(size_str: &str) -> Option<u64> {
        let size_str = size_str.trim().to_uppercase();

        // Extract numeric part and suffix
        let (num_str, suffix) = if size_str.ends_with("GB") {
            (&size_str[..size_str.len() - 2], "GB")
        } else if size_str.ends_with("MB") {
            (&size_str[..size_str.len() - 2], "MB")
        } else if size_str.ends_with("KB") {
            (&size_str[..size_str.len() - 2], "KB")
        } else if size_str.ends_with("B") {
            (&size_str[..size_str.len() - 1], "B")
        } else {
            // Assume bytes if no suffix
            (size_str.as_str(), "B")
        };

        let num: f64 = num_str.trim().parse().ok()?;

        let multiplier: u64 = match suffix {
            "GB" => 1024 * 1024 * 1024,
            "MB" => 1024 * 1024,
            "KB" => 1024,
            _ => 1,
        };

        Some((num * multiplier as f64) as u64)
    }

    /// Get minimum size in bytes
    pub fn get_min_size(&self) -> Option<u64> {
        self.min_size.as_ref().and_then(|s| Self::parse_size(s))
    }

    /// Get maximum size in bytes
    pub fn get_max_size(&self) -> Option<u64> {
        self.max_size.as_ref().and_then(|s| Self::parse_size(s))
    }

    /// Validate the arguments and return any errors
    pub fn validate(&self) -> Result<(), String> {
        // Check if directory exists
        if !self.directory.exists() {
            return Err(format!(
                "Directory does not exist: {}",
                self.directory.display()
            ));
        }

        if !self.directory.is_dir() {
            return Err(format!(
                "Path is not a directory: {}",
                self.directory.display()
            ));
        }

        // Validate size strings if provided
        if let Some(ref min) = self.min_size {
            if Self::parse_size(min).is_none() {
                return Err(format!(
                    "Invalid min-size format: '{}'. Use format like '5MB', '100KB', '1GB'",
                    min
                ));
            }
        }

        if let Some(ref max) = self.max_size {
            if Self::parse_size(max).is_none() {
                return Err(format!(
                    "Invalid max-size format: '{}'. Use format like '5MB', '100KB', '1GB'",
                    max
                ));
            }
        }

        // Check min <= max if both specified
        if let (Some(min), Some(max)) = (self.get_min_size(), self.get_max_size()) {
            if min > max {
                return Err(format!(
                    "min-size ({}) cannot be greater than max-size ({})",
                    self.min_size.as_ref().unwrap(),
                    self.max_size.as_ref().unwrap()
                ));
            }
        }

        Ok(())
    }
}

/// Configuration derived from CLI arguments
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub directory: PathBuf,
    pub file_type_filters: Option<Vec<FileType>>,
    pub dry_run: bool,
    pub sort_by: SortOrder,
    pub reverse: bool,
    pub show_hidden: bool,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
}

impl From<Args> for AppConfig {
    fn from(args: Args) -> Self {
        AppConfig {
            directory: args.directory.clone(),
            file_type_filters: args.get_file_type_filters(),
            dry_run: args.dry_run,
            sort_by: args.sort_by,
            reverse: args.reverse,
            show_hidden: args.show_hidden,
            min_size: args.get_min_size(),
            max_size: args.get_max_size(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            directory: PathBuf::from("."),
            file_type_filters: None,
            dry_run: false,
            sort_by: SortOrder::Date,
            reverse: false,
            show_hidden: false,
            min_size: None,
            max_size: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod args_tests {
        use super::*;

        #[test]
        fn test_parse_size_bytes() {
            assert_eq!(Args::parse_size("100"), Some(100));
            assert_eq!(Args::parse_size("100B"), Some(100));
            assert_eq!(Args::parse_size("0"), Some(0));
        }

        #[test]
        fn test_parse_size_kilobytes() {
            assert_eq!(Args::parse_size("1KB"), Some(1024));
            assert_eq!(Args::parse_size("5KB"), Some(5 * 1024));
            assert_eq!(Args::parse_size("1.5KB"), Some(1536));
        }

        #[test]
        fn test_parse_size_megabytes() {
            assert_eq!(Args::parse_size("1MB"), Some(1024 * 1024));
            assert_eq!(Args::parse_size("10MB"), Some(10 * 1024 * 1024));
        }

        #[test]
        fn test_parse_size_gigabytes() {
            assert_eq!(Args::parse_size("1GB"), Some(1024 * 1024 * 1024));
        }

        #[test]
        fn test_parse_size_case_insensitive() {
            assert_eq!(Args::parse_size("1kb"), Some(1024));
            assert_eq!(Args::parse_size("1Kb"), Some(1024));
            assert_eq!(Args::parse_size("1mb"), Some(1024 * 1024));
        }

        #[test]
        fn test_parse_size_invalid() {
            assert_eq!(Args::parse_size("abc"), None);
            assert_eq!(Args::parse_size("MB"), None);
            assert_eq!(Args::parse_size(""), None);
        }

        #[test]
        fn test_file_type_filter_conversion() {
            assert_eq!(FileType::from(FileTypeFilter::Text), FileType::Text);
            assert_eq!(FileType::from(FileTypeFilter::Image), FileType::Image);
            assert_eq!(FileType::from(FileTypeFilter::Pdf), FileType::Pdf);
            assert_eq!(FileType::from(FileTypeFilter::Binary), FileType::Binary);
        }

        #[test]
        fn test_args_default_values() {
            let args = Args {
                directory: PathBuf::from("."),
                file_types: vec![],
                dry_run: false,
                sort_by: SortOrder::Date,
                reverse: false,
                show_hidden: false,
                min_size: None,
                max_size: None,
            };

            assert_eq!(args.directory, PathBuf::from("."));
            assert!(!args.dry_run);
            assert_eq!(args.sort_by, SortOrder::Date);
            assert!(!args.reverse);
            assert!(!args.show_hidden);
            assert!(args.get_file_type_filters().is_none());
        }

        #[test]
        fn test_args_get_file_type_filters_empty() {
            let args = Args {
                directory: PathBuf::from("."),
                file_types: vec![],
                dry_run: false,
                sort_by: SortOrder::Date,
                reverse: false,
                show_hidden: false,
                min_size: None,
                max_size: None,
            };

            assert!(args.get_file_type_filters().is_none());
        }

        #[test]
        fn test_args_get_file_type_filters_multiple() {
            let args = Args {
                directory: PathBuf::from("."),
                file_types: vec![FileTypeFilter::Text, FileTypeFilter::Image],
                dry_run: false,
                sort_by: SortOrder::Date,
                reverse: false,
                show_hidden: false,
                min_size: None,
                max_size: None,
            };

            let filters = args.get_file_type_filters().unwrap();
            assert_eq!(filters.len(), 2);
            assert!(filters.contains(&FileType::Text));
            assert!(filters.contains(&FileType::Image));
        }

        #[test]
        fn test_args_validate_nonexistent_directory() {
            let args = Args {
                directory: PathBuf::from("/nonexistent/path/12345"),
                file_types: vec![],
                dry_run: false,
                sort_by: SortOrder::Date,
                reverse: false,
                show_hidden: false,
                min_size: None,
                max_size: None,
            };

            let result = args.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("does not exist"));
        }

        #[test]
        fn test_args_validate_invalid_size_format() {
            let args = Args {
                directory: PathBuf::from("."),
                file_types: vec![],
                dry_run: false,
                sort_by: SortOrder::Date,
                reverse: false,
                show_hidden: false,
                min_size: Some("invalid".to_string()),
                max_size: None,
            };

            let result = args.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("Invalid min-size"));
        }

        #[test]
        fn test_args_validate_min_greater_than_max() {
            let args = Args {
                directory: PathBuf::from("."),
                file_types: vec![],
                dry_run: false,
                sort_by: SortOrder::Date,
                reverse: false,
                show_hidden: false,
                min_size: Some("10MB".to_string()),
                max_size: Some("1MB".to_string()),
            };

            let result = args.validate();
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("cannot be greater than"));
        }

        #[test]
        fn test_args_validate_success() {
            let args = Args {
                directory: PathBuf::from("."),
                file_types: vec![],
                dry_run: false,
                sort_by: SortOrder::Date,
                reverse: false,
                show_hidden: false,
                min_size: Some("1KB".to_string()),
                max_size: Some("100MB".to_string()),
            };

            assert!(args.validate().is_ok());
        }
    }

    mod config_tests {
        use super::*;

        #[test]
        fn test_app_config_from_args() {
            let args = Args {
                directory: PathBuf::from("/test/path"),
                file_types: vec![FileTypeFilter::Text],
                dry_run: true,
                sort_by: SortOrder::Name,
                reverse: true,
                show_hidden: true,
                min_size: Some("1KB".to_string()),
                max_size: Some("1MB".to_string()),
            };

            let config: AppConfig = args.into();

            assert_eq!(config.directory, PathBuf::from("/test/path"));
            assert!(config.dry_run);
            assert_eq!(config.sort_by, SortOrder::Name);
            assert!(config.reverse);
            assert!(config.show_hidden);
            assert_eq!(config.min_size, Some(1024));
            assert_eq!(config.max_size, Some(1024 * 1024));
            assert!(config.file_type_filters.is_some());
        }

        #[test]
        fn test_app_config_default() {
            let config = AppConfig::default();

            assert_eq!(config.directory, PathBuf::from("."));
            assert!(!config.dry_run);
            assert_eq!(config.sort_by, SortOrder::Date);
            assert!(!config.reverse);
            assert!(!config.show_hidden);
            assert!(config.min_size.is_none());
            assert!(config.max_size.is_none());
            assert!(config.file_type_filters.is_none());
        }

        #[test]
        fn test_sort_order_default() {
            assert_eq!(SortOrder::default(), SortOrder::Date);
        }
    }
}
