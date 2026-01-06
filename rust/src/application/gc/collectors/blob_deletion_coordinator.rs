use std::sync::Arc;
use tracing::{debug, warn};

use crate::application::ports::{BlobRepository, BlobStore};
use crate::domain::value_objects::{ContentHash, StorageClass};

/// Result of a blob deletion operation
#[derive(Debug)]
pub struct BlobDeletionResult {
    pub content_hash: ContentHash,
    pub file_deleted: bool,
    pub db_entry_deleted: bool,
}

/// Detailed result of a blob deletion attempt with error information
#[derive(Debug)]
pub struct DetailedBlobDeletionResult {
    pub content_hash: ContentHash,
    pub success: bool,
    pub file_deleted: bool,
    pub db_entry_deleted: bool,
    pub errors: Vec<String>,
}

/// Coordinator for deleting blobs from both storage and database
#[derive(Clone)]
pub struct BlobDeletionCoordinator {
    blob_repo: Arc<dyn BlobRepository>,
    blob_store: Arc<dyn BlobStore>,
}

impl BlobDeletionCoordinator {
    pub fn new(blob_repo: Arc<dyn BlobRepository>, blob_store: Arc<dyn BlobStore>) -> Self {
        Self {
            blob_repo,
            blob_store,
        }
    }

    /// Delete a single blob from both storage and database
    pub async fn delete_blob(
        &self,
        content_hash: ContentHash,
        storage_class: StorageClass,
    ) -> DetailedBlobDeletionResult {
        let mut errors = Vec::new();

        // Delete physical file
        let file_result = self.blob_store.delete(&content_hash, storage_class).await;
        let file_deleted = file_result.is_ok();

        if let Err(e) = file_result {
            let error_msg = format!("File deletion failed: {}", e);
            debug!("{} for blob {}", error_msg, content_hash);
            errors.push(error_msg);
        }

        // Delete database entry
        let db_result = self.blob_repo.delete(&content_hash).await;
        let db_entry_deleted = db_result.is_ok();

        if let Err(e) = db_result {
            let error_msg = format!("Database deletion failed: {}", e);
            warn!("{} for blob {}", error_msg, content_hash);
            errors.push(error_msg);
        }

        let success = db_entry_deleted; // Consider DB deletion as primary success metric

        DetailedBlobDeletionResult {
            content_hash: content_hash.clone(),
            success,
            file_deleted,
            db_entry_deleted,
            errors,
        }
    }

