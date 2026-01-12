use super::{Decision, DecisionStatistics, FileEntry};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug)]
pub struct DecisionEngine {
    pub files: Vec<FileEntry>,
    pub decisions: Vec<(usize, Decision)>,
    staging_dir: PathBuf,
    dry_run: bool,
}

impl DecisionEngine {
    pub fn new(files: Vec<FileEntry>) -> Self {
        use std::time::UNIX_EPOCH;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let staging_dir =
            std::env::temp_dir().join(format!("fswp-{}-{}", std::process::id(), timestamp));
        fs::create_dir_all(&staging_dir).ok();

        Self {
            files,
            decisions: Vec::new(),
            staging_dir,
            dry_run: false,
        }
    }

    pub fn set_dry_run(&mut self, dry_run: bool) {
        self.dry_run = dry_run;
    }

    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }

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
                self.decisions.push((index, decision));
                Ok(())
            }
            Decision::Trash => {
                if self.dry_run {
                    self.decisions.push((index, decision));
                    return Ok(());
                }

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

    pub fn undo(&mut self) -> io::Result<()> {
        let (index, decision) = self
            .decisions
            .pop()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No decisions to undo"))?;

        if self.dry_run {
            return Ok(());
        }

        match decision {
            Decision::Keep => Ok(()),
            Decision::Trash => {
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

    pub fn commit_trash_decisions(&self) -> io::Result<()> {
        for (index, decision) in &self.decisions {
            if *decision == Decision::Trash {
                let staged_path = self.get_staged_path(*index);
                if staged_path.exists() {
                    trash::delete(&staged_path).map_err(|e| io::Error::other(e.to_string()))?;
                }
            }
        }
        Ok(())
    }

    fn get_staged_path(&self, index: usize) -> PathBuf {
        self.staging_dir.join(format!("file_{}", index))
    }
}

impl Drop for DecisionEngine {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.staging_dir).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::FileType;
    use chrono::Utc;
    use std::fs;
    use std::path::PathBuf;
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
