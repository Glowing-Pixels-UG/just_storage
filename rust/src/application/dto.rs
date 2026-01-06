use serde::{Deserialize, Serialize};
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
            created_at: obj.created_at().to_rfc3339(),
            updated_at: obj.updated_at().to_rfc3339(),
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
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
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
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
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
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// DTO for API key response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyDto {
    pub id: String,
    pub tenant_id: String,
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
            name: api_key.name().to_string(),
            description: api_key.description().map(|s| s.to_string()),
            permissions: api_key.permissions().clone(),
            is_active: api_key.is_active(),
            expires_at: api_key.expires_at().map(|dt| dt.to_rfc3339()),
            created_at: api_key.created_at().to_rfc3339(),
            updated_at: api_key.updated_at().to_rfc3339(),
            last_used_at: api_key.last_used_at().map(|dt| dt.to_rfc3339()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::{ApiKeyPermissions, StorageClass};
    use chrono::{DateTime, Utc};
    use validator::Validate;

    mod upload_request_tests {
        use super::*;

        #[test]
        fn test_upload_request_creation() {
            let request = UploadRequest {
                namespace: "test-namespace".to_string(),
                tenant_id: "test-tenant".to_string(),
                key: Some("test-key".to_string()),
                storage_class: Some(StorageClass::Hot),
            };

            assert_eq!(request.namespace, "test-namespace");
            assert_eq!(request.tenant_id, "test-tenant");
            assert_eq!(request.key, Some("test-key".to_string()));
            assert_eq!(request.storage_class, Some(StorageClass::Hot));
        }

        #[test]
        fn test_upload_request_default_storage_class() {
            let request = UploadRequest {
                namespace: "test".to_string(),
                tenant_id: "tenant".to_string(),
                key: None,
                storage_class: None,
            };

            assert_eq!(request.storage_class, None);
        }

        #[test]
        fn test_upload_request_serialization() {
            let request = UploadRequest {
                namespace: "test".to_string(),
                tenant_id: "tenant".to_string(),
                key: Some("key".to_string()),
                storage_class: Some(StorageClass::Cold),
            };

            let json = serde_json::to_string(&request).unwrap();
            let deserialized: UploadRequest = serde_json::from_str(&json).unwrap();

            assert_eq!(request, deserialized);
        }
    }

    mod search_request_tests {
        use super::*;

        #[test]
        fn test_search_request_validation_valid() {
            let request = SearchRequest {
                namespace: "test".to_string(),
                tenant_id: "tenant".to_string(),
                query: "valid query".to_string(),
                limit: Some(10),
                offset: Some(0),
            };

            assert!(request.validate().is_ok());
        }

        #[test]
        fn test_search_request_validation_empty_query() {
            let request = SearchRequest {
                namespace: "test".to_string(),
                tenant_id: "tenant".to_string(),
                query: "".to_string(),
                limit: Some(10),
                offset: Some(0),
            };

            assert!(request.validate().is_err());
        }

        #[test]
        fn test_search_request_validation_whitespace_query() {
            let request = SearchRequest {
                namespace: "test".to_string(),
                tenant_id: "tenant".to_string(),
                query: "   ".to_string(),
                limit: Some(10),
                offset: Some(0),
            };

            assert!(request.validate().is_err());
        }
    }

    mod create_api_key_request_tests {
        use super::*;

        #[test]
        fn test_create_api_key_request_validation_valid() {
            let request = CreateApiKeyRequest {
                name: "Valid Name".to_string(),
                description: Some("Valid description".to_string()),
                permissions: Some(ApiKeyPermissions::full_access()),
                expires_at: None,
            };

            assert!(request.validate().is_ok());
        }

        #[test]
        fn test_create_api_key_request_validation_empty_name() {
            let request = CreateApiKeyRequest {
                name: "".to_string(),
                description: None,
                permissions: None,
                expires_at: None,
            };

            assert!(request.validate().is_err());
        }

        #[test]
        fn test_create_api_key_request_validation_long_description() {
            let long_desc = "a".repeat(501); // Over 500 char limit
            let request = CreateApiKeyRequest {
                name: "Test".to_string(),
                description: Some(long_desc),
                permissions: None,
                expires_at: None,
            };

            assert!(request.validate().is_err());
        }
    }

    #[test]
    fn test_request_dto_validation_comprehensive() {
        // Test all request DTOs have proper validation

        // Valid requests should pass
        let valid_upload = UploadRequest {
            namespace: "valid-namespace".to_string(),
            tenant_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            key: Some("valid-key".to_string()),
            storage_class: Some(StorageClass::Hot),
        };

        let valid_search = SearchRequest {
            namespace: "valid-namespace".to_string(),
            tenant_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            query: "valid query".to_string(),
            limit: Some(10),
            offset: Some(0),
        };

        let valid_create_api_key = CreateApiKeyRequest {
            name: "Valid API Key".to_string(),
            description: Some("Valid description".to_string()),
            permissions: Some(ApiKeyPermissions::read_only()),
            expires_at: None,
        };

        assert!(valid_upload.validate().is_ok());
        assert!(valid_search.validate().is_ok());
        assert!(valid_create_api_key.validate().is_ok());
    }

    #[test]
    fn test_dto_serialization_round_trip() {
        // Test that DTOs can be serialized and deserialized correctly
        let original = ObjectDto {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            namespace: "test-namespace".to_string(),
            tenant_id: "test-tenant".to_string(),
            key: Some("test-key".to_string()),
            status: crate::domain::value_objects::ObjectStatus::Committed,
            storage_class: StorageClass::Hot,
            content_hash: Some("testhash12345678901234567890123456789012".to_string()),
            size_bytes: Some(1024),
            content_type: Some("application/json".to_string()),
            metadata: crate::domain::value_objects::ObjectMetadata::default(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ObjectDto = serde_json::from_str(&json).unwrap();

        assert_eq!(original.id, deserialized.id);
        assert_eq!(original.namespace, deserialized.namespace);
        assert_eq!(original.tenant_id, deserialized.tenant_id);
        assert_eq!(original.key, deserialized.key);
        assert_eq!(original.status, deserialized.status);
        assert_eq!(original.storage_class, deserialized.storage_class);
    }
}
