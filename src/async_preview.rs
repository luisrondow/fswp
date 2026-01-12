// Async preview module for background preview loading with caching
#![allow(dead_code)]

use crate::domain::FileEntry;
use crate::preview::{generate_preview, PreviewContent};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

/// Maximum number of cached previews
const CACHE_SIZE: usize = 10;

/// Represents a preview loading state
#[derive(Debug, Clone)]
pub enum PreviewState {
    /// Preview is loading
    Loading,
    /// Preview is ready with content
    Ready(PreviewContent),
    /// Preview failed with error
    Error(String),
}

/// Message types for the preview loader
enum PreviewRequest {
    /// Load a preview for a file
    Load {
        file_entry: FileEntry,
        response_tx: oneshot::Sender<PreviewState>,
    },
    /// Cancel any pending preview for a path
    Cancel { path: PathBuf },
    /// Shutdown the loader
    Shutdown,
}

/// LRU-like cache for previews
#[derive(Debug)]
struct PreviewCache {
    /// Cached previews mapped by file path
    cache: HashMap<PathBuf, PreviewContent>,
    /// Order of access for LRU eviction (most recent at end)
    access_order: Vec<PathBuf>,
    /// Maximum cache size
    max_size: usize,
}

impl PreviewCache {
    fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            access_order: Vec::new(),
            max_size,
        }
    }

    /// Get a cached preview, updating access order
    fn get(&mut self, path: &PathBuf) -> Option<PreviewContent> {
        if let Some(preview) = self.cache.get(path) {
            // Update access order (move to end)
            self.access_order.retain(|p| p != path);
            self.access_order.push(path.clone());
            Some(preview.clone())
        } else {
            None
        }
    }

    /// Insert a preview, evicting oldest if necessary
    fn insert(&mut self, path: PathBuf, preview: PreviewContent) {
        // Remove if already exists
        if self.cache.contains_key(&path) {
            self.access_order.retain(|p| p != &path);
        }

        // Evict oldest if at capacity
        if self.cache.len() >= self.max_size && !self.cache.contains_key(&path) {
            if let Some(oldest) = self.access_order.first().cloned() {
                self.cache.remove(&oldest);
                self.access_order.remove(0);
            }
        }

        // Insert new entry
        self.cache.insert(path.clone(), preview);
        self.access_order.push(path);
    }

    /// Check if a path is cached
    fn contains(&self, path: &PathBuf) -> bool {
        self.cache.contains_key(path)
    }

    /// Get cache size
    fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Clear the cache
    fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }
}

/// Handle for sending requests to the preview loader
#[derive(Clone)]
pub struct PreviewLoader {
    request_tx: mpsc::Sender<PreviewRequest>,
    cache: Arc<Mutex<PreviewCache>>,
    /// Track current loading path to allow cancellation
    current_loading: Arc<Mutex<Option<PathBuf>>>,
}

impl PreviewLoader {
    /// Create a new preview loader with a background task
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel(32);
        let cache = Arc::new(Mutex::new(PreviewCache::new(CACHE_SIZE)));
        let current_loading = Arc::new(Mutex::new(None));

        let loader = Self {
            request_tx,
            cache: Arc::clone(&cache),
            current_loading: Arc::clone(&current_loading),
        };

        // Spawn the background worker
        let cache_clone = Arc::clone(&cache);
        let current_loading_clone = Arc::clone(&current_loading);
        tokio::spawn(async move {
            Self::worker(request_rx, cache_clone, current_loading_clone).await;
        });

