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
        if self.ref_count > 0 {
            self.ref_count -= 1;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::ContentHash;
    use std::str::FromStr;

    fn create_test_blob() -> Blob {
        let content_hash = ContentHash::from_str(&"a".repeat(64)).unwrap();
        Blob::new(content_hash, StorageClass::Hot, 123)
    }

    #[test]
    fn test_blob_new() {
        let blob = create_test_blob();
        assert_eq!(blob.ref_count(), 1);
        assert!(!blob.can_gc());
    }

    #[test]
    fn test_blob_increment_ref() {
        let mut blob = create_test_blob();
        blob.increment_ref();
        assert_eq!(blob.ref_count(), 2);
    }

    #[test]
    fn test_blob_decrement_ref() {
        let mut blob = create_test_blob();
        blob.increment_ref();
        blob.decrement_ref();
        assert_eq!(blob.ref_count(), 1);
    }

    #[test]
    fn test_blob_decrement_ref_and_can_gc() {
        let mut blob = create_test_blob();
        assert_eq!(blob.ref_count(), 1);
        assert!(!blob.can_gc());

        blob.decrement_ref();
        assert_eq!(blob.ref_count(), 0);
        assert!(blob.can_gc());

        // Test saturating subtraction
        blob.decrement_ref();
        assert_eq!(blob.ref_count(), 0);
        assert!(blob.can_gc());
    }
}