    /// Delete multiple blobs concurrently
    pub async fn delete_blobs(
        &self,
        blobs: Vec<(ContentHash, StorageClass)>,
    ) -> Result<Vec<DetailedBlobDeletionResult>, super::errors::BatchProcessingError> {
        use super::batch_processor::{BatchConfig, BatchProcessor};

        let config = BatchConfig::default();

        let processor = {
            let coordinator = self.clone();
            move |(content_hash, storage_class): (ContentHash, StorageClass)| {
                let coordinator = coordinator.clone();
                async move { coordinator.delete_blob(content_hash, storage_class).await }
            }
        };

        let batch_results = BatchProcessor::process_concurrent(blobs, &config, processor).await;

        let results: Vec<DetailedBlobDeletionResult> = batch_results
            .into_iter()
            .map(|batch_result| batch_result.result)
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{BlobRepository, BlobStore, RepositoryError, StorageError};
    use crate::domain::value_objects::{ContentHash, StorageClass};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockBlobRepository {
        deleted_hashes: Mutex<Vec<String>>,
        should_fail: bool,
    }

    impl MockBlobRepository {
        fn new(should_fail: bool) -> Self {
            Self {
                deleted_hashes: Mutex::new(Vec::new()),
                should_fail,
            }
        }
    }

    #[async_trait]
    impl BlobRepository for MockBlobRepository {
        async fn get_or_create(
            &self,
            _content_hash: &ContentHash,
            _storage_class: StorageClass,
            _size_bytes: u64,
        ) -> Result<crate::domain::entities::Blob, RepositoryError> {
            unimplemented!()
        }

        async fn increment_ref(&self, _content_hash: &ContentHash) -> Result<(), RepositoryError> {
            unimplemented!()
        }

        async fn decrement_ref(&self, _content_hash: &ContentHash) -> Result<i32, RepositoryError> {
            unimplemented!()
        }

        async fn find_orphaned(
            &self,
            _limit: i64,
        ) -> Result<Vec<crate::domain::entities::Blob>, RepositoryError> {
            unimplemented!()
        }

        async fn delete(&self, content_hash: &ContentHash) -> Result<(), RepositoryError> {
            if self.should_fail {
                return Err(RepositoryError::Database(sqlx::Error::RowNotFound));
            }
            self.deleted_hashes
                .lock()
                .unwrap()
                .push(content_hash.to_string());
            Ok(())
        }
    }

    struct MockBlobStore {
        deleted_files: Mutex<Vec<String>>,
        should_fail: bool,
    }

    impl MockBlobStore {
        fn new(should_fail: bool) -> Self {
            Self {
                deleted_files: Mutex::new(Vec::new()),
                should_fail,
            }
        }
    }

    #[async_trait]
    impl BlobStore for MockBlobStore {
        async fn write(
            &self,
            _reader: crate::application::ports::BlobReader,
            _storage_class: StorageClass,
        ) -> Result<(ContentHash, u64), StorageError> {
            unimplemented!()
        }

        async fn read(
            &self,
            _content_hash: &ContentHash,
            _storage_class: StorageClass,
        ) -> Result<crate::application::ports::BlobReader, StorageError> {
            unimplemented!()
        }

        async fn delete(
            &self,
            content_hash: &ContentHash,
            _storage_class: StorageClass,
        ) -> Result<(), StorageError> {
            if self.should_fail {
                return Err(StorageError::NotFound(content_hash.to_string()));
            }
            self.deleted_files
                .lock()
                .unwrap()
                .push(content_hash.to_string());
            Ok(())
        }

        async fn exists(
            &self,
            _content_hash: &ContentHash,
            _storage_class: StorageClass,
        ) -> Result<bool, StorageError> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_delete_blob_success() {
        let repo = Arc::new(MockBlobRepository::new(false));
        let store = Arc::new(MockBlobStore::new(false));
        let coordinator = BlobDeletionCoordinator::new(repo.clone(), store.clone());

        let content_hash = ContentHash::from_hex("testhash".repeat(16)).unwrap();
        let storage_class = StorageClass::Hot;

        let result = coordinator
            .delete_blob(content_hash.clone(), storage_class)
            .await;

        assert_eq!(result.content_hash, content_hash);
        assert!(result.file_deleted);
        assert!(result.db_entry_deleted);

        assert_eq!(store.deleted_files.lock().unwrap().len(), 1);
        assert_eq!(repo.deleted_hashes.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_delete_blob_file_failure() {
        let repo = Arc::new(MockBlobRepository::new(false));
        let store = Arc::new(MockBlobStore::new(true)); // fail file deletion
        let coordinator = BlobDeletionCoordinator::new(repo.clone(), store.clone());

        let content_hash = ContentHash::from_hex("testhash".repeat(16)).unwrap();
        let storage_class = StorageClass::Hot;

        let result = coordinator
            .delete_blob(content_hash.clone(), storage_class)
            .await;

        assert_eq!(result.content_hash, content_hash);
        assert!(!result.file_deleted);
        assert!(result.db_entry_deleted); // DB deletion should still succeed

        assert_eq!(store.deleted_files.lock().unwrap().len(), 0);
        assert_eq!(repo.deleted_hashes.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_delete_blob_db_failure() {
        let repo = Arc::new(MockBlobRepository::new(true)); // fail DB deletion
        let store = Arc::new(MockBlobStore::new(false));
        let coordinator = BlobDeletionCoordinator::new(repo.clone(), store.clone());

        let content_hash = ContentHash::from_hex("testhash".repeat(16)).unwrap();
        let storage_class = StorageClass::Hot;

        let result = coordinator
            .delete_blob(content_hash.clone(), storage_class)
            .await;

        assert_eq!(result.content_hash, content_hash);
        assert!(result.file_deleted);
        assert!(!result.db_entry_deleted); // DB deletion should fail

        assert_eq!(store.deleted_files.lock().unwrap().len(), 1);
        assert_eq!(repo.deleted_hashes.lock().unwrap().len(), 0);
    }
}
