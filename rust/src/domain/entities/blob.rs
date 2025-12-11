use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{ContentHash, StorageClass};

/// Blob entity - represents physical storage with ref counting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blob {
    content_hash: ContentHash,
    storage_class: StorageClass,
    size_bytes: u64,
    ref_count: i32,
    created_at: DateTime<Utc>,
}

impl Blob {
    /// Create new blob with ref_count = 1
    pub fn new(content_hash: ContentHash, storage_class: StorageClass, size_bytes: u64) -> Self {
        Self {
            content_hash,
            storage_class,
            size_bytes,
            ref_count: 1,
            created_at: Utc::now(),
        }
    }

    /// Reconstruct from storage
    pub fn reconstruct(
        content_hash: ContentHash,
        storage_class: StorageClass,
        size_bytes: u64,
        ref_count: i32,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            content_hash,
            storage_class,
            size_bytes,
            ref_count,
            created_at,
        }
    }

    /// Increment reference count
    pub fn increment_ref(&mut self) {
        self.ref_count += 1;
    }

    /// Decrement reference count
    pub fn decrement_ref(&mut self) {
        self.ref_count = self.ref_count.saturating_sub(1);
    }

    /// Check if blob can be garbage collected
    pub fn can_gc(&self) -> bool {
        self.ref_count == 0
    }

    // Getters
    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }

    pub fn storage_class(&self) -> StorageClass {
        self.storage_class
    }

    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    pub fn ref_count(&self) -> i32 {
        self.ref_count
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
