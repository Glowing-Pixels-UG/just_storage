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

        blob.decrement_ref();
        assert_eq!(blob.ref_count(), 0);
        assert!(blob.can_gc());
    }

    #[test]
    fn test_blob_creation_edge_cases() {
        let content_hash = ContentHash::from_str(&"b".repeat(64)).unwrap();

        // Test with zero size
        let blob_zero = Blob::new(content_hash.clone(), StorageClass::Hot, 0);
        assert_eq!(blob_zero.size_bytes(), 0);
        assert_eq!(blob_zero.ref_count(), 1);

        // Test with maximum size
        let blob_max = Blob::new(content_hash.clone(), StorageClass::Cold, u64::MAX);
        assert_eq!(blob_max.size_bytes(), u64::MAX);
        assert_eq!(blob_max.ref_count(), 1);

        // Test with both storage classes
        let blob_hot = Blob::new(content_hash.clone(), StorageClass::Hot, 1024);
        let blob_cold = Blob::new(content_hash, StorageClass::Cold, 1024);

        assert_eq!(blob_hot.storage_class(), StorageClass::Hot);
        assert_eq!(blob_cold.storage_class(), StorageClass::Cold);
    }

    #[test]
    fn test_blob_ref_count_operations_comprehensive() {
        let mut blob = create_test_blob();

        // Initial state
        assert_eq!(blob.ref_count(), 1);
        assert!(!blob.can_gc());

        // Increment multiple times
        blob.increment_ref();
        blob.increment_ref();
        blob.increment_ref();
        assert_eq!(blob.ref_count(), 4);
        assert!(!blob.can_gc());

        // Decrement back to 1
        blob.decrement_ref();
        blob.decrement_ref();
        blob.decrement_ref();
        assert_eq!(blob.ref_count(), 1);
        assert!(!blob.can_gc());

        // Decrement to 0
        blob.decrement_ref();
        assert_eq!(blob.ref_count(), 0);
        assert!(blob.can_gc());

        // Further decrements should not go below 0
        blob.decrement_ref();
        blob.decrement_ref();
        assert_eq!(blob.ref_count(), 0);
        assert!(blob.can_gc());
    }

    #[test]
    fn test_blob_gc_eligibility() {
        let mut blob = create_test_blob();

        // Fresh blob cannot be GC'd
        assert!(!blob.can_gc());

        // Blob with refs cannot be GC'd
        blob.increment_ref();
        assert!(!blob.can_gc());
        blob.decrement_ref();
        assert!(!blob.can_gc());

        // Only when ref_count reaches 0 can it be GC'd
        blob.decrement_ref();
        assert!(blob.can_gc());

        // Even with zero refs, it stays GC-eligible
        assert!(blob.can_gc());
    }

    #[test]
    fn test_blob_equality_and_hashing() {
        let content_hash1 = ContentHash::from_str(&"c".repeat(64)).unwrap();
        let content_hash2 = ContentHash::from_str(&"d".repeat(64)).unwrap();

        let blob1a = Blob::new(content_hash1.clone(), StorageClass::Hot, 100);
        let blob1b = Blob::new(content_hash1.clone(), StorageClass::Hot, 100);
        let blob2 = Blob::new(content_hash2, StorageClass::Hot, 100);

        // Blobs with same content hash should be equal
        assert_eq!(blob1a, blob1b);
        // Blobs with different content hashes should not be equal
        assert_ne!(blob1a, blob2);
    }

    #[test]
    fn test_blob_clone() {
        let blob1 = create_test_blob();
        let blob2 = blob1.clone();

        assert_eq!(blob1, blob2);
        assert_eq!(blob1.content_hash(), blob2.content_hash());
        assert_eq!(blob1.storage_class(), blob2.storage_class());
        assert_eq!(blob1.size_bytes(), blob2.size_bytes());
        assert_eq!(blob1.ref_count(), blob2.ref_count());
    }

    #[test]
    fn test_blob_timestamps() {
        let blob = create_test_blob();

        // Created at should be set
        let created_at = blob.created_at();
        let now = Utc::now();

        // Should be within the last minute
        assert!(created_at > now - chrono::Duration::minutes(1));
        assert!(created_at <= now);
    }

    #[test]
    fn test_blob_with_different_sizes() {
        let content_hash = ContentHash::from_str(&"e".repeat(64)).unwrap();

        // Test various sizes
        let sizes = vec![1, 1024, 1024 * 1024, 1024 * 1024 * 1024];

        for size in sizes {
            let blob = Blob::new(content_hash.clone(), StorageClass::Hot, size);
            assert_eq!(blob.size_bytes(), size);
        }
    }

    #[test]
    fn test_blob_ref_count_overflow_protection() {
        let mut blob = create_test_blob();

        // Increment many times - should handle large ref counts
        for _ in 0..1000 {
            blob.increment_ref();
        }
        assert_eq!(blob.ref_count(), 1001);

        // Decrement back down
        for _ in 0..1000 {
            blob.decrement_ref();
        }
        assert_eq!(blob.ref_count(), 1);

        // One more decrement to 0
        blob.decrement_ref();
        assert_eq!(blob.ref_count(), 0);
        assert!(blob.can_gc());
    }
}
