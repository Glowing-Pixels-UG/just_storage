//! Domain object factories & test builders (Phase 1 subset)

use std::sync::Arc;
use uuid::Uuid;

use just_storage::domain::entities::Blob;
use just_storage::domain::entities::Object;
use just_storage::domain::value_objects::{
    ContentHash, Namespace, ObjectStatus, StorageClass, TenantId,
};

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
    obj
}

/// Create a test blob with given content hash string
pub fn create_test_blob_from_hex(hash_hex: &str, storage_class: StorageClass) -> Blob {
    let hash = ContentHash::from_hex(hash_hex.to_string()).unwrap();
    Blob::new(hash, storage_class, 1024)
}
