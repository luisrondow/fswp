// Allow dead code for now since we're building incrementally with TDD
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileType {
    Text,
    Image,
    Pdf,
    Binary,
}

impl FileType {
    pub fn from_extension(ext: &str) -> Self {
        let ext = ext.to_lowercase();
        match ext.as_str() {
            // Text files
            "txt" | "md" | "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "json" | "yaml" | "yml"
            | "toml" | "xml" | "html" | "css" | "sh" | "bash" | "c" | "cpp" | "h" | "hpp"
            | "java" | "go" | "rb" | "php" | "swift" | "kt" | "cs" | "sql" => FileType::Text,

            // Image files
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "svg" | "ico" => FileType::Image,

            // PDF files
            "pdf" => FileType::Pdf,

            // Everything else is binary
            _ => FileType::Binary,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Keep,
    Trash,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub modified_date: DateTime<Utc>,
    pub file_type: FileType,
}

impl FileEntry {
    pub fn from_path(path: &Path) -> io::Result<Self> {
        let metadata = fs::metadata(path)?;
        let modified = metadata.modified()?;
        let modified_date: DateTime<Utc> = modified.into();

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let file_type = FileType::from_extension(extension);

        Ok(FileEntry {
            path: path.to_path_buf(),
            name,
            size: metadata.len(),
            modified_date,
            file_type,
        })
    }
}

#[derive(Debug)]
pub struct AppState {
    pub files: Vec<FileEntry>,
    pub current_index: usize,
    pub decisions_stack: Vec<(usize, Decision)>,
}

/// Discovers files in a directory, filtering hidden files and sorting by modification date.
///
/// # Arguments
/// * `dir_path` - The directory to scan for files
///
/// # Returns
/// * `Ok(Vec<FileEntry>)` - A vector of file entries sorted by modification date (oldest first)
/// * `Err(io::Error)` - If the directory cannot be read or accessed
///
/// # Behavior
/// - Filters out hidden files (names starting with '.')
/// - Filters out directories
/// - Does not recurse into subdirectories
/// - Sorts results by modification date in ascending order
/// - Handles permission errors gracefully by skipping inaccessible files
pub fn discover_files(dir_path: &Path) -> io::Result<Vec<FileEntry>> {
    discover_files_with_options(dir_path, &DiscoveryOptions::default())
}

/// Options for file discovery
#[derive(Debug, Clone, Default)]
pub struct DiscoveryOptions {
    /// File type filters (None = all types)
    pub file_types: Option<Vec<FileType>>,
    /// Show hidden files
    pub show_hidden: bool,
    /// Minimum file size in bytes
    pub min_size: Option<u64>,
    /// Maximum file size in bytes
    pub max_size: Option<u64>,
    /// Sort order
    pub sort_by: SortBy,
    /// Reverse sort order
    pub reverse: bool,
}

/// Sort order for files
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortBy {
    /// Sort by modification date
    #[default]
    Date,
    /// Sort by file name
    Name,
    /// Sort by file size
    Size,
    /// Sort by file type
    Type,
}

/// Discovers files with custom options
pub fn discover_files_with_options(
    dir_path: &Path,
    options: &DiscoveryOptions,
) -> io::Result<Vec<FileEntry>> {
    let mut files = Vec::new();

    // Read directory entries
    let entries = fs::read_dir(dir_path)?;

    for entry_result in entries {
        // Skip entries that cannot be read (permission errors, etc.)
        let entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // Get file name
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => continue,
        };

        // Skip hidden files unless show_hidden is true
        if !options.show_hidden && file_name.starts_with('.') {
            continue;
        }

        // Skip directories
        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if metadata.is_dir() {
            continue;
        }

        // Create FileEntry from path
        let file_entry = match FileEntry::from_path(&path) {
            Ok(fe) => fe,
            Err(_) => continue,
        };

        // Filter by file type
        if let Some(ref type_filters) = options.file_types {
            if !type_filters.contains(&file_entry.file_type) {
                continue;
            }
        }

        // Filter by min size
        if let Some(min_size) = options.min_size {
            if file_entry.size < min_size {
                continue;
            }
        }

        // Filter by max size
        if let Some(max_size) = options.max_size {
            if file_entry.size > max_size {
                continue;
            }
        }

        files.push(file_entry);
    }

    // Sort based on options
    match options.sort_by {
        SortBy::Date => files.sort_by(|a, b| a.modified_date.cmp(&b.modified_date)),
        SortBy::Name => files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        SortBy::Size => files.sort_by(|a, b| a.size.cmp(&b.size)),
        SortBy::Type => files.sort_by(|a, b| {
            let type_order = |t: &FileType| match t {
                FileType::Text => 0,
                FileType::Image => 1,
                FileType::Pdf => 2,
                FileType::Binary => 3,
            };
            type_order(&a.file_type).cmp(&type_order(&b.file_type))
        }),
    }

    // Reverse if requested
    if options.reverse {
        files.reverse();
    }

    Ok(files)
}

/// Statistics about decisions made during the session
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionStatistics {
    pub total_files: usize,
    pub kept: usize,
    pub trashed: usize,
}

/// Decision engine that manages file decisions and trash operations
#[derive(Debug)]
pub struct DecisionEngine {
    pub files: Vec<FileEntry>,
    pub decisions: Vec<(usize, Decision)>,
    staging_dir: PathBuf,
    /// Dry run mode - don't actually move files
    dry_run: bool,
}

impl DecisionEngine {
    /// Creates a new decision engine with the given files
    pub fn new(files: Vec<FileEntry>) -> Self {
        // Create a unique temporary staging directory for trash operations
        // Use process ID + timestamp + thread ID to ensure uniqueness across parallel tests
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let staging_dir =
            std::env::temp_dir().join(format!("file-tinder-{}-{}", std::process::id(), timestamp));
        fs::create_dir_all(&staging_dir).ok();

        Self {
            files,
            decisions: Vec::new(),
            staging_dir,
            dry_run: false,
        }
    }

