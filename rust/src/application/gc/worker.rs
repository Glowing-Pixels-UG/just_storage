use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info};

use crate::application::gc::collectors::{
    errors::GcResult as CollectorResult, Collector, OrphanedBlobCollector, StuckUploadCollector,
};
use crate::application::gc::config::GcConfig;
use crate::application::gc::results::GcResult;
use crate::application::gc::scheduler::TaskScheduler;
use crate::application::ports::{BlobRepository, BlobStore, ObjectRepository};

/// Garbage collector for orphaned blobs and stuck uploads.
///
/// This is the main orchestrator for all garbage collection operations in the system.
/// It manages multiple collectors and coordinates their execution according to
/// configured schedules and intervals.
///
/// The collector supports:
/// - **Orphaned blob cleanup**: Removes blobs with zero references
/// - **Stuck upload cleanup**: Removes incomplete uploads that have been stuck too long
/// - **Extensible architecture**: New collectors can be easily added
/// - **Periodic execution**: Can run continuously with configurable intervals
/// - **Conditional execution**: Some collectors only run periodically for efficiency
///
/// # Thread Safety
///
/// The garbage collector is thread-safe and designed to be run from multiple
/// concurrent tasks. All state is protected with appropriate synchronization primitives.
///
/// # Examples
///
/// Basic usage with just orphaned blob collection:
/// ```rust,ignore
/// use std::time::Duration;
/// use crate::application::gc::GarbageCollector;
///
/// let gc = GarbageCollector::new(
///     blob_repository,
///     blob_store,
///     Duration::from_secs(300), // 5 minutes
///     100, // batch size
/// );
///
/// // Run one collection cycle
/// let result = gc.collect_once().await?;
/// println!("Cleaned up {} items", result.total_deleted);
///
/// // Or run continuously
/// gc.run().await;
/// ```
///
/// Advanced usage with stuck upload cleanup:
/// ```rust,ignore
/// let gc = GarbageCollector::with_object_repo(
///     blob_repository,
///     blob_store,
///     Some(object_repository),
///     Duration::from_secs(300), // main interval
///     100, // batch size
///     24,  // stuck upload age threshold in hours
/// );
/// ```
pub struct GarbageCollector {
    /// The collection of all registered garbage collectors.
    collectors: Vec<Box<dyn Collector + Send + Sync>>,
    /// Configuration for collection intervals and parameters.
    config: GcConfig,
    /// Optional scheduler for stuck upload cleanup (runs less frequently).
    stuck_upload_scheduler: Option<TaskScheduler>,
}

impl GarbageCollector {
    /// Creates a new garbage collector for orphaned blobs only.
    ///
    /// This constructor creates a collector that only handles orphaned blob cleanup.
    /// For stuck upload cleanup, use `with_object_repo` instead.
    ///
    /// # Arguments
    ///
    /// * `blob_repo` - Repository for accessing blob metadata.
    /// * `blob_store` - Store for accessing physical blob files.
    /// * `interval` - How often to run the collection cycle.
    /// * `batch_size` - Maximum number of blobs to process per collection cycle.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// let gc = GarbageCollector::new(
    ///     Arc::new(blob_repository),
    ///     Arc::new(blob_store),
    ///     Duration::from_secs(300), // 5 minutes
    ///     100
    /// );
    /// ```
    pub fn new(
        blob_repo: Arc<dyn BlobRepository>,
        blob_store: Arc<dyn BlobStore>,
        interval: Duration,
        batch_size: i64,
    ) -> Self {
        let orphaned_collector =
            OrphanedBlobCollector::new(Arc::clone(&blob_repo), Arc::clone(&blob_store), batch_size);

        let collectors: Vec<Box<dyn Collector + Send + Sync>> = vec![Box::new(orphaned_collector)];

        Self {
            collectors,
            config: GcConfig::new(interval, batch_size, 1),
            stuck_upload_scheduler: None, // No stuck upload collector
        }
    }