        loader
    }

    /// Background worker that processes preview requests
    async fn worker(
        mut request_rx: mpsc::Receiver<PreviewRequest>,
        cache: Arc<Mutex<PreviewCache>>,
        current_loading: Arc<Mutex<Option<PathBuf>>>,
    ) {
        while let Some(request) = request_rx.recv().await {
            match request {
                PreviewRequest::Load {
                    file_entry,
                    response_tx,
                } => {
                    let path = file_entry.path.clone();

                    // Check cache first
                    {
                        let mut cache_guard = cache.lock().await;
                        if let Some(cached) = cache_guard.get(&path) {
                            let _ = response_tx.send(PreviewState::Ready(cached));
                            continue;
                        }
                    }

                    // Mark as currently loading
                    {
                        let mut loading = current_loading.lock().await;
                        *loading = Some(path.clone());
                    }

                    // Generate preview (this is the expensive part)
                    let result =
                        tokio::task::spawn_blocking(move || generate_preview(&file_entry)).await;

                    // Check if cancelled
                    {
                        let loading = current_loading.lock().await;
                        if loading.as_ref() != Some(&path) {
                            // Was cancelled, don't send response
                            continue;
                        }
                    }

                    // Process result
                    let state = match result {
                        Ok(Ok(preview)) => {
                            // Cache the result
                            {
                                let mut cache_guard = cache.lock().await;
                                cache_guard.insert(path.clone(), preview.clone());
                            }
                            PreviewState::Ready(preview)
                        }
                        Ok(Err(e)) => PreviewState::Error(e.to_string()),
                        Err(e) => PreviewState::Error(format!("Task panicked: {}", e)),
                    };

                    // Clear current loading
                    {
                        let mut loading = current_loading.lock().await;
                        if loading.as_ref() == Some(&path) {
                            *loading = None;
                        }
                    }

                    let _ = response_tx.send(state);
                }
                PreviewRequest::Cancel { path } => {
                    let mut loading = current_loading.lock().await;
                    if loading.as_ref() == Some(&path) {
                        *loading = None;
                    }
                }
                PreviewRequest::Shutdown => {
                    break;
                }
            }
        }
    }

    /// Request a preview, returns immediately with cached result or Loading state
    pub async fn request_preview(&self, file_entry: &FileEntry) -> PreviewState {
        let path = file_entry.path.clone();

        // Check cache first
        {
            let mut cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&path) {
                return PreviewState::Ready(cached);
            }
        }

        // Create response channel
        let (response_tx, response_rx) = oneshot::channel();

        // Send request
        let request = PreviewRequest::Load {
            file_entry: file_entry.clone(),
            response_tx,
        };

        if self.request_tx.send(request).await.is_err() {
            return PreviewState::Error("Preview loader shut down".to_string());
        }

        // Wait for response with timeout
        match tokio::time::timeout(std::time::Duration::from_secs(5), response_rx).await {
            Ok(Ok(state)) => state,
            Ok(Err(_)) => PreviewState::Error("Response channel closed".to_string()),
            Err(_) => PreviewState::Error("Preview timed out".to_string()),
        }
    }

    /// Try to get a cached preview without loading
    pub async fn get_cached(&self, path: &PathBuf) -> Option<PreviewContent> {
        let mut cache = self.cache.lock().await;
        cache.get(path)
    }

    /// Check if a preview is cached
    pub async fn is_cached(&self, path: &PathBuf) -> bool {
        let cache = self.cache.lock().await;
        cache.contains(path)
    }

    /// Cancel the current loading preview
    pub async fn cancel_current(&self) {
        let path = {
            let loading = self.current_loading.lock().await;
            loading.clone()
        };

        if let Some(path) = path {
            let _ = self.request_tx.send(PreviewRequest::Cancel { path }).await;
        }
    }

    /// Cancel loading for a specific path
    pub async fn cancel(&self, path: PathBuf) {
        let _ = self.request_tx.send(PreviewRequest::Cancel { path }).await;
    }

    /// Shutdown the preview loader
    pub async fn shutdown(&self) {
        let _ = self.request_tx.send(PreviewRequest::Shutdown).await;
    }

    /// Get the number of cached previews
    pub async fn cache_size(&self) -> usize {
        let cache = self.cache.lock().await;
        cache.len()
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
    }
}

impl Default for PreviewLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Synchronous wrapper for preview loading in non-async contexts
/// Uses a polling approach for integration with synchronous TUI loops
pub struct SyncPreviewManager {
    loader: PreviewLoader,
    runtime: tokio::runtime::Runtime,
    /// Current preview state for the active file
    current_state: PreviewState,
    /// Path of the file we're currently showing/loading
    current_path: Option<PathBuf>,
    /// Receiver for the current pending preview request
    receiver: Option<oneshot::Receiver<PreviewState>>,
}

impl SyncPreviewManager {
    /// Create a new sync preview manager
    pub fn new() -> Self {
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        let loader = runtime.block_on(async { PreviewLoader::new() });

        Self {
            loader,
            runtime,
            current_state: PreviewState::Loading,
            current_path: None,
            receiver: None,
        }
    }