    /// Sets dry run mode
    pub fn set_dry_run(&mut self, dry_run: bool) {
        self.dry_run = dry_run;
    }

    /// Returns whether dry run mode is enabled
    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    /// Records a decision for the file at the given index
    ///
    /// For Keep decisions, the file is left untouched.
    /// For Trash decisions, the file is moved to a staging directory (unless dry-run).
    pub fn record_decision(&mut self, index: usize, decision: Decision) -> io::Result<()> {
        if index >= self.files.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "File index out of bounds",
            ));
        }

        let file_entry = &self.files[index];
        let original_path = &file_entry.path;

        match decision {
            Decision::Keep => {
                // Keep decision - no filesystem action needed
                self.decisions.push((index, decision));
                Ok(())
            }
            Decision::Trash => {
                // In dry-run mode, just record the decision without moving files
                if self.dry_run {
                    self.decisions.push((index, decision));
                    return Ok(());
                }

                // Move file to staging directory
                if !original_path.exists() {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("File not found: {:?}", original_path),
                    ));
                }

                let staged_path = self.get_staged_path(index);
                fs::create_dir_all(staged_path.parent().unwrap())?;
                fs::rename(original_path, &staged_path)?;

                self.decisions.push((index, decision));
                Ok(())
            }
        }
    }

    /// Undoes the last decision
    ///
    /// For Keep decisions, simply removes from the stack.
    /// For Trash decisions, restores the file from the staging directory.
    pub fn undo(&mut self) -> io::Result<()> {
        let (index, decision) = self
            .decisions
            .pop()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No decisions to undo"))?;

        // In dry-run mode, just remove from decisions (no file operations)
        if self.dry_run {
            return Ok(());
        }

        match decision {
            Decision::Keep => {
                // Keep decision - no filesystem action needed
                Ok(())
            }
            Decision::Trash => {
                // Restore file from staging directory
                let file_entry = &self.files[index];
                let original_path = &file_entry.path;
                let staged_path = self.get_staged_path(index);

                if !staged_path.exists() {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Staged file not found: {:?}", staged_path),
                    ));
                }

                fs::rename(&staged_path, original_path)?;
                Ok(())
            }
        }
    }

    /// Returns statistics about the decisions made
    pub fn get_statistics(&self) -> DecisionStatistics {
        let mut kept = 0;
        let mut trashed = 0;

        for (_, decision) in &self.decisions {
            match decision {
                Decision::Keep => kept += 1,
                Decision::Trash => trashed += 1,
            }
        }

        DecisionStatistics {
            total_files: self.files.len(),
            kept,
            trashed,
        }
    }

    /// Commits all trash decisions by moving staged files to system trash
    pub fn commit_trash_decisions(&self) -> io::Result<()> {
        for (index, decision) in &self.decisions {
            if *decision == Decision::Trash {
                let staged_path = self.get_staged_path(*index);
                if staged_path.exists() {
                    trash::delete(&staged_path)
                        .map_err(|e| io::Error::other(format!("Trash error: {}", e)))?;
                }
            }
        }
        Ok(())
    }

    /// Gets the staged path for a file at the given index
    fn get_staged_path(&self, index: usize) -> PathBuf {
        self.staging_dir.join(format!("file_{}", index))
    }
}

