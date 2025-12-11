use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

use crate::application::ports::{BlobRepository, BlobStore};

/// Garbage collector for orphaned blobs
pub struct GarbageCollector {
    blob_repo: Arc<dyn BlobRepository>,
    blob_store: Arc<dyn BlobStore>,
    interval: Duration,
    batch_size: i64,
}

impl GarbageCollector {
    pub fn new(
        blob_repo: Arc<dyn BlobRepository>,
        blob_store: Arc<dyn BlobStore>,
        interval: Duration,
        batch_size: i64,
    ) -> Self {
        Self {
            blob_repo,
            blob_store,
            interval,
            batch_size,
        }
    }

    /// Run garbage collection loop
    pub async fn run(self: Arc<Self>) {
        info!(
            "Starting garbage collector with interval: {:?}",
            self.interval
        );

        let mut interval = time::interval(self.interval);

        loop {
            interval.tick().await;

            match self.collect_once().await {
                Ok(count) => {
                    if count > 0 {
                        info!("Garbage collection completed: {} blobs deleted", count);
                    }
                }
                Err(e) => {
                    error!("Garbage collection failed: {}", e);
                }
            }
        }
    }

    /// Run one GC cycle
    async fn collect_once(&self) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        // Find orphaned blobs (ref_count = 0)
        let orphaned_blobs = self.blob_repo.find_orphaned(self.batch_size).await?;

        let count = orphaned_blobs.len();
        if count == 0 {
            return Ok(0);
        }

        info!("Found {} orphaned blobs to delete", count);