    /// Request a preview for a file, returns current state (non-blocking)
    pub fn request_preview(&mut self, file_entry: &FileEntry) -> &PreviewState {
        let path = file_entry.path.clone();

        // If different path, start new request
        if self.current_path.as_ref() != Some(&path) {
            // Cancel previous if any
            if self.current_path.is_some() {
                self.runtime.block_on(self.loader.cancel_current());
            }

            self.current_path = Some(path.clone());
            self.receiver = None;

            // Check cache first (sync/block_on lock but fast)
            if let Some(cached) = self.runtime.block_on(self.loader.get_cached(&path)) {
                self.current_state = PreviewState::Ready(cached);
                return &self.current_state;
            }

            // Start loading in background
            self.current_state = PreviewState::Loading;

            let (tx, rx) = oneshot::channel();
            let loader = self.loader.clone();
            let file_entry_clone = file_entry.clone();

            // Send request to background worker
            let request = PreviewRequest::Load {
                file_entry: file_entry_clone,
                response_tx: tx,
            };

            // Send request (using block_on for the Send itself to ensure it's queued)
            let _ = self
                .runtime
                .block_on(async move { loader.request_tx.send(request).await });

            self.receiver = Some(rx);
        }

        // If we're loading, check if the receiver has a value
        if matches!(self.current_state, PreviewState::Loading) {
            if let Some(ref mut rx) = self.receiver {
                match rx.try_recv() {
                    Ok(state) => {
                        self.current_state = state;
                        self.receiver = None;
                    }
                    Err(oneshot::error::TryRecvError::Empty) => {
                        // Still loading, check if cache has it (maybe from elsewhere)
                        if let Some(cached) = self.runtime.block_on(self.loader.get_cached(&path)) {
                            self.current_state = PreviewState::Ready(cached);
                            self.receiver = None;
                        }
                    }
                    Err(oneshot::error::TryRecvError::Closed) => {
                        // Channel closed, try checking cache one last time
                        if let Some(cached) = self.runtime.block_on(self.loader.get_cached(&path)) {
                            self.current_state = PreviewState::Ready(cached);
                        } else {
                            self.current_state =
                                PreviewState::Error("Preview channel closed".to_string());
                        }
                        self.receiver = None;
                    }
                }
            }
        }

        &self.current_state
    }

    /// Poll for preview completion (non-blocking) - now identical to request_preview for simplicity
    pub fn poll_preview(&mut self, file_entry: &FileEntry) -> &PreviewState {
        self.request_preview(file_entry)
    }

    /// Get the current preview state
    pub fn current_state(&self) -> &PreviewState {
        &self.current_state
    }

    /// Reset the manager (e.g., when changing files)
    pub fn reset(&mut self) {
        if self.current_path.is_some() {
            self.runtime.block_on(self.loader.cancel_current());
        }
        self.current_path = None;
        self.current_state = PreviewState::Loading;
        self.receiver = None;
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.runtime.block_on(self.loader.cache_size())
    }
}

