//! Builders for test objects, blobs, and DTOs (Phase 1 helpers)

use std::str::FromStr;
use uuid::Uuid;

use just_storage::application::dto::UploadRequest;
use just_storage::domain::entities::{Blob, Object};
use just_storage::domain::value_objects::{ContentHash, ObjectStatus, StorageClass, TenantId, Namespace};

/// Builder for domain `Object` test instances
pub struct ObjectBuilder {
    namespace: String,
    tenant_id: Uuid,
    key: Option<String>,
    storage_class: StorageClass,
    status: ObjectStatus,
    content_hash: Option<String>,
    size_bytes: Option<u64>,
}

impl Default for ObjectBuilder {
    fn default() -> Self {
        Self {
            namespace: "test".to_string(),
            tenant_id: Uuid::new_v4(),
            key: Some("test_key".to_string()),
            storage_class: StorageClass::Hot,
            status: ObjectStatus::Committed,
            content_hash: Some("a".repeat(64)),
            size_bytes: Some(1024),
        }
    }
}

impl ObjectBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn namespace(mut self, ns: &str) -> Self {
        self.namespace = ns.to_string();
        self
    }

    pub fn tenant_id(mut self, id: Uuid) -> Self {
        self.tenant_id = id;
        self
    }

    pub fn key(mut self, k: Option<&str>) -> Self {
        self.key = k.map(|s| s.to_string());
        self
    }

    pub fn storage_class(mut self, sc: StorageClass) -> Self {
        self.storage_class = sc;
        self
    }

    pub fn status(mut self, st: ObjectStatus) -> Self {
        self.status = st;
        self
    }

    pub fn content_hash(mut self, hex: &str) -> Self {
        self.content_hash = Some(hex.to_string());
        self
    }

    pub fn size_bytes(mut self, size: u64) -> Self {
        self.size_bytes = Some(size);
        self
    }

    pub fn build(self) -> Object {
        let mut obj = Object::new(
            Namespace::new(self.namespace).unwrap(),
            TenantId::new(self.tenant_id),
            self.key,
            self.storage_class,
        );

        if self.status == ObjectStatus::Committed {
            if let (Some(hash_hex), Some(size)) = (self.content_hash, self.size_bytes) {
                let hash = ContentHash::from_hex(hash_hex).expect("invalid hash");
                obj.commit(&hash, size).expect("commit failed");
            }
        }

        obj
    }
}

/// Builder for `Blob` test instances
pub struct BlobBuilder {
    content_hash_hex: String,
    storage_class: StorageClass,
    size_bytes: u64,
}

impl Default for BlobBuilder {
    fn default() -> Self {
        Self {
            content_hash_hex: "a".repeat(64),
            storage_class: StorageClass::Hot,
            size_bytes: 1024,
        }
    }
}

impl BlobBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn hex(mut self, hex: &str) -> Self {
        self.content_hash_hex = hex.to_string();
        self
    }

    pub fn storage_class(mut self, sc: StorageClass) -> Self {
        self.storage_class = sc;
        self
    }

    pub fn size_bytes(mut self, size: u64) -> Self {
        self.size_bytes = size;
        self
    }

    pub fn build(self) -> Blob {
        let hash = ContentHash::from_hex(self.content_hash_hex).expect("invalid hash");
        Blob::new(hash, self.storage_class, self.size_bytes)
    }
}

/// Builder for `UploadRequest` DTOs
pub struct UploadRequestBuilder {
    namespace: String,
    tenant_id: String,
    key: Option<String>,
    storage_class: Option<StorageClass>,
}

impl Default for UploadRequestBuilder {
    fn default() -> Self {
        Self {
            namespace: "test".to_string(),
            tenant_id: Uuid::new_v4().to_string(),
            key: Some("test_key".to_string()),
            storage_class: Some(StorageClass::Hot),
        }
    }
}

impl UploadRequestBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn namespace(mut self, ns: &str) -> Self {
        self.namespace = ns.to_string();
        self
    }

    pub fn tenant_id(mut self, tenant: &str) -> Self {
        self.tenant_id = tenant.to_string();
        self
    }

    pub fn key(mut self, key: Option<&str>) -> Self {
        self.key = key.map(|s| s.to_string());
        self
    }

    pub fn storage_class(mut self, sc: StorageClass) -> Self {
        self.storage_class = Some(sc);
        self
    }

    pub fn build(self) -> UploadRequest {
        UploadRequest {
            namespace: self.namespace,
            tenant_id: self.tenant_id,
            key: self.key,
            storage_class: self.storage_class,
        }
    }
}