        for blob in orphaned_blobs {
            let content_hash = blob.content_hash();
            let storage_class = blob.storage_class();

            // Delete physical file
            match self.blob_store.delete(content_hash, storage_class).await {
                Ok(_) => {
                    info!("Deleted blob file: {}", content_hash);
                }
                Err(e) => {
                    warn!("Failed to delete blob file {}: {}", content_hash, e);
                    // Continue anyway - DB entry will be deleted
                }
            }

            // Delete DB entry
            if let Err(e) = self.blob_repo.delete(content_hash).await {
                error!("Failed to delete blob DB entry {}: {}", content_hash, e);
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{BlobRepository, BlobStore, RepositoryError, StorageError};
    use crate::domain::entities::Blob;
    use crate::domain::value_objects::{ContentHash, StorageClass};
    use async_trait::async_trait;

    use std::collections::HashMap;
    use tokio::sync::Mutex;

    struct MockBlobRepository {
        blobs: Mutex<Vec<Blob>>,
    }

    impl MockBlobRepository {
        fn new(blobs: Vec<Blob>) -> Self {
            Self {
                blobs: Mutex::new(blobs),
            }
        }
    }

    #[async_trait]
    impl BlobRepository for MockBlobRepository {
        async fn get_or_create(
            &self,
            content_hash: &ContentHash,
            storage_class: StorageClass,
            size_bytes: u64,
        ) -> Result<Blob, RepositoryError> {
            let mut blobs = self.blobs.lock().await;

            // Try find existing
            if let Some(b) = blobs
                .iter_mut()
                .find(|b| b.content_hash() == content_hash && b.storage_class() == storage_class)
            {
                b.increment_ref();
                return Ok(b.clone());
            }

            // Create new
            let b = Blob::new(content_hash.clone(), storage_class, size_bytes);
            blobs.push(b.clone());
            Ok(b)
        }

        async fn increment_ref(&self, content_hash: &ContentHash) -> Result<(), RepositoryError> {
            let mut blobs = self.blobs.lock().await;
            if let Some(b) = blobs.iter_mut().find(|b| b.content_hash() == content_hash) {
                b.increment_ref();
                Ok(())
            } else {
                Err(RepositoryError::NotFound(content_hash.to_string()))
            }
        }

        async fn decrement_ref(&self, content_hash: &ContentHash) -> Result<i32, RepositoryError> {
            let mut blobs = self.blobs.lock().await;
            if let Some(b) = blobs.iter_mut().find(|b| b.content_hash() == content_hash) {
                b.decrement_ref();
                Ok(b.ref_count())
            } else {
                Err(RepositoryError::NotFound(content_hash.to_string()))
            }
        }

        async fn find_orphaned(&self, limit: i64) -> Result<Vec<Blob>, RepositoryError> {
            let blobs = self.blobs.lock().await;
            let orphaned: Vec<Blob> = blobs
                .iter()
                .filter(|b| b.can_gc())
                .take(limit as usize)
                .cloned()
                .collect();
            Ok(orphaned)
        }

        async fn delete(&self, content_hash: &ContentHash) -> Result<(), RepositoryError> {
            let mut blobs = self.blobs.lock().await;
            blobs.retain(|b| b.content_hash() != content_hash);
            Ok(())
        }
    }

    struct MockBlobStore {
        // map from (content_hash, storage_class) -> data
        map: Mutex<HashMap<(String, StorageClass), Vec<u8>>>,
        // set of content hashes for which delete fails
        fail_on_delete: Mutex<std::collections::HashSet<String>>,
    }

    impl MockBlobStore {
        fn new() -> Self {
            Self {
                map: Mutex::new(HashMap::new()),
                fail_on_delete: Mutex::new(std::collections::HashSet::new()),
            }
        }

        async fn mark_delete_fail(&self, content_hash: &ContentHash) {
            let mut s = self.fail_on_delete.lock().await;
            s.insert(content_hash.as_hex().to_string());
        }
    }

    #[async_trait]
    impl BlobStore for MockBlobStore {
        async fn write(
            &self,
            mut reader: crate::application::ports::BlobReader,
            storage_class: StorageClass,
        ) -> Result<(ContentHash, u64), StorageError> {
            use sha2::{Digest, Sha256};
            use tokio::io::AsyncReadExt;

            let mut buf = Vec::new();
            let n = reader
                .read_to_end(&mut buf)
                .await
                .map_err(StorageError::Io)?;

            // compute sha256
            let mut hasher = Sha256::new();
            hasher.update(&buf);
            let hash_hex = hex::encode(hasher.finalize());
            let content_hash = ContentHash::from_hex(hash_hex)
                .map_err(|e| StorageError::Internal(e.to_string()))?;

            let mut map = self.map.lock().await;
            map.insert((content_hash.as_hex().to_string(), storage_class), buf);

            Ok((content_hash, n as u64))
        }

        async fn read(
            &self,
            content_hash: &ContentHash,
            storage_class: StorageClass,
        ) -> Result<crate::application::ports::BlobReader, StorageError> {
            use bytes::Bytes;
            use futures_util::stream::once;
            use tokio::io::BufReader;
            use tokio_util::io::StreamReader;

            let map = self.map.lock().await;
            if let Some(data) = map.get(&(content_hash.as_hex().to_string(), storage_class)) {
                // clone data out of the map so we can drop the lock before creating reader
                let data_vec = data.clone();
                drop(map);
                // stream yields Bytes which implements Buf
                let stream = once(async { Ok::<_, std::io::Error>(Bytes::from(data_vec)) });
                let reader = StreamReader::new(stream);
                Ok(Box::pin(BufReader::new(reader)))
            } else {
                Err(StorageError::NotFound(content_hash.to_string()))
            }
        }

        async fn delete(
            &self,
            content_hash: &ContentHash,
            storage_class: StorageClass,
        ) -> Result<(), StorageError> {
            let should_fail = {
                let s = self.fail_on_delete.lock().await;
                s.contains(content_hash.as_hex())
            };

            if should_fail {
                return Err(StorageError::Internal(
                    "simulated delete failure".to_string(),
                ));
            }

            let mut map = self.map.lock().await;
            map.remove(&(content_hash.as_hex().to_string(), storage_class));
            Ok(())
        }

        async fn exists(
            &self,
            content_hash: &ContentHash,
            storage_class: StorageClass,
        ) -> Result<bool, StorageError> {
            let map = self.map.lock().await;
            Ok(map.contains_key(&(content_hash.as_hex().to_string(), storage_class)))
        }
    }

    #[tokio::test]
    async fn test_gc_empty() {
        let repo = Arc::new(MockBlobRepository::new(vec![]));
        let store = Arc::new(MockBlobStore::new());

        let gc = GarbageCollector::new(repo, store, Duration::from_secs(60), 100);

        let result = gc.collect_once().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_gc_deletes_orphan() {
        use crate::domain::value_objects::ContentHash;
        use chrono::Utc;

        let content_hash = ContentHash::from_hex("0".repeat(64)).unwrap();

        // Blob with ref_count = 0 should be deleted
        let blob = Blob::reconstruct(content_hash.clone(), StorageClass::Hot, 42, 0, Utc::now());

        let repo = Arc::new(MockBlobRepository::new(vec![blob]));
        let store = Arc::new(MockBlobStore::new());

        // insert data into store for the content hash
        {
            let mut map = store.map.lock().await;
            map.insert(
                (content_hash.as_hex().to_string(), StorageClass::Hot),
                vec![1u8, 2, 3],
            );
        }

        let gc = GarbageCollector::new(repo.clone(), store.clone(), Duration::from_secs(60), 100);

        let result = gc.collect_once().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // store should no longer have the blob
        let exists = store
            .exists(&content_hash, StorageClass::Hot)
            .await
            .unwrap();
        assert!(!exists);

        // repo should not return the orphan anymore
        let orphaned = repo.find_orphaned(100).await.unwrap();
        assert!(orphaned.is_empty());
    }

    #[tokio::test]
    async fn test_gc_delete_failure_but_db_deleted() {
        use crate::domain::value_objects::ContentHash;
        use chrono::Utc;

        let content_hash = ContentHash::from_hex("1".repeat(64)).unwrap();
        let blob = Blob::reconstruct(content_hash.clone(), StorageClass::Hot, 42, 0, Utc::now());

        let repo = Arc::new(MockBlobRepository::new(vec![blob]));
        let store = Arc::new(MockBlobStore::new());

        // put data into store and mark delete to fail
        {
            let mut map = store.map.lock().await;
            map.insert(
                (content_hash.as_hex().to_string(), StorageClass::Hot),
                vec![4u8, 5, 6],
            );
        }
        store.mark_delete_fail(&content_hash).await;

        let gc = GarbageCollector::new(repo.clone(), store.clone(), Duration::from_secs(60), 100);
        let result = gc.collect_once().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);

        // store should still contain the blob because delete failed
        let exists = store
            .exists(&content_hash, StorageClass::Hot)
            .await
            .unwrap();
        assert!(exists);

        // repo should not have orphan anymore (deleted despite store deletion failure)
        let orphaned = repo.find_orphaned(100).await.unwrap();
        assert!(orphaned.is_empty());
    }

    #[tokio::test]
    async fn test_gc_respects_batch_size_limit() {
        use crate::domain::value_objects::ContentHash;
        use chrono::Utc;

        let content_hash1 = ContentHash::from_hex("2".repeat(64)).unwrap();
        let content_hash2 = ContentHash::from_hex("3".repeat(64)).unwrap();

        let blob1 = Blob::reconstruct(content_hash1.clone(), StorageClass::Hot, 1, 0, Utc::now());
        let blob2 = Blob::reconstruct(content_hash2.clone(), StorageClass::Hot, 2, 0, Utc::now());

        let repo = Arc::new(MockBlobRepository::new(vec![blob1, blob2]));
        let store = Arc::new(MockBlobStore::new());

        {
            let mut map = store.map.lock().await;
            map.insert(
                (content_hash1.as_hex().to_string(), StorageClass::Hot),
                vec![1u8],
            );
            map.insert(
                (content_hash2.as_hex().to_string(), StorageClass::Hot),
                vec![2u8],
            );
        }

        let gc = GarbageCollector::new(repo.clone(), store.clone(), Duration::from_secs(60), 1);
        let count = gc.collect_once().await.unwrap();
        assert_eq!(count, 1);

        // one orphan should remain
        let orphaned = repo.find_orphaned(10).await.unwrap();
        assert_eq!(orphaned.len(), 1);

        // Run again to delete second
        let count2 = gc.collect_once().await.unwrap();
        assert_eq!(count2, 1);

        let orphaned2 = repo.find_orphaned(10).await.unwrap();
        assert!(orphaned2.is_empty());
    }
}
