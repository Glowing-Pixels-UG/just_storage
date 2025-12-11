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
            let root = match class {
                StorageClass::Hot => PathBuf::from("/data/hot"),
                StorageClass::Cold => PathBuf::from("/data/cold"),
            };
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