impl Default for SyncPreviewManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::FileType;
    use chrono::Utc;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_file_entry(path: PathBuf, name: &str, file_type: FileType) -> FileEntry {
        FileEntry {
            path,
            name: name.to_string(),
            size: 100,
            modified_date: Utc::now(),
            file_type,
        }
    }

    // Cache tests
    mod cache_tests {
        use super::*;

        #[test]
        fn test_cache_new() {
            let cache = PreviewCache::new(5);
            assert!(cache.is_empty());
            assert_eq!(cache.len(), 0);
        }

        #[test]
        fn test_cache_insert_and_get() {
            let mut cache = PreviewCache::new(5);
            let path = PathBuf::from("/test/file.txt");
            let preview = PreviewContent::Text(vec!["line1".to_string(), "line2".to_string()]);

            cache.insert(path.clone(), preview.clone());

            assert!(!cache.is_empty());
            assert_eq!(cache.len(), 1);
            assert!(cache.contains(&path));

            let cached = cache.get(&path);
            assert!(cached.is_some());
            match cached.unwrap() {
                PreviewContent::Text(lines) => {
                    assert_eq!(lines, vec!["line1".to_string(), "line2".to_string()]);
                }
                _ => panic!("Expected Text content"),
            }
        }

        #[test]
        fn test_cache_lru_eviction() {
            let mut cache = PreviewCache::new(3);

            // Insert 3 items
            for i in 0..3 {
                let path = PathBuf::from(format!("/test/file{}.txt", i));
                cache.insert(path, PreviewContent::Text(vec![format!("preview {}", i)]));
            }

            assert_eq!(cache.len(), 3);

            // Insert 4th item, should evict first
            let path4 = PathBuf::from("/test/file3.txt");
            cache.insert(path4, PreviewContent::Text(vec!["preview 3".to_string()]));

            assert_eq!(cache.len(), 3);
            assert!(!cache.contains(&PathBuf::from("/test/file0.txt")));
            assert!(cache.contains(&PathBuf::from("/test/file1.txt")));
            assert!(cache.contains(&PathBuf::from("/test/file2.txt")));
            assert!(cache.contains(&PathBuf::from("/test/file3.txt")));
        }

        #[test]
        fn test_cache_access_updates_order() {
            let mut cache = PreviewCache::new(3);

            // Insert 3 items
            for i in 0..3 {
                let path = PathBuf::from(format!("/test/file{}.txt", i));
                cache.insert(path, PreviewContent::Text(vec![format!("preview {}", i)]));
            }

            // Access the first item (making it most recently used)
            let _ = cache.get(&PathBuf::from("/test/file0.txt"));

            // Insert 4th item, should evict file1 (oldest accessed)
            let path4 = PathBuf::from("/test/file3.txt");
            cache.insert(path4, PreviewContent::Text(vec!["preview 3".to_string()]));

            assert!(cache.contains(&PathBuf::from("/test/file0.txt"))); // Was accessed, should remain
            assert!(!cache.contains(&PathBuf::from("/test/file1.txt"))); // Should be evicted
            assert!(cache.contains(&PathBuf::from("/test/file2.txt")));
            assert!(cache.contains(&PathBuf::from("/test/file3.txt")));
        }

        #[test]
        fn test_cache_clear() {
            let mut cache = PreviewCache::new(5);

            for i in 0..3 {
                let path = PathBuf::from(format!("/test/file{}.txt", i));
                cache.insert(path, PreviewContent::Text(vec![format!("preview {}", i)]));
            }

            assert_eq!(cache.len(), 3);

            cache.clear();

            assert!(cache.is_empty());
            assert_eq!(cache.len(), 0);
        }

        #[test]
        fn test_cache_update_existing() {
            let mut cache = PreviewCache::new(5);
            let path = PathBuf::from("/test/file.txt");

            cache.insert(
                path.clone(),
                PreviewContent::Text(vec!["old preview".to_string()]),
            );
            cache.insert(
                path.clone(),
                PreviewContent::Text(vec!["new preview".to_string()]),
            );

            assert_eq!(cache.len(), 1);
            let cached = cache.get(&path).unwrap();
            match cached {
                PreviewContent::Text(lines) => {
                    assert_eq!(lines, vec!["new preview".to_string()]);
                }
                _ => panic!("Expected Text content"),
            }
        }
    }

    // Async loader tests
    mod async_loader_tests {
        use super::*;

        #[tokio::test]
        async fn test_preview_loader_creation() {
            let loader = PreviewLoader::new();
            assert_eq!(loader.cache_size().await, 0);
        }

        #[tokio::test]
        async fn test_preview_loader_caches_result() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, "Hello, World!").unwrap();

            let file_entry = create_test_file_entry(file_path.clone(), "test.txt", FileType::Text);

            let loader = PreviewLoader::new();

            // First request should generate preview
            let state = loader.request_preview(&file_entry).await;
            assert!(matches!(state, PreviewState::Ready(_)));

            // Should be cached now
            assert!(loader.is_cached(&file_path).await);
            assert_eq!(loader.cache_size().await, 1);

            // Second request should return cached
            let state2 = loader.request_preview(&file_entry).await;
            assert!(matches!(state2, PreviewState::Ready(_)));
        }

        #[tokio::test]
        async fn test_preview_loader_handles_nonexistent_file() {
            let file_entry = create_test_file_entry(
                PathBuf::from("/nonexistent/file.txt"),
                "file.txt",
                FileType::Text,
            );

            let loader = PreviewLoader::new();
            let state = loader.request_preview(&file_entry).await;

            // Should return error state (file not found)
            assert!(matches!(state, PreviewState::Error(_)));
        }

        #[tokio::test]
        async fn test_preview_loader_clear_cache() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, "Hello").unwrap();

            let file_entry = create_test_file_entry(file_path.clone(), "test.txt", FileType::Text);

            let loader = PreviewLoader::new();
            let _ = loader.request_preview(&file_entry).await;

            assert_eq!(loader.cache_size().await, 1);

            loader.clear_cache().await;
            assert_eq!(loader.cache_size().await, 0);
        }

        #[tokio::test]
        async fn test_preview_loader_shutdown() {
            let loader = PreviewLoader::new();
            loader.shutdown().await;

            // After shutdown, requests should fail gracefully
            let file_entry =
                create_test_file_entry(PathBuf::from("/test/file.txt"), "file.txt", FileType::Text);

            // Give time for shutdown to complete
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let state = loader.request_preview(&file_entry).await;
            // Should either timeout or return error
            assert!(matches!(
                state,
                PreviewState::Error(_) | PreviewState::Loading
            ));
        }
    }

    // Sync manager tests
    mod sync_manager_tests {
        use super::*;

        #[test]
        fn test_sync_manager_creation() {
            let manager = SyncPreviewManager::new();
            assert_eq!(manager.cache_size(), 0);
            assert!(matches!(manager.current_state(), PreviewState::Loading));
        }

        #[test]
        fn test_sync_manager_request_preview() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, "Test content").unwrap();

            let file_entry = create_test_file_entry(file_path, "test.txt", FileType::Text);

            let mut manager = SyncPreviewManager::new();

            // First request should be Loading
            let state = manager.request_preview(&file_entry);
            assert!(matches!(state, PreviewState::Loading));

            // Poll until ready (with timeout)
            let mut ready = false;
            for _ in 0..10 {
                let state = manager.poll_preview(&file_entry);
                if matches!(state, PreviewState::Ready(_)) {
                    ready = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }

            assert!(ready, "Preview should be ready within timeout");
        }

        #[test]
        fn test_sync_manager_caches_result() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, "Test content").unwrap();

            let file_entry = create_test_file_entry(file_path, "test.txt", FileType::Text);

            let mut manager = SyncPreviewManager::new();

            // First request
            let _ = manager.request_preview(&file_entry);

            // Wait for it to be cached
            let mut cached = false;
            for _ in 0..10 {
                if manager.cache_size() > 0 {
                    cached = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            assert!(cached, "Result should be cached within timeout");

            // Second request (same file) should be Ready immediately
            let state = manager.request_preview(&file_entry);
            assert!(matches!(state, PreviewState::Ready(_)));
        }

        #[test]
        fn test_sync_manager_reset() {
            let temp_dir = TempDir::new().unwrap();
            let file_path = temp_dir.path().join("test.txt");
            fs::write(&file_path, "Test content").unwrap();

            let file_entry = create_test_file_entry(file_path, "test.txt", FileType::Text);

            let mut manager = SyncPreviewManager::new();
            let _ = manager.request_preview(&file_entry);

            manager.reset();
            assert!(matches!(manager.current_state(), PreviewState::Loading));
        }

        #[test]
        fn test_sync_manager_handles_file_change() {
            let temp_dir = TempDir::new().unwrap();

            let file1 = temp_dir.path().join("file1.txt");
            let file2 = temp_dir.path().join("file2.txt");
            fs::write(&file1, "Content 1").unwrap();
            fs::write(&file2, "Content 2").unwrap();

            let entry1 = create_test_file_entry(file1, "file1.txt", FileType::Text);
            let entry2 = create_test_file_entry(file2, "file2.txt", FileType::Text);

            let mut manager = SyncPreviewManager::new();

            // Load first file
            manager.request_preview(&entry1);

            // Poll until ready
            let mut ready = false;
            for _ in 0..10 {
                if matches!(manager.poll_preview(&entry1), PreviewState::Ready(_)) {
                    ready = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            assert!(ready, "First file should be ready");

            // Load second file
            manager.request_preview(&entry2);

            // Poll until ready
            ready = false;
            for _ in 0..10 {
                if matches!(manager.poll_preview(&entry2), PreviewState::Ready(_)) {
                    ready = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            assert!(ready, "Second file should be ready");

            // Both should be cached
            assert_eq!(manager.cache_size(), 2);
        }
    }
}
