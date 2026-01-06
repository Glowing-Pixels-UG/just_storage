use async_trait::async_trait;

use crate::domain::{entities::ApiKey, value_objects::ApiKeyId};

/// Repository error for API key operations
#[derive(Debug, thiserror::Error)]
pub enum ApiKeyRepositoryError {
    #[error("API key not found: {0}")]
    NotFound(String),
    #[error("API key already exists: {0}")]
    AlreadyExists(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Validation error: {0}")]
    Validation(String),
}

/// API key repository interface
#[async_trait]
pub trait ApiKeyRepository: Send + Sync {
    /// Create a new API key
    async fn create(&self, api_key: ApiKey) -> Result<(), ApiKeyRepositoryError>;

    /// Find API key by ID
    async fn find_by_id(&self, id: &ApiKeyId) -> Result<Option<ApiKey>, ApiKeyRepositoryError>;

    /// Find API key by key value (for authentication)
    async fn find_by_key(&self, key: &str) -> Result<Option<ApiKey>, ApiKeyRepositoryError>;

    /// List API keys for a tenant
    async fn list_by_tenant(
        &self,
        tenant_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ApiKey>, ApiKeyRepositoryError>;

    /// Count API keys for a tenant
    async fn count_by_tenant(&self, tenant_id: &str) -> Result<i64, ApiKeyRepositoryError>;

    /// Update an API key
    async fn update(&self, api_key: &ApiKey) -> Result<(), ApiKeyRepositoryError>;

    /// Delete an API key
    async fn delete(&self, id: &ApiKeyId) -> Result<(), ApiKeyRepositoryError>;

    /// Mark API key as used (update last_used_at)
    async fn mark_used(&self, id: &ApiKeyId) -> Result<(), ApiKeyRepositoryError>;

    /// Clean up expired API keys
    async fn cleanup_expired(&self) -> Result<i64, ApiKeyRepositoryError>;
}

#[cfg(test)]
mockall::mock! {
    pub ApiKeyRepository {}

    #[async_trait]
    impl ApiKeyRepository for ApiKeyRepository {
        async fn create(&self, api_key: ApiKey) -> Result<(), ApiKeyRepositoryError>;
        async fn find_by_id(&self, id: &ApiKeyId) -> Result<Option<ApiKey>, ApiKeyRepositoryError>;
        async fn find_by_key(&self, key: &str) -> Result<Option<ApiKey>, ApiKeyRepositoryError>;
        async fn list_by_tenant(&self, tenant_id: &str, limit: i64, offset: i64) -> Result<Vec<ApiKey>, ApiKeyRepositoryError>;
        async fn count_by_tenant(&self, tenant_id: &str) -> Result<i64, ApiKeyRepositoryError>;
        async fn update(&self, api_key: &ApiKey) -> Result<(), ApiKeyRepositoryError>;
        async fn delete(&self, id: &ApiKeyId) -> Result<(), ApiKeyRepositoryError>;
        async fn mark_used(&self, id: &ApiKeyId) -> Result<(), ApiKeyRepositoryError>;
        async fn cleanup_expired(&self) -> Result<i64, ApiKeyRepositoryError>;
    }
}
