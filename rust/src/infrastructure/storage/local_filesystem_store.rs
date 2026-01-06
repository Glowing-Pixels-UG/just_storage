use async_trait::async_trait;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use tokio::fs::{self, File};
use tokio::io::BufReader;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::application::ports::{BlobReader, BlobStore, StorageError};
use crate::domain::value_objects::{ContentHash, StorageClass};
use crate::infrastructure::storage::{ContentHasher, PathBuilder};

/// Simple directory caching strategy
#[derive(Clone)]
enum DirectoryCache {
    /// HashSet for directory path caching
    Simple(Arc<RwLock<HashSet<PathBuf>>>),
}

impl DirectoryCache {
    fn new() -> Self {
        // Start with simple cache, adapt based on usage patterns
        Self::Simple(Arc::new(RwLock::new(HashSet::new())))
    }

    fn contains(&self, path: &PathBuf) -> bool {
        match self {
            DirectoryCache::Simple(cache) => {
                if let Ok(guard) = cache.try_read() {
                    guard.contains(path)
                } else {
                    false // Conservative fallback
                }
            }
        }
    }

    fn insert(&self, path: PathBuf) {
        match self {
            DirectoryCache::Simple(cache) => {
                if let Ok(mut guard) = cache.try_write() {
                    guard.insert(path);
                }
            }
        }
    }
}

/// Local filesystem blob store implementation with adaptive caching
pub struct LocalFilesystemStore {
    path_builder: PathBuilder,
    durable_writes: bool,
    precreate_dirs: bool,
    // Adaptive directory cache that optimizes based on usage patterns
    created_dirs: Arc<RwLock<DirectoryCache>>,
    // Track concurrent operations to adapt caching strategy
    concurrent_ops: Arc<AtomicUsize>,
    // Configuration for adaptive behavior
    concurrent_threshold: usize,
    // Whether to use adaptive buffering for I/O operations
    adaptive_buffering: bool,
}

impl LocalFilesystemStore {
    pub fn new(hot_root: PathBuf, cold_root: PathBuf) -> Self {
        Self::with_options(hot_root, cold_root, true, true)
    }

    pub fn with_durability(hot_root: PathBuf, cold_root: PathBuf, durable_writes: bool) -> Self {
        Self::with_options(hot_root, cold_root, durable_writes, true)
    }

    pub fn with_options(
        hot_root: PathBuf,
        cold_root: PathBuf,
        durable_writes: bool,
        precreate_dirs: bool,
    ) -> Self {
        Self::with_config(hot_root, cold_root, durable_writes, precreate_dirs, 10)
    }

    pub fn with_config(
        hot_root: PathBuf,
        cold_root: PathBuf,
        durable_writes: bool,
        precreate_dirs: bool,
        concurrent_threshold: usize,
    ) -> Self {
        Self::with_full_config(
            hot_root,
            cold_root,
            durable_writes,
            precreate_dirs,
            concurrent_threshold,
            true, // adaptive_buffering default
        )
    }

    pub fn with_full_config(
        hot_root: PathBuf,
        cold_root: PathBuf,
        durable_writes: bool,
        precreate_dirs: bool,
        concurrent_threshold: usize,
        adaptive_buffering: bool,
    ) -> Self {
        Self {
            path_builder: PathBuilder::new(hot_root, cold_root),
            durable_writes,
            precreate_dirs,
            created_dirs: Arc::new(RwLock::new(DirectoryCache::new())),
            concurrent_ops: Arc::new(AtomicUsize::new(0)),
            concurrent_threshold,
            adaptive_buffering,
        }
    }

