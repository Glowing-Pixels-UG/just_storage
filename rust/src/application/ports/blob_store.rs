use async_trait::async_trait;
#[cfg(test)]
use mockall::{automock, predicate::*};
use std::pin::Pin;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::domain::value_objects::{ContentHash, StorageClass};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Blob not found: {0}")]
    NotFound(String),

    #[error("Hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Type alias for async reader
pub type BlobReader = Pin<Box<dyn AsyncRead + Send>>;

/// Type alias for async writer
pub type BlobWriter = Pin<Box<dyn AsyncWrite + Send>>;

/// Port for physical blob storage operations
#[cfg_attr(test, automock)]
#[async_trait]
pub trait BlobStore: Send + Sync {
    /// Write blob and return (content_hash, size_bytes)
    /// Reader is consumed and hash is computed during write
    async fn write(
        &self,
        reader: BlobReader,
        storage_class: StorageClass,
    ) -> Result<(ContentHash, u64), StorageError>;

    /// Read blob by content hash
    async fn read(
        &self,
        content_hash: &ContentHash,
        storage_class: StorageClass,
    ) -> Result<BlobReader, StorageError>;

    /// Delete blob file
    async fn delete(
        &self,
        content_hash: &ContentHash,
        storage_class: StorageClass,
    ) -> Result<(), StorageError>;

    /// Check if blob exists
    async fn exists(
        &self,
        content_hash: &ContentHash,
        storage_class: StorageClass,
    ) -> Result<bool, StorageError>;
}
