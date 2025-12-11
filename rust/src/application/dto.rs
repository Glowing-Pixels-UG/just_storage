use serde::{Deserialize, Serialize};

use crate::domain::{
    entities::Object,
    value_objects::{ObjectId, ObjectMetadata, ObjectStatus, StorageClass},
};

/// DTO for object metadata responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectDto {
    pub id: String,
    pub namespace: String,
    pub tenant_id: String,
    pub key: Option<String>,
    pub status: ObjectStatus,
    pub storage_class: StorageClass,
    pub content_hash: Option<String>,
    pub size_bytes: Option<u64>,
    pub content_type: Option<String>,
    pub metadata: ObjectMetadata,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Object> for ObjectDto {
    fn from(obj: Object) -> Self {
        Self {
            id: obj.id().to_string(),
            namespace: obj.namespace().to_string(),
            tenant_id: obj.tenant_id().to_string(),
            key: obj.key().map(|k| k.to_string()),
            status: obj.status(),
            storage_class: obj.storage_class(),
            content_hash: obj.content_hash().map(|h| h.to_string()),
            size_bytes: obj.size_bytes(),
            content_type: obj.content_type().map(|c| c.to_string()),
            metadata: obj.metadata().clone(),
            created_at: obj.created_at().to_rfc3339(),
            updated_at: obj.updated_at().to_rfc3339(),
        }
    }
}

/// DTO for upload request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadRequest {
    pub namespace: String,
    pub tenant_id: String,
    pub key: Option<String>,
    pub storage_class: Option<StorageClass>,
}

/// DTO for list request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRequest {
    pub namespace: String,
    pub tenant_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// DTO for list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse {
    pub objects: Vec<ObjectDto>,
    pub total: usize,
    pub limit: i64,
    pub offset: i64,
}

/// DTO for download response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadMetadata {
    pub object_id: ObjectId,
    pub size_bytes: u64,
    pub content_hash: String,
}