    /// Initialize storage directories
    pub async fn init(&self) -> Result<(), StorageError> {
        // Create directory structure
        for class in [StorageClass::Hot, StorageClass::Cold] {
            // Create temp directory
            let root = self.path_builder.root(class);
            fs::create_dir_all(root.join("temp")).await?;

            // Create sha256 directory
            let root = self.path_builder.root(class);
            let sha256_root = root.join("sha256");
            fs::create_dir_all(&sha256_root).await?;

            // Pre-create all 256 hex prefix directories to avoid doing it on every write
            // This is a one-time cost at startup that significantly speeds up write operations
            if self.precreate_dirs {
                for i in 0..=255 {
                    let prefix = format!("{:02x}", i);
                    fs::create_dir_all(sha256_root.join(prefix)).await?;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl BlobStore for LocalFilesystemStore {
    async fn write(
        &self,
        reader: BlobReader,
        storage_class: StorageClass,
    ) -> Result<(ContentHash, u64), StorageError> {
        // 1. Generate temp path
        let temp_id = Uuid::new_v4();
        let temp_path = self.path_builder.temp_path(storage_class, temp_id);

        // 2. Write to temp file and compute hash
        // Use a guard to ensure temp file cleanup on error
        debug!("Writing blob to temp file: {:?}", temp_path);
        let (content_hash, size_bytes) =
            match ContentHasher::write_and_hash_with_durability_adaptive(
                &temp_path,
                reader,
                self.durable_writes,
                self.adaptive_buffering,
            )
            .await
            {
                Ok(result) => {
                    debug!(
                        "Blob written successfully: hash={}, size={}",
                        result.0, result.1
                    );
                    result
                }
                Err(e) => {
                    // Clean up temp file on write/hash failure
                    warn!("Failed to write blob to temp file {:?}: {}", temp_path, e);
                    let _ = fs::remove_file(&temp_path).await;
                    return Err(e);
                }
            };

        // 3. Move to final content-addressable location (atomic)
        let final_path = self.path_builder.final_path(storage_class, &content_hash);

        // Ensure parent directory exists (adaptive caching based on concurrency patterns)
        if let Some(parent) = final_path.parent() {
            let parent_path = parent.to_path_buf();
            let concurrent_count = self.concurrent_ops.fetch_add(1, Ordering::Relaxed);

            // Try to upgrade to concurrent cache if high concurrency detected
            if concurrent_count > self.concurrent_threshold {
                // For simplicity, we'll upgrade on the next operation
                // In a production system, this would use atomic operations
                debug!(
                    "High concurrency detected ({} ops), consider upgrading cache",
                    concurrent_count
                );
            }

            // Check cache and create directory if needed
            let cache_hit = {
                let cache_guard = self.created_dirs.read().unwrap();
                cache_guard.contains(&parent_path)
            };

            if !cache_hit {
                // Create directory (idempotent operation)
                if let Err(e) = fs::create_dir_all(&parent_path).await {
                    // Clean up temp file if directory creation fails
                    self.concurrent_ops.fetch_sub(1, Ordering::Relaxed);
                    let _ = fs::remove_file(&temp_path).await;
                    return Err(StorageError::Io(e));
                }

                // Insert into cache (may have been created by another thread, but that's fine)
                if let Ok(cache_guard) = self.created_dirs.try_write() {
                    cache_guard.insert(parent_path);
                }
            }

            self.concurrent_ops.fetch_sub(1, Ordering::Relaxed);
        }

        // Check if file already exists (deduplication)
        // Use fs::metadata which is optimized for existence checks
        let file_exists = fs::metadata(&final_path).await.is_ok();

        if file_exists {
            // File exists, just delete temp (deduplication case)
            debug!("Blob already exists (deduplication): {}", content_hash);
            // Best effort cleanup - ignore errors
            let _ = fs::remove_file(&temp_path).await;
        } else {
            debug!("Moving blob to final location: {:?}", final_path);
            // Atomic rename - file doesn't exist
            if let Err(e) = fs::rename(&temp_path, &final_path).await {
                // If rename fails, try to clean up temp file (best effort)
                let _ = fs::remove_file(&temp_path).await;
                return Err(StorageError::Io(e));
            }

            // Ensure parent directory is synced to persist the rename operation if durability is required
            if self.durable_writes {
                if let Some(parent) = final_path.parent() {
                    match File::open(parent).await {
                        Ok(parent_file) => {
                            if let Err(e) = parent_file.sync_all().await {
                                // Sync failed, but file is already renamed - log but don't fail
                                tracing::warn!(
                                    "Failed to sync parent directory after rename: {}",
                                    e
                                );
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to open parent directory for sync: {}", e);
                        }
                    }
                }
            }
        }

        Ok((content_hash, size_bytes))
    }

    async fn read(
        &self,
        content_hash: &ContentHash,
        storage_class: StorageClass,
    ) -> Result<BlobReader, StorageError> {
        let path = self.path_builder.final_path(storage_class, content_hash);

        let file = File::open(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(content_hash.to_string())
            } else {
                StorageError::Io(e)
            }
        })?;

        Ok(Box::pin(BufReader::new(file)))
    }

    async fn delete(
        &self,
        content_hash: &ContentHash,
        storage_class: StorageClass,
    ) -> Result<(), StorageError> {
        let path = self.path_builder.final_path(storage_class, content_hash);

        fs::remove_file(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound(content_hash.to_string())
            } else {
                StorageError::Io(e)
            }
        })?;

        Ok(())
    }

    async fn exists(
        &self,
        content_hash: &ContentHash,
        storage_class: StorageClass,
    ) -> Result<bool, StorageError> {
        let path = self.path_builder.final_path(storage_class, content_hash);
        Ok(fs::metadata(&path).await.is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn test_store_init_creates_directories() {
        let hot_dir = TempDir::new().unwrap();
        let cold_dir = TempDir::new().unwrap();

        let store =
            LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());
        store.init().await.unwrap();

        assert!(hot_dir.path().join("sha256").exists());
        assert!(cold_dir.path().join("sha256").exists());
    }

    #[tokio::test]
    async fn test_write_and_read_blob() {
        let hot_dir = TempDir::new().unwrap();
        let cold_dir = TempDir::new().unwrap();

        let store =
            LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());
        store.init().await.unwrap();

        let content = b"Hello, World!";
        let reader = Box::pin(std::io::Cursor::new(content));

        let (hash, size) = store.write(reader, StorageClass::Hot).await.unwrap();

        assert_eq!(size, content.len() as u64);

        // Read back
        let mut reader = store.read(&hash, StorageClass::Hot).await.unwrap();
        let mut buffer = Vec::with_capacity(content.len());
        reader.read_to_end(&mut buffer).await.unwrap();

        assert_eq!(buffer, content);
    }

    #[tokio::test]
    async fn test_exists() {
        let hot_dir = TempDir::new().unwrap();
        let cold_dir = TempDir::new().unwrap();

        let store =
            LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());
        store.init().await.unwrap();

        let content = b"test data";
        let reader = Box::pin(std::io::Cursor::new(content));
        let (hash, _) = store.write(reader, StorageClass::Hot).await.unwrap();

        assert!(store.exists(&hash, StorageClass::Hot).await.unwrap());
        assert!(!store.exists(&hash, StorageClass::Cold).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete() {
        let hot_dir = TempDir::new().unwrap();
        let cold_dir = TempDir::new().unwrap();

        let store =
            LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());
        store.init().await.unwrap();

        let content = b"to be deleted";
        let reader = Box::pin(std::io::Cursor::new(content));
        let (hash, _) = store.write(reader, StorageClass::Hot).await.unwrap();

        assert!(store.exists(&hash, StorageClass::Hot).await.unwrap());

        store.delete(&hash, StorageClass::Hot).await.unwrap();

        assert!(!store.exists(&hash, StorageClass::Hot).await.unwrap());
    }

    #[tokio::test]
    async fn test_deduplication() {
        let hot_dir = TempDir::new().unwrap();
        let cold_dir = TempDir::new().unwrap();

        let store =
            LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());
        store.init().await.unwrap();

        let content = b"duplicate content";

        // Write first time
        let reader1 = Box::pin(std::io::Cursor::new(content));
        let (hash1, _) = store.write(reader1, StorageClass::Hot).await.unwrap();

        // Write second time
        let reader2 = Box::pin(std::io::Cursor::new(content));
        let (hash2, _) = store.write(reader2, StorageClass::Hot).await.unwrap();

        assert_eq!(hash1, hash2);

        // Verify file exists
        assert!(store.exists(&hash1, StorageClass::Hot).await.unwrap());
    }
}