impl Drop for DecisionEngine {
    fn drop(&mut self) {
        // Clean up staging directory
        fs::remove_dir_all(&self.staging_dir).ok();
    }
}

impl AppState {
    pub fn new(files: Vec<FileEntry>) -> Self {
        Self {
            files,
            current_index: 0,
            decisions_stack: Vec::new(),
        }
    }

    pub fn next(&mut self) {
        if self.current_index < self.files.len().saturating_sub(1) {
            self.current_index += 1;
        }
    }

    pub fn previous(&mut self) {
        if self.current_index > 0 {
            self.current_index -= 1;
        }
    }

    pub fn current_file(&self) -> Option<&FileEntry> {
        self.files.get(self.current_index)
    }

    pub fn record_decision(&mut self, decision: Decision) {
        self.decisions_stack.push((self.current_index, decision));
    }

    pub fn undo(&mut self) -> Option<(usize, Decision)> {
        self.decisions_stack.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod file_type_tests {
        use super::*;

        #[test]
        fn test_file_type_from_extension_text() {
            assert_eq!(FileType::from_extension("txt"), FileType::Text);
            assert_eq!(FileType::from_extension("rs"), FileType::Text);
            assert_eq!(FileType::from_extension("py"), FileType::Text);
            assert_eq!(FileType::from_extension("js"), FileType::Text);
            assert_eq!(FileType::from_extension("md"), FileType::Text);
        }

        #[test]
        fn test_file_type_from_extension_image() {
            assert_eq!(FileType::from_extension("png"), FileType::Image);
            assert_eq!(FileType::from_extension("jpg"), FileType::Image);
            assert_eq!(FileType::from_extension("jpeg"), FileType::Image);
            assert_eq!(FileType::from_extension("gif"), FileType::Image);
            assert_eq!(FileType::from_extension("webp"), FileType::Image);
        }

        #[test]
        fn test_file_type_from_extension_pdf() {
            assert_eq!(FileType::from_extension("pdf"), FileType::Pdf);
        }

        #[test]
        fn test_file_type_from_extension_binary() {
            assert_eq!(FileType::from_extension("exe"), FileType::Binary);
            assert_eq!(FileType::from_extension("bin"), FileType::Binary);
            assert_eq!(FileType::from_extension("unknown"), FileType::Binary);
            assert_eq!(FileType::from_extension(""), FileType::Binary);
        }

        #[test]
        fn test_file_type_case_insensitive() {
            assert_eq!(FileType::from_extension("PNG"), FileType::Image);
            assert_eq!(FileType::from_extension("TXT"), FileType::Text);
            assert_eq!(FileType::from_extension("PDF"), FileType::Pdf);
        }
    }

    mod file_entry_tests {
        use super::*;
        use std::fs;
        use tempfile::NamedTempFile;

        #[test]
        fn test_file_entry_from_path() {
            let temp_file = NamedTempFile::new().unwrap();
            let path = temp_file.path();
            fs::write(path, b"test content").unwrap();

            let entry = FileEntry::from_path(path).unwrap();

            assert_eq!(entry.path, path);
            assert!(entry.name.len() > 0);
            assert_eq!(entry.size, 12);
            assert_eq!(entry.file_type, FileType::Binary);
        }

        #[test]
        fn test_file_entry_from_path_with_extension() {
            let temp_file = NamedTempFile::new().unwrap();
            let path = temp_file.path();
            let txt_path = path.with_extension("txt");
            fs::write(&txt_path, b"hello").unwrap();

            let entry = FileEntry::from_path(&txt_path).unwrap();

            assert_eq!(entry.file_type, FileType::Text);
            assert_eq!(entry.size, 5);

            fs::remove_file(&txt_path).ok();
        }

        #[test]
        fn test_file_entry_nonexistent_file() {
            let result = FileEntry::from_path(Path::new("/nonexistent/file.txt"));
            assert!(result.is_err());
        }
    }

    mod app_state_tests {
        use super::*;

        fn create_test_entry(name: &str) -> FileEntry {
            FileEntry {
                path: PathBuf::from(name),
                name: name.to_string(),
                size: 0,
                modified_date: Utc::now(),
                file_type: FileType::Text,
            }
        }

        #[test]
        fn test_app_state_new() {
            let files = vec![
                create_test_entry("file1.txt"),
                create_test_entry("file2.txt"),
            ];
            let state = AppState::new(files.clone());

            assert_eq!(state.files.len(), 2);
            assert_eq!(state.current_index, 0);
            assert_eq!(state.decisions_stack.len(), 0);
        }

        #[test]
        fn test_app_state_next() {
            let files = vec![
                create_test_entry("file1.txt"),
                create_test_entry("file2.txt"),
            ];
            let mut state = AppState::new(files);

            assert_eq!(state.current_index, 0);

            state.next();
            assert_eq!(state.current_index, 1);

            state.next();
            assert_eq!(state.current_index, 1); // Should stay at last item
        }

        #[test]
        fn test_app_state_previous() {
            let files = vec![
                create_test_entry("file1.txt"),
                create_test_entry("file2.txt"),
            ];
            let mut state = AppState::new(files);
            state.current_index = 1;

            state.previous();
            assert_eq!(state.current_index, 0);

            state.previous();
            assert_eq!(state.current_index, 0); // Should stay at first item
        }

        #[test]
        fn test_app_state_current_file() {
            let files = vec![
                create_test_entry("file1.txt"),
                create_test_entry("file2.txt"),
            ];
            let state = AppState::new(files);

            let current = state.current_file();
            assert!(current.is_some());
            assert_eq!(current.unwrap().name, "file1.txt");
        }

        #[test]
        fn test_app_state_current_file_empty() {
            let state = AppState::new(vec![]);
            assert!(state.current_file().is_none());
        }

        #[test]
        fn test_app_state_record_decision() {
            let files = vec![create_test_entry("file1.txt")];
            let mut state = AppState::new(files);

            state.record_decision(Decision::Trash);

            assert_eq!(state.decisions_stack.len(), 1);
            assert_eq!(state.decisions_stack[0], (0, Decision::Trash));
        }

        #[test]
        fn test_app_state_undo() {
            let files = vec![
                create_test_entry("file1.txt"),
                create_test_entry("file2.txt"),
            ];
            let mut state = AppState::new(files);

            state.record_decision(Decision::Keep);
            state.next();
            state.record_decision(Decision::Trash);

            assert_eq!(state.current_index, 1);
            assert_eq!(state.decisions_stack.len(), 2);

            let undone = state.undo();
            assert!(undone.is_some());
            assert_eq!(undone.unwrap(), (1, Decision::Trash));
            assert_eq!(state.current_index, 1);
            assert_eq!(state.decisions_stack.len(), 1);
        }

        #[test]
        fn test_app_state_undo_empty() {
            let files = vec![create_test_entry("file1.txt")];
            let mut state = AppState::new(files);

            let undone = state.undo();
            assert!(undone.is_none());
        }
    }

    mod file_discovery_tests {
        use super::*;
        use std::fs;
        use std::thread;
        use std::time::Duration;
        use tempfile::TempDir;

        #[test]
        fn test_discover_files_in_directory() {
            let temp_dir = TempDir::new().unwrap();
            let dir_path = temp_dir.path();

            // Create test files
            fs::write(dir_path.join("file1.txt"), b"content1").unwrap();
            fs::write(dir_path.join("file2.rs"), b"content2").unwrap();
            fs::write(dir_path.join("file3.md"), b"content3").unwrap();

            let files = discover_files(dir_path).unwrap();

            assert_eq!(files.len(), 3);
            let names: Vec<_> = files.iter().map(|f| f.name.as_str()).collect();
            assert!(names.contains(&"file1.txt"));
            assert!(names.contains(&"file2.rs"));
            assert!(names.contains(&"file3.md"));
        }

        #[test]
        fn test_discover_files_filters_hidden_files() {
            let temp_dir = TempDir::new().unwrap();
            let dir_path = temp_dir.path();

            // Create regular and hidden files
            fs::write(dir_path.join("visible.txt"), b"content").unwrap();
            fs::write(dir_path.join(".hidden"), b"secret").unwrap();
            fs::write(dir_path.join(".gitignore"), b"ignore").unwrap();

            let files = discover_files(dir_path).unwrap();

            assert_eq!(files.len(), 1);
            assert_eq!(files[0].name, "visible.txt");
        }

        #[test]
        fn test_discover_files_filters_hidden_directories() {
            let temp_dir = TempDir::new().unwrap();
            let dir_path = temp_dir.path();

            // Create regular directory with file
            let visible_dir = dir_path.join("visible_dir");
            fs::create_dir(&visible_dir).unwrap();
            fs::write(visible_dir.join("file.txt"), b"content").unwrap();

            // Create hidden directory with file
            let hidden_dir = dir_path.join(".hidden_dir");
            fs::create_dir(&hidden_dir).unwrap();
            fs::write(hidden_dir.join("file.txt"), b"secret").unwrap();

            // Create file in root
            fs::write(dir_path.join("root.txt"), b"root").unwrap();

            let files = discover_files(dir_path).unwrap();

            // Should only find root.txt, not files in .hidden_dir
            let names: Vec<_> = files.iter().map(|f| f.name.as_str()).collect();
            assert!(names.contains(&"root.txt"));
            assert!(!names.iter().any(|n| n.contains("hidden")));
        }

        #[test]
        fn test_discover_files_sorts_by_modification_date() {
            let temp_dir = TempDir::new().unwrap();
            let dir_path = temp_dir.path();

            // Create files with delays to ensure different modification times
            fs::write(dir_path.join("oldest.txt"), b"first").unwrap();
            thread::sleep(Duration::from_millis(10));

            fs::write(dir_path.join("middle.txt"), b"second").unwrap();
            thread::sleep(Duration::from_millis(10));

            fs::write(dir_path.join("newest.txt"), b"third").unwrap();

            let files = discover_files(dir_path).unwrap();

            assert_eq!(files.len(), 3);
            // Files should be sorted by modification date (oldest first)
            assert_eq!(files[0].name, "oldest.txt");
            assert_eq!(files[1].name, "middle.txt");
            assert_eq!(files[2].name, "newest.txt");

            // Verify dates are in ascending order
            assert!(files[0].modified_date <= files[1].modified_date);
            assert!(files[1].modified_date <= files[2].modified_date);
        }

        #[test]
        fn test_discover_files_empty_directory() {
            let temp_dir = TempDir::new().unwrap();
            let dir_path = temp_dir.path();

            let files = discover_files(dir_path).unwrap();

            assert_eq!(files.len(), 0);
        }

        #[test]
        fn test_discover_files_nonexistent_directory() {
            let result = discover_files(Path::new("/nonexistent/directory"));
            assert!(result.is_err());
        }

        #[test]
        fn test_discover_files_only_files_not_directories() {
            let temp_dir = TempDir::new().unwrap();
            let dir_path = temp_dir.path();

            // Create files and subdirectories
            fs::write(dir_path.join("file.txt"), b"content").unwrap();
            fs::create_dir(dir_path.join("subdir")).unwrap();
            fs::write(dir_path.join("subdir").join("nested.txt"), b"nested").unwrap();

            let files = discover_files(dir_path).unwrap();

            // Should only include the root file, not the subdirectory or its contents
            assert_eq!(files.len(), 1);
            assert_eq!(files[0].name, "file.txt");
        }

        #[test]
        fn test_discover_with_file_type_filter() {
            let temp_dir = TempDir::new().unwrap();
            let dir_path = temp_dir.path();

            fs::write(dir_path.join("code.rs"), b"fn main() {}").unwrap();
            fs::write(dir_path.join("image.png"), b"PNG").unwrap();
            fs::write(dir_path.join("notes.txt"), b"notes").unwrap();

            let options = DiscoveryOptions {
                file_types: Some(vec![FileType::Text]),
                ..Default::default()
            };

            let files = discover_files_with_options(dir_path, &options).unwrap();

            assert_eq!(files.len(), 2);
            let names: Vec<_> = files.iter().map(|f| f.name.as_str()).collect();
            assert!(names.contains(&"code.rs"));
            assert!(names.contains(&"notes.txt"));
            assert!(!names.contains(&"image.png"));
        }

        #[test]
        fn test_discover_with_size_filter() {
            let temp_dir = TempDir::new().unwrap();
            let dir_path = temp_dir.path();

            fs::write(dir_path.join("small.txt"), b"hi").unwrap();
            fs::write(dir_path.join("medium.txt"), b"hello world!").unwrap();
            fs::write(dir_path.join("large.txt"), vec![b'x'; 1000]).unwrap();

            let options = DiscoveryOptions {
                min_size: Some(10),
                max_size: Some(100),
                ..Default::default()
            };

            let files = discover_files_with_options(dir_path, &options).unwrap();

            assert_eq!(files.len(), 1);
            assert_eq!(files[0].name, "medium.txt");
        }

        #[test]
        fn test_discover_with_show_hidden() {
            let temp_dir = TempDir::new().unwrap();
            let dir_path = temp_dir.path();

            fs::write(dir_path.join("visible.txt"), b"visible").unwrap();
            fs::write(dir_path.join(".hidden"), b"hidden").unwrap();

            let options = DiscoveryOptions {
                show_hidden: true,
                ..Default::default()
            };

            let files = discover_files_with_options(dir_path, &options).unwrap();

            assert_eq!(files.len(), 2);
            let names: Vec<_> = files.iter().map(|f| f.name.as_str()).collect();
            assert!(names.contains(&"visible.txt"));
            assert!(names.contains(&".hidden"));
        }

        #[test]
        fn test_discover_sort_by_name() {
            let temp_dir = TempDir::new().unwrap();
            let dir_path = temp_dir.path();

            fs::write(dir_path.join("charlie.txt"), b"c").unwrap();
            fs::write(dir_path.join("alpha.txt"), b"a").unwrap();
            fs::write(dir_path.join("bravo.txt"), b"b").unwrap();

            let options = DiscoveryOptions {
                sort_by: SortBy::Name,
                ..Default::default()
            };

            let files = discover_files_with_options(dir_path, &options).unwrap();

            assert_eq!(files[0].name, "alpha.txt");
            assert_eq!(files[1].name, "bravo.txt");
            assert_eq!(files[2].name, "charlie.txt");
        }

        #[test]
        fn test_discover_sort_by_size() {
            let temp_dir = TempDir::new().unwrap();
            let dir_path = temp_dir.path();

            fs::write(dir_path.join("large.txt"), vec![b'x'; 100]).unwrap();
            fs::write(dir_path.join("small.txt"), b"s").unwrap();
            fs::write(dir_path.join("medium.txt"), vec![b'x'; 50]).unwrap();

            let options = DiscoveryOptions {
                sort_by: SortBy::Size,
                ..Default::default()
            };

            let files = discover_files_with_options(dir_path, &options).unwrap();

            assert_eq!(files[0].name, "small.txt");
            assert_eq!(files[1].name, "medium.txt");
            assert_eq!(files[2].name, "large.txt");
        }

        #[test]
        fn test_discover_reverse_sort() {
            let temp_dir = TempDir::new().unwrap();
            let dir_path = temp_dir.path();

            fs::write(dir_path.join("alpha.txt"), b"a").unwrap();
            fs::write(dir_path.join("bravo.txt"), b"b").unwrap();
            fs::write(dir_path.join("charlie.txt"), b"c").unwrap();

            let options = DiscoveryOptions {
                sort_by: SortBy::Name,
                reverse: true,
                ..Default::default()
            };

            let files = discover_files_with_options(dir_path, &options).unwrap();

            assert_eq!(files[0].name, "charlie.txt");
            assert_eq!(files[1].name, "bravo.txt");
            assert_eq!(files[2].name, "alpha.txt");
        }
    }

    mod decision_engine_tests {
        use super::*;
        use std::fs;
        use tempfile::TempDir;

        fn create_test_entry_with_path(path: PathBuf) -> FileEntry {
            FileEntry {
                path: path.clone(),
                name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("test")
                    .to_string(),
                size: 0,
                modified_date: Utc::now(),
                file_type: FileType::Text,
            }
        }

        #[test]
        fn test_decision_engine_new() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, b"content").unwrap();

            let files = vec![create_test_entry_with_path(file_path)];
            let engine = DecisionEngine::new(files);

            assert_eq!(engine.decisions.len(), 0);
        }