    pub fn with_object_repo(
        blob_repo: Arc<dyn BlobRepository>,
        blob_store: Arc<dyn BlobStore>,
        object_repo: Option<Arc<dyn ObjectRepository>>,
        interval: Duration,
        batch_size: i64,
        stuck_upload_age_hours: i64,
    ) -> Self {
        let config = GcConfig::new(interval, batch_size, stuck_upload_age_hours);

        let mut collectors: Vec<Box<dyn Collector + Send + Sync>> = Vec::new();

        // Add orphaned blob collector
        let orphaned_collector =
            OrphanedBlobCollector::new(Arc::clone(&blob_repo), Arc::clone(&blob_store), batch_size);
        collectors.push(Box::new(orphaned_collector));

        // Add stuck upload collector if object repo is provided
        let stuck_upload_scheduler = if let Some(obj_repo) = object_repo {
            let stuck_upload_collector =
                StuckUploadCollector::new(obj_repo, stuck_upload_age_hours);
            collectors.push(Box::new(stuck_upload_collector));
            Some(TaskScheduler::new(config.stuck_upload_cleanup_interval()))
        } else {
            None
        };

        Self {
            collectors,
            config,
            stuck_upload_scheduler,
        }
    }

    pub fn with_config(
        blob_repo: Arc<dyn BlobRepository>,
        blob_store: Arc<dyn BlobStore>,
        object_repo: Option<Arc<dyn ObjectRepository>>,
        config: GcConfig,
    ) -> Self {
        let mut collectors: Vec<Box<dyn Collector + Send + Sync>> = Vec::new();

        // Add orphaned blob collector
        let orphaned_collector = OrphanedBlobCollector::new(
            Arc::clone(&blob_repo),
            Arc::clone(&blob_store),
            config.batch_size,
        );
        collectors.push(Box::new(orphaned_collector));

        // Add stuck upload collector if object repo is provided
        let stuck_upload_scheduler = if let Some(obj_repo) = object_repo {
            let stuck_upload_collector =
                StuckUploadCollector::new(obj_repo, config.stuck_upload_age_hours);
            collectors.push(Box::new(stuck_upload_collector));
            Some(TaskScheduler::new(config.stuck_upload_cleanup_interval()))
        } else {
            None
        };

        Self {
            collectors,
            config,
            stuck_upload_scheduler,
        }
    }

    /// Run garbage collection loop
    pub async fn run(self: Arc<Self>) {
        info!(
            "Starting garbage collector with interval: {:?}",
            self.config.interval
        );

        let mut interval = time::interval(self.config.interval);

        loop {
            interval.tick().await;

            match self.collect_once().await {
                Ok(result) => {
                    if result.has_deletions() {
                        info!(
                            "Garbage collection completed: {} total deleted ({} orphaned blobs, {} stuck uploads)",
                            result.total_deleted,
                            result.orphaned_blobs_deleted,
                            result.stuck_uploads_deleted
                        );
                    }

                    if !result.is_success() {
                        for error in &result.errors {
                            error!("GC error: {}", error);
                        }
                    }
                }
                Err(e) => {
                    error!("Garbage collection cycle failed: {}", e);
                }
            }
        }
    }

