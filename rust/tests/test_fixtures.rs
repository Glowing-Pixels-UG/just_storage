//! Shared test fixtures and utilities for all test types
//!
//! This module provides common test setup patterns to reduce duplication
//! and make tests more maintainable.

use uuid::Uuid;


// Re-export the shared TestEnvironment from `tests/common`
// This eliminates the duplicate implementation and ensures all tests
// use a single source of truth for environment setup.
pub use crate::common::TestEnvironment;

// Re-export shared DB & storage helpers from `tests/common::database`
pub use crate::common::database::{cleanup_test_data, setup_test_database, setup_test_storage};

/// Test data factories
pub mod factories {
    use super::*;
    use just_storage::domain::entities::{Blob, Object};
    use just_storage::domain::value_objects::{ContentHash, Namespace, ObjectStatus, StorageClass, TenantId};

    /// Create a test object with default values
    pub fn create_test_object() -> Object {
        let mut obj = Object::new(
            Namespace::new("test".to_string()).unwrap(),
            TenantId::new(Uuid::new_v4()),
            Some("test_key".to_string()),
            StorageClass::Hot,
        );
        // Commit the object to set content hash and size
        let content_hash =
            ContentHash::from_hex("testhash12345678901234567890123456789012".to_string()).unwrap();
        obj.commit(&content_hash, 1024).unwrap();
        obj.set_content_type("application/json".to_string());
        // Metadata is already initialized with default values
        obj
    }

    /// Create a test object with custom parameters
    pub fn create_custom_object(
        namespace: &str,
        tenant_id: &str,
        key: Option<&str>,
        storage_class: StorageClass,
        status: ObjectStatus,
        content_hash: &str,
        size_bytes: Option<u64>,
    ) -> Object {
        let uuid = Uuid::parse_str(tenant_id).unwrap_or_else(|_| Uuid::new_v4());
        let mut obj = Object::new(
            Namespace::new(namespace.to_string()).unwrap(),
            TenantId::new(uuid),
            key.map(|s| s.to_string()),
            storage_class,
        );

        // If the object should be committed, commit it with the provided hash and size
        if status == ObjectStatus::Committed {
            if let Some(size) = size_bytes {
                let hash = ContentHash::from_hex(content_hash.to_string()).unwrap();
                obj.commit(&hash, size).unwrap();
            }
        }

        obj.set_content_type("application/octet-stream".to_string());
        obj
    }

    /// Create a test blob
    pub fn create_test_blob(content_hash: &ContentHash, storage_class: StorageClass) -> Blob {
        Blob::new(
            content_hash.clone(),
            storage_class,
            1024, // size_bytes
        )
    }
}

#[path = "common/mod.rs"]
mod common;

pub use crate::common::assertions;
pub use common::http;
pub use common::mocks;
