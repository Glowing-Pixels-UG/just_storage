use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use utoipa::ToSchema;
use validator::Validate;

use crate::domain::{
    entities::Object,
    value_objects::{ApiKeyPermissions, ObjectId, ObjectMetadata, ObjectStatus, StorageClass},
};

/// DTO for object metadata responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
            created_at: obj.created_at().format(&Rfc3339).unwrap_or_default(),
            updated_at: obj.updated_at().format(&Rfc3339).unwrap_or_default(),
        }
    }
}

/// DTO for upload request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, Validate)]
pub struct UploadRequest {
    #[validate(length(min = 1, max = 100))]
    pub namespace: String,
    #[validate(length(min = 1, max = 100))]
    pub tenant_id: String,
    #[validate(length(max = 255))]
    pub key: Option<String>,
    pub storage_class: Option<StorageClass>,
}

/// DTO for list request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct ListRequest {
    #[validate(length(min = 1, max = 100))]
    pub namespace: String,
    #[validate(length(min = 1, max = 100))]
    pub tenant_id: String,
    #[validate(range(min = 1, max = 1000))]
    pub limit: Option<i64>,
    #[validate(range(min = 0))]
    pub offset: Option<i64>,
}

/// Sorting options for search results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    CreatedAt,
    UpdatedAt,
    SizeBytes,
    Key,
    ContentType,
}

/// Sort direction
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Date range filter
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DateRange {
    pub from: Option<time::OffsetDateTime>,
    pub to: Option<time::OffsetDateTime>,
}

/// Size range filter
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct SizeRange {
    #[validate(range(min = 0))]
    pub min: Option<u64>,
    #[validate(range(min = 0))]
    pub max: Option<u64>,
}

/// Advanced search request with filters
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct SearchRequest {
    #[validate(length(min = 1, max = 100))]
    pub namespace: String,
    #[validate(length(min = 1, max = 100))]
    pub tenant_id: String,

    // Pagination
    #[validate(range(min = 1, max = 1000))]
    pub limit: Option<i64>,
    #[validate(range(min = 0))]
    pub offset: Option<i64>,

    // Sorting
    pub sort_by: Option<SortField>,
    pub sort_direction: Option<SortDirection>,

    // Basic filters
    #[validate(length(max = 255))]
    pub key_contains: Option<String>,
    #[validate(length(max = 255))]
    pub content_type: Option<String>,
    pub storage_class: Option<crate::domain::value_objects::StorageClass>,

    // Range filters
    pub size_range: Option<SizeRange>,
    pub created_at_range: Option<DateRange>,
    pub updated_at_range: Option<DateRange>,

    // Metadata filters (JSON path queries)
    pub metadata_filters: Option<serde_json::Value>,
}

/// Text search request (full-text search)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TextSearchRequest {
    #[validate(length(min = 1, max = 100))]
    pub namespace: String,
    #[validate(length(min = 1, max = 100))]
    pub tenant_id: String,

    // Pagination
    #[validate(range(min = 1, max = 1000))]
    pub limit: Option<i64>,
    #[validate(range(min = 0))]
    pub offset: Option<i64>,

    // Full-text search query
    #[validate(length(min = 1, max = 1000))]
    pub query: String,

    // Search in specific fields
    pub search_in_metadata: Option<bool>, // default: true
    pub search_in_key: Option<bool>,      // default: true
}

/// DTO for list response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListResponse {
    pub objects: Vec<ObjectDto>,
    pub total: usize,
    pub limit: i64,
    pub offset: i64,
}

/// DTO for search response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchResponse {
    pub objects: Vec<ObjectDto>,
    pub total: usize,
    pub limit: i64,
    pub offset: i64,
}

/// DTO for text search response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TextSearchResponse {
    pub objects: Vec<ObjectDto>,
    pub total: usize,
    pub limit: i64,
    pub offset: i64,
    pub query: String,
}

/// DTO for download response metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadMetadata {
    pub object_id: ObjectId,
    pub size_bytes: u64,
    pub content_hash: String,
}

/// DTO for API key creation request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct CreateApiKeyRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(max = 500))]
    pub description: Option<String>,
    pub permissions: Option<ApiKeyPermissions>,
    pub expires_at: Option<time::OffsetDateTime>,
}

/// DTO for API key update request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateApiKeyRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    #[validate(length(max = 500))]
    pub description: Option<String>,
    pub permissions: Option<ApiKeyPermissions>,
    pub is_active: Option<bool>,
    pub expires_at: Option<time::OffsetDateTime>,
}

/// DTO for API key response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyDto {
    pub id: String,
    pub tenant_id: String,
    pub key: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub permissions: ApiKeyPermissions,
    pub is_active: bool,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_used_at: Option<String>,
}

/// DTO for API key list response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyListResponse {
    pub api_keys: Vec<ApiKeyDto>,
    pub total: usize,
}

impl From<crate::domain::entities::ApiKey> for ApiKeyDto {
    fn from(api_key: crate::domain::entities::ApiKey) -> Self {
        Self {
            id: api_key.id().to_string(),
            tenant_id: api_key.tenant_id().to_string(),
            key: None, // Cleartext key should be handled explicitly when needed
            name: api_key.name().to_string(),
            description: api_key.description().map(|s| s.to_string()),
            permissions: api_key.permissions().clone(),
            is_active: api_key.is_active(),
            expires_at: api_key
                .expires_at()
                .map(|dt| dt.format(&Rfc3339).unwrap_or_default()),
            created_at: api_key.created_at().format(&Rfc3339).unwrap_or_default(),
            updated_at: api_key.updated_at().format(&Rfc3339).unwrap_or_default(),
            last_used_at: api_key
                .last_used_at()
                .map(|dt| dt.format(&Rfc3339).unwrap_or_default()),
        }
    }
}