    /// Runs one complete garbage collection cycle.
    ///
    /// This method executes all registered collectors in sequence, collecting
    /// their results into a comprehensive summary. Some collectors may only run
    /// conditionally based on their configured schedules.
    ///
    /// # Returns
    ///
    /// A `GcResult` containing counts of deleted items and any errors that occurred.
    /// The operation succeeds even if individual collectors fail - errors are collected
    /// and returned in the result.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let result = gc.collect_once().await?;
    ///
    /// if result.is_success() {
    ///     println!("GC completed successfully");
    ///     println!("Orphaned blobs deleted: {}", result.orphaned_blobs_deleted);
    ///     println!("Stuck uploads deleted: {}", result.stuck_uploads_deleted);
    /// } else {
    ///     for error in &result.errors {
    ///         eprintln!("GC error: {}", error);
    ///     }
    /// }
    /// ```
    ///
    /// # Thread Safety
    ///
    /// This method is safe to call concurrently from multiple tasks.
    pub async fn collect_once(&self) -> CollectorResult<GcResult> {
        let mut result = GcResult::default();

        for collector in &self.collectors {
            let collector_name = collector.name();

            let should_run = match collector_name {
                "stuck_upload_collector" => self.should_run_stuck_upload_cleanup(),
                _ => true,
            };

            if should_run {
                match collector.collect().await {
                    Ok(count) => match collector_name {
                        "orphaned_blob_collector" => result.orphaned_blobs_deleted = count,
                        "stuck_upload_collector" => result.stuck_uploads_deleted = count,
                        _ => result.total_deleted += count,
                    },
                    Err(e) => {
                        result
                            .errors
                            .push(format!("{} collection failed: {}", collector_name, e));
                    }
                }
            }
        }

        result.total_deleted = result.orphaned_blobs_deleted + result.stuck_uploads_deleted;
        Ok(result)
    }

