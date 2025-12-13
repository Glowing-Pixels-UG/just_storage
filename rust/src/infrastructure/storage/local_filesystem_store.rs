use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs::{self, File};
use tokio::io::BufReader;
use uuid::Uuid;

use crate::application::ports::{BlobReader, BlobStore, StorageError};
use crate::domain::value_objects::{ContentHash, StorageClass};
use crate::infrastructure::storage::{ContentHasher, PathBuilder};

/// Local filesystem blob store implementation
pub struct LocalFilesystemStore {
    path_builder: PathBuilder,
}

impl LocalFilesystemStore {
    pub fn new(hot_root: PathBuf, cold_root: PathBuf) -> Self {
        Self {
            path_builder: PathBuilder::new(hot_root, cold_root),
        }
    }

    /// Initialize storage directories
    pub async fn init(&self) -> Result<(), StorageError> {
        // Create directory structure
        for class in [StorageClass::Hot, StorageClass::Cold] {
            let temp_dir = self.path_builder.temp_path(class, Uuid::new_v4());
            if let Some(parent) = temp_dir.parent() {
                fs::create_dir_all(parent).await?;
            }

            // Create sha256 directory
            let root = self.path_builder.root(class);
            fs::create_dir_all(root.join("sha256")).await?;
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
        let (content_hash, size_bytes) = ContentHasher::write_and_hash(&temp_path, reader).await?;

        // 3. Move to final content-addressable location (atomic)
        let final_path = self.path_builder.final_path(storage_class, &content_hash);

        // Create parent directory if needed
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Check if file already exists (deduplication)
        if fs::metadata(&final_path).await.is_ok() {
            // File exists, just delete temp
            fs::remove_file(temp_path).await?;
        } else {
            // Atomic rename
            fs::rename(temp_path, final_path).await?;
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

        let store = LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());
        store.init().await.unwrap();

        assert!(hot_dir.path().join("sha256").exists());
        assert!(cold_dir.path().join("sha256").exists());
    }

    #[tokio::test]
    async fn test_write_and_read_blob() {
        let hot_dir = TempDir::new().unwrap();
        let cold_dir = TempDir::new().unwrap();

        let store = LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());
        store.init().await.unwrap();

        let content = b"Hello, World!";
        let reader = Box::pin(std::io::Cursor::new(content));

        let (hash, size) = store.write(reader, StorageClass::Hot).await.unwrap();

        assert_eq!(size, content.len() as u64);

        // Read back
        let mut reader = store.read(&hash, StorageClass::Hot).await.unwrap();
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).await.unwrap();

        assert_eq!(buffer, content);
    }

    #[tokio::test]
    async fn test_exists() {
        let hot_dir = TempDir::new().unwrap();
        let cold_dir = TempDir::new().unwrap();

        let store = LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());
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

        let store = LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());
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

        let store = LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());
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
