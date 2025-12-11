use async_trait::async_trait;

use crate::domain::entities::Blob;
use crate::domain::value_objects::{ContentHash, StorageClass};

use super::RepositoryError;

/// Port for blob reference counting operations
#[async_trait]
pub trait BlobRepository: Send + Sync {
    /// Get or create blob entry
    async fn get_or_create(
        &self,
        content_hash: &ContentHash,
        storage_class: StorageClass,
        size_bytes: u64,
    ) -> Result<Blob, RepositoryError>;

    /// Increment reference count
    async fn increment_ref(&self, content_hash: &ContentHash) -> Result<(), RepositoryError>;

    /// Decrement reference count
    async fn decrement_ref(&self, content_hash: &ContentHash) -> Result<i32, RepositoryError>;

    /// Find blobs with zero references for GC
    async fn find_orphaned(&self, limit: i64) -> Result<Vec<Blob>, RepositoryError>;

    /// Delete blob entry (hard delete)
    async fn delete(&self, content_hash: &ContentHash) -> Result<(), RepositoryError>;
}