    /// Check if stuck upload cleanup should run
    fn should_run_stuck_upload_cleanup(&self) -> bool {
        self.stuck_upload_scheduler
            .as_ref()
            .map(|scheduler| scheduler.should_run())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::gc::config::GcConfig;
    use crate::application::ports::{
        BlobRepository, BlobStore, ObjectRepository, RepositoryError, StorageError,
    };
    use crate::domain::entities::Blob;
    use crate::domain::value_objects::{ContentHash, StorageClass};
    use async_trait::async_trait;
    struct MockBlobRepository {
        blobs: std::sync::Mutex<Vec<Blob>>,
    }

    impl MockBlobRepository {
        fn new(blobs: Vec<Blob>) -> Self {
            Self {
                blobs: std::sync::Mutex::new(blobs),
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
        ) -> Result<Blob, RepositoryError> {
            unimplemented!("Not needed for GC worker tests")
        }

        async fn increment_ref(&self, _content_hash: &ContentHash) -> Result<(), RepositoryError> {
            unimplemented!("Not needed for GC worker tests")
        }

        async fn decrement_ref(&self, _content_hash: &ContentHash) -> Result<i32, RepositoryError> {
            unimplemented!("Not needed for GC worker tests")
        }

        async fn find_orphaned(&self, limit: i64) -> Result<Vec<Blob>, RepositoryError> {
            let blobs = self.blobs.lock().unwrap();
            let orphaned: Vec<Blob> = blobs
                .iter()
                .filter(|b| b.ref_count() == 0)
                .take(limit as usize)
                .cloned()
                .collect();
            Ok(orphaned)
        }

        async fn delete(&self, content_hash: &ContentHash) -> Result<(), RepositoryError> {
            let mut blobs = self.blobs.lock().unwrap();
            blobs.retain(|b| b.content_hash() != content_hash);
            Ok(())
        }
    }

    struct MockBlobStore;

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
            _content_hash: &ContentHash,
            _storage_class: StorageClass,
        ) -> Result<(), StorageError> {
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

    struct MockObjectRepository;

    #[async_trait]
    impl ObjectRepository for MockObjectRepository {
        async fn cleanup_stuck_uploads(&self, _age_hours: i64) -> Result<usize, RepositoryError> {
            Ok(0)
        }

        async fn find_by_id(
            &self,
            _id: &crate::domain::value_objects::ObjectId,
        ) -> Result<Option<crate::domain::entities::Object>, RepositoryError> {
            unimplemented!()
        }


        async fn save(
            &self,
            _object: &crate::domain::entities::Object,
        ) -> Result<(), RepositoryError> {
            unimplemented!()
        }

        async fn delete(
            &self,
            _id: &crate::domain::value_objects::ObjectId,
        ) -> Result<(), RepositoryError> {
            unimplemented!()
        }

        async fn find_by_key(
            &self,
            _namespace: &crate::domain::value_objects::Namespace,
            _tenant_id: &crate::domain::value_objects::TenantId,
            _key: &str,
        ) -> Result<Option<crate::domain::entities::Object>, RepositoryError> {
            unimplemented!()
        }

        async fn list(
            &self,
            _namespace: &crate::domain::value_objects::Namespace,
            _tenant_id: &crate::domain::value_objects::TenantId,
            _limit: i64,
            _offset: i64,
        ) -> Result<Vec<crate::domain::entities::Object>, RepositoryError> {
            unimplemented!()
        }

        async fn search(&self, _request: &crate::application::dto::SearchRequest) -> Result<Vec<crate::domain::entities::Object>, RepositoryError> {
            unimplemented!()
        }

        async fn text_search(&self, _request: &crate::application::dto::TextSearchRequest) -> Result<Vec<crate::domain::entities::Object>, RepositoryError> {
            unimplemented!()
        }

        async fn find_stuck_writing_objects(
            &self,
            _age_hours: i64,
            _limit: i64,
        ) -> Result<Vec<crate::domain::value_objects::ObjectId>, RepositoryError> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_gc_collect_once_no_orphaned() {
        let repo = Arc::new(MockBlobRepository::new(vec![]));
        let store = Arc::new(MockBlobStore);

        let gc = GarbageCollector::new(repo, store, Duration::from_secs(60), 100);

        let result = gc.collect_once().await.unwrap();
        assert_eq!(result.total_deleted, 0);
        assert_eq!(result.orphaned_blobs_deleted, 0);
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_gc_collect_once_with_orphaned() {
        let content_hash = ContentHash::from_hex("testhash".repeat(16)).unwrap();
        let blob = Blob::new(content_hash, StorageClass::Hot, 42);

        let repo = Arc::new(MockBlobRepository::new(vec![blob]));
        let store = Arc::new(MockBlobStore);

        let gc = GarbageCollector::new(repo, store, Duration::from_secs(60), 100);

        let result = gc.collect_once().await.unwrap();
        assert_eq!(result.total_deleted, 1);
        assert_eq!(result.orphaned_blobs_deleted, 1);
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_gc_with_config() {
        let repo = Arc::new(MockBlobRepository::new(vec![]));
        let store = Arc::new(MockBlobStore);
        let object_repo = Arc::new(MockObjectRepository);

        let config = GcConfig::new(Duration::from_secs(300), 50, 2);

        let gc = GarbageCollector::with_config(repo, store, Some(object_repo), config);

        let result = gc.collect_once().await.unwrap();
        assert_eq!(result.total_deleted, 0);
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_gc_result_methods() {
        let result = GcResult {
            total_deleted: 5,
            orphaned_blobs_deleted: 3,
            stuck_uploads_deleted: 2,
            errors: vec![],
        };

        assert!(result.is_success());
        assert!(result.has_deletions());
        assert_eq!(result.total_deleted, 5);
    }

    #[tokio::test]
    async fn test_gc_result_with_errors() {
        let result = GcResult {
            total_deleted: 2,
            orphaned_blobs_deleted: 2,
            stuck_uploads_deleted: 0,
            errors: vec!["Test error".to_string()],
        };

        assert!(!result.is_success());
        assert!(result.has_deletions());
    }

    #[tokio::test]
    async fn test_should_run_stuck_upload_cleanup() {
        let repo = Arc::new(MockBlobRepository::new(vec![]));
        let store = Arc::new(MockBlobStore);
        let object_repo = Arc::new(MockObjectRepository);

        let config = GcConfig::new(Duration::from_secs(60), 100, 1);
        let gc = GarbageCollector::with_config(repo, store, Some(object_repo), config);

        // Initially should run (first time)
        assert!(gc.should_run_stuck_upload_cleanup());

        // Immediately after should not run
        assert!(!gc.should_run_stuck_upload_cleanup());
    }
}
