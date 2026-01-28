#![allow(dead_code)]

//! Domain object factories & test builders (Phase 1 subset)

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
    let content_hash = ContentHash::from_hex("a".repeat(64)).unwrap();
    obj.commit(&content_hash, 1024).unwrap();
    obj.set_content_type("application/json".to_string());
    obj
}

/// Create a test blob with given content hash string
pub fn create_test_blob_from_hex(hash_hex: &str, storage_class: StorageClass) -> Blob {
    let hash = ContentHash::from_hex(hash_hex.to_string()).unwrap();
    Blob::new(hash, storage_class, 1024)
}

/// Create a test blob from an existing ContentHash
pub fn create_test_blob(content_hash: &ContentHash, storage_class: StorageClass) -> Blob {
    Blob::new(content_hash.clone(), storage_class, 1024)
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