        #[test]
        fn test_decision_engine_record_keep() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, b"content").unwrap();

            let entry = create_test_entry_with_path(file_path.clone());
            let mut engine = DecisionEngine::new(vec![entry]);

            let result = engine.record_decision(0, Decision::Keep);
            assert!(result.is_ok());
            assert_eq!(engine.decisions.len(), 1);

            // Keep should not modify the filesystem
            assert!(file_path.exists());
        }

        #[test]
        fn test_decision_engine_record_trash() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, b"content").unwrap();

            let entry = create_test_entry_with_path(file_path.clone());
            let mut engine = DecisionEngine::new(vec![entry]);

            let result = engine.record_decision(0, Decision::Trash);
            assert!(result.is_ok());
            assert_eq!(engine.decisions.len(), 1);

            // Trash should move file to trash
            assert!(!file_path.exists());
        }

        #[test]
        fn test_decision_engine_undo_keep() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, b"content").unwrap();

            let entry = create_test_entry_with_path(file_path.clone());
            let mut engine = DecisionEngine::new(vec![entry]);

            engine.record_decision(0, Decision::Keep).unwrap();
            assert_eq!(engine.decisions.len(), 1);

            let result = engine.undo();
            assert!(result.is_ok());
            assert_eq!(engine.decisions.len(), 0);

            // File should still exist after undoing Keep
            assert!(file_path.exists());
        }

        #[test]
        fn test_decision_engine_undo_trash() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, b"content").unwrap();

            let entry = create_test_entry_with_path(file_path.clone());
            let mut engine = DecisionEngine::new(vec![entry]);

            engine.record_decision(0, Decision::Trash).unwrap();
            assert!(!file_path.exists());

            let result = engine.undo();
            assert!(result.is_ok());
            assert_eq!(engine.decisions.len(), 0);

            // File should be restored after undoing Trash
            assert!(file_path.exists());
        }

        #[test]
        fn test_decision_engine_undo_empty() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, b"content").unwrap();

            let entry = create_test_entry_with_path(file_path);
            let mut engine = DecisionEngine::new(vec![entry]);

            let result = engine.undo();
            assert!(result.is_err());
        }

        #[test]
        fn test_decision_engine_multiple_decisions() {
            let temp_dir = TempDir::new().unwrap();

            let file1 = temp_dir.path().join("file1.txt");
            let file2 = temp_dir.path().join("file2.txt");
            let file3 = temp_dir.path().join("file3.txt");

            fs::write(&file1, b"content1").unwrap();
            fs::write(&file2, b"content2").unwrap();
            fs::write(&file3, b"content3").unwrap();

            let files = vec![
                create_test_entry_with_path(file1.clone()),
                create_test_entry_with_path(file2.clone()),
                create_test_entry_with_path(file3.clone()),
            ];

            let mut engine = DecisionEngine::new(files);

            engine.record_decision(0, Decision::Keep).unwrap();
            engine.record_decision(1, Decision::Trash).unwrap();
            engine.record_decision(2, Decision::Keep).unwrap();

            assert_eq!(engine.decisions.len(), 3);
            assert!(file1.exists());
            assert!(!file2.exists());
            assert!(file3.exists());
        }

        #[test]
        fn test_decision_engine_undo_multiple() {
            let temp_dir = TempDir::new().unwrap();

            let file1 = temp_dir.path().join("file1.txt");
            let file2 = temp_dir.path().join("file2.txt");

            fs::write(&file1, b"content1").unwrap();
            fs::write(&file2, b"content2").unwrap();

            let files = vec![
                create_test_entry_with_path(file1.clone()),
                create_test_entry_with_path(file2.clone()),
            ];

            let mut engine = DecisionEngine::new(files);

            engine.record_decision(0, Decision::Trash).unwrap();
            engine.record_decision(1, Decision::Trash).unwrap();

            assert!(!file1.exists());
            assert!(!file2.exists());
            assert_eq!(engine.decisions.len(), 2);

            // Undo second trash
            engine.undo().unwrap();
            assert!(!file1.exists());
            assert!(file2.exists());
            assert_eq!(engine.decisions.len(), 1);

            // Undo first trash
            engine.undo().unwrap();
            assert!(file1.exists());
            assert!(file2.exists());
            assert_eq!(engine.decisions.len(), 0);
        }

        #[test]
        fn test_decision_engine_trash_nonexistent_file() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("nonexistent.txt");

            let entry = create_test_entry_with_path(file_path);
            let mut engine = DecisionEngine::new(vec![entry]);

            let result = engine.record_decision(0, Decision::Trash);
            assert!(result.is_err());
        }

        #[test]
        fn test_decision_engine_get_statistics() {
            let temp_dir = TempDir::new().unwrap();

            let file1 = temp_dir.path().join("file1.txt");
            let file2 = temp_dir.path().join("file2.txt");
            let file3 = temp_dir.path().join("file3.txt");
            let file4 = temp_dir.path().join("file4.txt");

            fs::write(&file1, b"content1").unwrap();
            fs::write(&file2, b"content2").unwrap();
            fs::write(&file3, b"content3").unwrap();
            fs::write(&file4, b"content4").unwrap();

            let files = vec![
                create_test_entry_with_path(file1),
                create_test_entry_with_path(file2),
                create_test_entry_with_path(file3),
                create_test_entry_with_path(file4),
            ];

            let mut engine = DecisionEngine::new(files);

            engine.record_decision(0, Decision::Keep).unwrap();
            engine.record_decision(1, Decision::Trash).unwrap();
            engine.record_decision(2, Decision::Trash).unwrap();
            engine.record_decision(3, Decision::Keep).unwrap();

            let stats = engine.get_statistics();
            assert_eq!(stats.total_files, 4);
            assert_eq!(stats.kept, 2);
            assert_eq!(stats.trashed, 2);
        }

        #[test]
        fn test_decision_engine_dry_run_trash() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, b"content").unwrap();

            let entry = create_test_entry_with_path(file_path.clone());
            let mut engine = DecisionEngine::new(vec![entry]);
            engine.set_dry_run(true);

            // In dry-run mode, trash should NOT move the file
            let result = engine.record_decision(0, Decision::Trash);
            assert!(result.is_ok());
            assert_eq!(engine.decisions.len(), 1);

            // File should still exist (not moved)
            assert!(file_path.exists());
        }

        #[test]
        fn test_decision_engine_dry_run_undo() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, b"content").unwrap();

            let entry = create_test_entry_with_path(file_path.clone());
            let mut engine = DecisionEngine::new(vec![entry]);
            engine.set_dry_run(true);

            engine.record_decision(0, Decision::Trash).unwrap();
            assert_eq!(engine.decisions.len(), 1);

            let result = engine.undo();
            assert!(result.is_ok());
            assert_eq!(engine.decisions.len(), 0);

            // File should still exist
            assert!(file_path.exists());
        }

        #[test]
        fn test_decision_engine_is_dry_run() {
            let engine = DecisionEngine::new(vec![]);
            assert!(!engine.is_dry_run());

            let mut engine2 = DecisionEngine::new(vec![]);
            engine2.set_dry_run(true);
            assert!(engine2.is_dry_run());
        }
    }
}
