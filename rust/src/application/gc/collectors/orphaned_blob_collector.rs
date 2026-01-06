use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info};

use super::{
    blob_deletion_coordinator::BlobDeletionCoordinator, collector::Collector, errors::GcResult,
};
use crate::application::ports::BlobRepository;

/// Collector for orphaned blobs (blobs with reference count = 0).
///
/// This collector identifies blobs that are no longer referenced by any objects
/// (reference count = 0) and removes them from both the blob store and the
/// database. This helps reclaim storage space and maintain data consistency.
///
/// The collector processes blobs in concurrent batches for improved performance,
/// but coordinates the deletion to ensure both the physical file and database
/// entry are removed atomically.
///
/// # Thread Safety
///
/// This collector is thread-safe and can be shared across multiple tasks.
///
/// # Examples
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use crate::application::gc::collectors::OrphanedBlobCollector;
///
/// let collector = OrphanedBlobCollector::new(
///     blob_repository,
///     blob_store,
///     100, // batch size
/// );
///
/// let cleaned_count = collector.collect().await?;
/// println!("Cleaned up {} orphaned blobs", cleaned_count);
/// ```
pub struct OrphanedBlobCollector {
    /// Repository for querying blob metadata.
    blob_repo: Arc<dyn BlobRepository>,
    /// Coordinator for handling blob deletion operations.
    deletion_coordinator: BlobDeletionCoordinator,
    /// Maximum number of blobs to process in a single collection cycle.
    batch_size: i64,
}

#[async_trait]
impl Collector for OrphanedBlobCollector {
    fn name(&self) -> &'static str {
        "orphaned_blob_collector"
    }

    async fn collect(&self) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        self.collect_internal()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

impl OrphanedBlobCollector {
    /// Creates a new orphaned blob collector.
    ///
    /// # Arguments
    ///
    /// * `blob_repo` - Repository for accessing blob metadata.
    /// * `blob_store` - Store for accessing physical blob files.
    /// * `batch_size` - Maximum number of blobs to process per collection cycle.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let collector = OrphanedBlobCollector::new(
    ///     Arc::new(blob_repository),
    ///     Arc::new(blob_store),
    ///     100
    /// );
    /// ```
    pub fn new(
        blob_repo: Arc<dyn BlobRepository>,
        blob_store: Arc<dyn crate::application::ports::BlobStore>,
        batch_size: i64,
    ) -> Self {
        let deletion_coordinator = BlobDeletionCoordinator::new(Arc::clone(&blob_repo), blob_store);

        Self {
            blob_repo,
            deletion_coordinator,
            batch_size,
        }
    }

    /// Collect and delete orphaned blobs.
    ///
    /// This method performs a complete collection cycle:
    /// 1. Queries the database for blobs with reference count = 0
    /// 2. Processes the blobs in concurrent batches for efficiency
    /// 3. Deletes each blob from both the physical store and database
    /// 4. Returns the count of successfully deleted blobs
    ///
    /// # Returns
    ///
    /// The number of orphaned blobs that were successfully deleted.
    /// Note that partial failures (e.g., file deletion fails but DB entry succeeds)
    /// are logged but still count as successful deletions from the database perspective.
    ///
    /// # Errors
    ///
    /// Returns an error if the initial query for orphaned blobs fails.
    /// Individual blob deletion failures are logged but don't fail the entire operation.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let deleted_count = collector.collect().await?;
    /// if deleted_count > 0 {
    ///     println!("Reclaimed space by deleting {} orphaned blobs", deleted_count);
    /// }
    /// ```
    async fn collect_internal(&self) -> GcResult<usize> {
        let orphaned_blobs = self
            .blob_repo
            .find_orphaned(self.batch_size)
            .await
            .map_err(|e| super::errors::GcError::QueryError { source: e.into() })?;

        let blob_count = orphaned_blobs.len();

        if blob_count == 0 {
            return Ok(0);
        }

        debug!("Found {} orphaned blobs to delete", blob_count);

        // Convert blobs to deletion tuples
        let blob_info: Vec<_> = orphaned_blobs
            .into_iter()
            .map(|blob| (blob.content_hash().clone(), blob.storage_class()))
            .collect();

        // Delete all blobs concurrently using the coordinator
        let deletion_results = self
            .deletion_coordinator
            .delete_blobs(blob_info)
            .await
            .map_err(|e| super::errors::GcError::DeletionError { source: e.into() })?;

        // Count successful deletions (based on DB deletion success)
        let total_deleted = deletion_results
            .iter()
            .filter(|result| result.db_entry_deleted)
            .count();

        info!("Cleaned up {} orphaned blobs", total_deleted);
        Ok(total_deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::gc::collectors::test_utils::{
        create_test_blob, MockBlobRepository, MockBlobStore,
    };
    use std::sync::Arc;

    #[tokio::test]
    async fn test_collect_no_orphaned_blobs() {
        let mock_repo = Arc::new(MockBlobRepository::new(vec![]));
        let mock_store = Arc::new(MockBlobStore::new());

        let collector = OrphanedBlobCollector::new(mock_repo, mock_store, 100);

        let result = collector.collect().await.unwrap();
        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn test_collect_orphaned_blobs() {
        let blob = create_test_blob("testhash", 0); // ref_count = 0 (orphaned)

        let mock_repo = Arc::new(MockBlobRepository::new(vec![blob]));
        let mock_store = Arc::new(MockBlobStore::new());

        let collector = OrphanedBlobCollector::new(mock_repo.clone(), mock_store.clone(), 100);

        let result = collector.collect().await.unwrap();
        assert_eq!(result, 1);

        // Verify deletions occurred
        assert_eq!(mock_repo.deleted_hashes.lock().unwrap().len(), 1);
        assert_eq!(mock_store.deleted_files.lock().unwrap().len(), 1);
    }
}
