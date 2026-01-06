use std::sync::Arc;

use crate::application::{
    dto::{ApiKeyDto, ApiKeyListResponse, CreateApiKeyRequest, UpdateApiKeyRequest},
    ports::{ApiKeyRepository, ApiKeyRepositoryError},
};
use crate::domain::{
    entities::ApiKey,
    value_objects::{ApiKeyId, ApiKeyPermissions},
};

/// Use case for creating API keys
pub struct CreateApiKeyUseCase {
    repository: Arc<dyn ApiKeyRepository>,
}

impl CreateApiKeyUseCase {
    pub fn new(repository: Arc<dyn ApiKeyRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        tenant_id: String,
        request: CreateApiKeyRequest,
    ) -> Result<ApiKeyDto, ApiKeyUseCaseError> {
        let permissions = request
            .permissions
            .unwrap_or_else(ApiKeyPermissions::full_access);

        let api_key = ApiKey::new(
            tenant_id,
            request.name,
            request.description,
            permissions,
            request.expires_at,
        );

        self.repository.create(api_key.clone()).await?;
        Ok(api_key.into())
    }
}

/// Use case for listing API keys
pub struct ListApiKeysUseCase {
    repository: Arc<dyn ApiKeyRepository>,
}

impl ListApiKeysUseCase {
    pub fn new(repository: Arc<dyn ApiKeyRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        tenant_id: String,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<ApiKeyListResponse, ApiKeyUseCaseError> {
        let limit = limit.unwrap_or(50).clamp(1, 100);
        let offset = offset.unwrap_or(0).max(0);

        let api_keys = self
            .repository
            .list_by_tenant(&tenant_id, limit, offset)
            .await?;
        let total = self.repository.count_by_tenant(&tenant_id).await?;

        let api_keys_dto = api_keys.into_iter().map(Into::into).collect();

        Ok(ApiKeyListResponse {
            api_keys: api_keys_dto,
            total: total as usize,
        })
    }
}

/// Use case for getting a single API key
pub struct GetApiKeyUseCase {
    repository: Arc<dyn ApiKeyRepository>,
}

impl GetApiKeyUseCase {
    pub fn new(repository: Arc<dyn ApiKeyRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        tenant_id: &str,
        api_key_id: &str,
    ) -> Result<ApiKeyDto, ApiKeyUseCaseError> {
        let id = api_key_id
            .parse::<ApiKeyId>()
            .map_err(|_| ApiKeyUseCaseError::InvalidId(api_key_id.to_string()))?;

        let api_key = self
            .repository
            .find_by_id(&id)
            .await?
            .ok_or_else(|| ApiKeyUseCaseError::NotFound(api_key_id.to_string()))?;

        // Check tenant ownership
        if api_key.tenant_id() != tenant_id {
            return Err(ApiKeyUseCaseError::NotFound(api_key_id.to_string()));
        }

        Ok(api_key.into())
    }
}

/// Use case for updating API keys
pub struct UpdateApiKeyUseCase {
    repository: Arc<dyn ApiKeyRepository>,
}

impl UpdateApiKeyUseCase {
    pub fn new(repository: Arc<dyn ApiKeyRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        tenant_id: &str,
        api_key_id: &str,
        request: UpdateApiKeyRequest,
    ) -> Result<ApiKeyDto, ApiKeyUseCaseError> {
        let id = api_key_id
            .parse::<ApiKeyId>()
            .map_err(|_| ApiKeyUseCaseError::InvalidId(api_key_id.to_string()))?;

        let mut api_key = self
            .repository
            .find_by_id(&id)
            .await?
            .ok_or_else(|| ApiKeyUseCaseError::NotFound(api_key_id.to_string()))?;

        // Check tenant ownership
        if api_key.tenant_id() != tenant_id {
            return Err(ApiKeyUseCaseError::NotFound(api_key_id.to_string()));
        }

        // Update fields
        if let Some(name) = request.name {
            api_key.set_name(name);
        }
        if let Some(description) = request.description {
            api_key.set_description(Some(description));
        }
        if let Some(permissions) = request.permissions {
            api_key.set_permissions(permissions);
        }
        if let Some(is_active) = request.is_active {
            api_key.set_active(is_active);
        }
        if let Some(expires_at) = request.expires_at {
            api_key.set_expires_at(Some(expires_at));
        }

        self.repository.update(&api_key).await?;
        Ok(api_key.into())
    }
}

/// Use case for deleting API keys
pub struct DeleteApiKeyUseCase {
    repository: Arc<dyn ApiKeyRepository>,
}

impl DeleteApiKeyUseCase {
    pub fn new(repository: Arc<dyn ApiKeyRepository>) -> Self {
        Self { repository }
    }

    pub async fn execute(
        &self,
        tenant_id: &str,
        api_key_id: &str,
    ) -> Result<(), ApiKeyUseCaseError> {
        let id = api_key_id
            .parse::<ApiKeyId>()
            .map_err(|_| ApiKeyUseCaseError::InvalidId(api_key_id.to_string()))?;

        let api_key = self
            .repository
            .find_by_id(&id)
            .await?
            .ok_or_else(|| ApiKeyUseCaseError::NotFound(api_key_id.to_string()))?;

        // Check tenant ownership
        if api_key.tenant_id() != tenant_id {
            return Err(ApiKeyUseCaseError::NotFound(api_key_id.to_string()));
        }

        self.repository.delete(&id).await?;
        Ok(())
    }
}

/// Use case errors
#[derive(Debug, thiserror::Error)]
pub enum ApiKeyUseCaseError {
    #[error("API key not found: {0}")]
    NotFound(String),
    #[error("Invalid API key ID: {0}")]
    InvalidId(String),
    #[error("Repository error: {0}")]
    Repository(#[from] ApiKeyRepositoryError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::ApiKey;
    use crate::domain::value_objects::{ApiKeyId, ApiKeyValue, ApiKeyPermissions};
    use async_trait::async_trait;
    use mockall::mock;
    use mockall::predicate::*;

    // Mock repository for testing
    mock! {
        pub ApiKeyRepositoryImpl {}

        #[async_trait]
        impl ApiKeyRepository for ApiKeyRepositoryImpl {
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

    mod create_api_key_tests {
        use super::*;

        #[tokio::test]
        async fn test_create_api_key_success() {
            let mut mock_repo = MockApiKeyRepositoryImpl::new();
            mock_repo
                .expect_create()
                .times(1)
                .returning(|_| Ok(()));

            let use_case = CreateApiKeyUseCase::new(Arc::new(mock_repo));

            let request = CreateApiKeyRequest {
                name: "Test API Key".to_string(),
                description: Some("Test description".to_string()),
                permissions: Some(ApiKeyPermissions::read_only()),
                expires_at: None,
            };

            let result = use_case.execute("tenant-123".to_string(), request).await;

            assert!(result.is_ok());
            let api_key_dto = result.unwrap();
            assert_eq!(api_key_dto.name, "Test API Key");
            assert_eq!(api_key_dto.description, Some("Test description".to_string()));
            assert_eq!(api_key_dto.permissions, ApiKeyPermissions::read_only());
            assert_eq!(api_key_dto.tenant_id, "tenant-123");
        }

        #[tokio::test]
        async fn test_create_api_key_default_permissions() {
            let mut mock_repo = MockApiKeyRepositoryImpl::new();
            mock_repo
                .expect_create()
                .times(1)
                .returning(|_| Ok(()));

            let use_case = CreateApiKeyUseCase::new(Arc::new(mock_repo));

            let request = CreateApiKeyRequest {
                name: "Test API Key".to_string(),
                description: None,
                permissions: None, // Should default to full access
                expires_at: None,
            };

            let result = use_case.execute("tenant-123".to_string(), request).await;

            assert!(result.is_ok());
            let api_key_dto = result.unwrap();
            assert_eq!(api_key_dto.permissions, ApiKeyPermissions::full_access());
        }

        #[tokio::test]
        async fn test_create_api_key_repository_error() {
            let mut mock_repo = MockApiKeyRepositoryImpl::new();
            mock_repo
                .expect_create()
                .times(1)
                .returning(|_| Err(ApiKeyRepositoryError::Database(sqlx::Error::RowNotFound)));

            let use_case = CreateApiKeyUseCase::new(Arc::new(mock_repo));

            let request = CreateApiKeyRequest {
                name: "Test API Key".to_string(),
                description: None,
                permissions: None,
                expires_at: None,
            };

            let result = use_case.execute("tenant-123".to_string(), request).await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), ApiKeyUseCaseError::Repository(_)));
        }
    }

    mod list_api_keys_tests {
        use super::*;

        #[tokio::test]
        async fn test_list_api_keys_success() {
            let api_keys = vec![
                ApiKey::new(
                    "tenant-123".to_string(),
                    "Key 1".to_string(),
                    Some("Description 1".to_string()),
                    ApiKeyPermissions::read_only(),
                    None,
                ),
                ApiKey::new(
                    "tenant-123".to_string(),
                    "Key 2".to_string(),
                    None,
                    ApiKeyPermissions::full_access(),
                    None,
                ),
            ];

            let mut mock_repo = MockApiKeyRepositoryImpl::new();
            mock_repo
                .expect_list_by_tenant()
                .with(eq("tenant-123"), eq(50), eq(0))
                .times(1)
                .returning(move |_, _, _| Ok(api_keys.clone()));

            let use_case = ListApiKeysUseCase::new(Arc::new(mock_repo));

            let result = use_case.execute("tenant-123".to_string(), None, None).await;

            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.api_keys.len(), 2);
            assert_eq!(response.api_keys[0].name, "Key 1");
            assert_eq!(response.api_keys[1].name, "Key 2");
        }

        #[tokio::test]
        async fn test_list_api_keys_with_pagination() {
            let api_keys = vec![ApiKey::new(
                "tenant-123".to_string(),
                "Key 1".to_string(),
                None,
                ApiKeyPermissions::read_only(),
                None,
            )];

            let mut mock_repo = MockApiKeyRepositoryImpl::new();
            mock_repo
                .expect_list_by_tenant()
                .with(eq("tenant-123"), eq(10), eq(20))
                .times(1)
                .returning(move |_, _, _| Ok(api_keys.clone()));

            let use_case = ListApiKeysUseCase::new(Arc::new(mock_repo));

            let result = use_case.execute("tenant-123".to_string(), Some(10), Some(20)).await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_list_api_keys_empty() {
            let mut mock_repo = MockApiKeyRepositoryImpl::new();
            mock_repo
                .expect_list_by_tenant()
                .times(1)
                .returning(|_, _, _| Ok(vec![]));

            let use_case = ListApiKeysUseCase::new(Arc::new(mock_repo));

            let result = use_case.execute("tenant-123".to_string(), None, None).await;

            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.api_keys.len(), 0);
        }
    }

    mod get_api_key_tests {
        use super::*;

        #[tokio::test]
        async fn test_get_api_key_success() {
            let api_key = ApiKey::new(
                "tenant-123".to_string(),
                "Test Key".to_string(),
                Some("Test description".to_string()),
                ApiKeyPermissions::read_only(),
                None,
            );
            let api_key_id = *api_key.id();

            let mut mock_repo = MockApiKeyRepositoryImpl::new();
            mock_repo
                .expect_find_by_id()
                .with(eq(api_key_id))
                .times(1)
                .returning(move |_| Ok(Some(api_key.clone())));

            let use_case = GetApiKeyUseCase::new(Arc::new(mock_repo));

            let result = use_case.execute("tenant-123", &api_key_id.to_string()).await;

            assert!(result.is_ok());
            let api_key_dto = result.unwrap();
            assert_eq!(api_key_dto.name, "Test Key");
        }

        #[tokio::test]
        async fn test_get_api_key_not_found() {
            let api_key_id = ApiKeyId::new();

            let mut mock_repo = MockApiKeyRepositoryImpl::new();
            mock_repo
                .expect_find_by_id()
                .with(eq(api_key_id))
                .times(1)
                .returning(|_| Ok(None));

            let use_case = GetApiKeyUseCase::new(Arc::new(mock_repo));

            let result = use_case.execute("tenant-123", &api_key_id.to_string()).await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), ApiKeyUseCaseError::NotFound(_)));
        }
    }

    mod update_api_key_tests {
        use super::*;

        #[tokio::test]
        async fn test_update_api_key_success() {
            let api_key = ApiKey::new(
                "tenant-123".to_string(),
                "Original Name".to_string(),
                Some("Original description".to_string()),
                ApiKeyPermissions::read_only(),
                None,
            );
            let api_key_id = *api_key.id();

            let mut mock_repo = MockApiKeyRepositoryImpl::new();
            mock_repo
                .expect_find_by_id()
                .with(eq(api_key_id))
                .times(1)
                .returning(move |_| Ok(Some(api_key.clone())));

            mock_repo
                .expect_update()
                .times(1)
                .returning(|_| Ok(()));

            let use_case = UpdateApiKeyUseCase::new(Arc::new(mock_repo));

            let request = UpdateApiKeyRequest {
                name: Some("Updated Name".to_string()),
                description: Some("Updated description".to_string()),
                permissions: Some(ApiKeyPermissions::full_access()),
                is_active: None,
                expires_at: None,
            };

            let result = use_case.execute("tenant-123", &api_key_id.to_string(), request).await;

            assert!(result.is_ok());
            let api_key_dto = result.unwrap();
            assert_eq!(api_key_dto.name, "Updated Name");
            assert_eq!(api_key_dto.permissions, ApiKeyPermissions::full_access());
        }

        #[tokio::test]
        async fn test_update_api_key_not_found() {
            let api_key_id = ApiKeyId::new();

            let mut mock_repo = MockApiKeyRepositoryImpl::new();
            mock_repo
                .expect_find_by_id()
                .with(eq(api_key_id))
                .times(1)
                .returning(|_| Ok(None));

            let use_case = UpdateApiKeyUseCase::new(Arc::new(mock_repo));

            let request = UpdateApiKeyRequest {
                name: Some("Updated Name".to_string()),
                description: None,
                permissions: None,
                is_active: None,
                expires_at: None,
            };

            let result = use_case.execute("tenant-123", &api_key_id.to_string(), request).await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), ApiKeyUseCaseError::NotFound(_)));
        }
    }

    mod delete_api_key_tests {
        use super::*;

        #[tokio::test]
        async fn test_delete_api_key_success() {
            let api_key_id = ApiKeyId::new();

            let mut mock_repo = MockApiKeyRepositoryImpl::new();
            mock_repo
                .expect_delete()
                .with(eq(api_key_id))
                .times(1)
                .returning(|_| Ok(()));

            let use_case = DeleteApiKeyUseCase::new(Arc::new(mock_repo));

            let result = use_case.execute("tenant-123", &api_key_id.to_string()).await;

            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_delete_api_key_repository_error() {
            let api_key_id = ApiKeyId::new();

            let mut mock_repo = MockApiKeyRepositoryImpl::new();
            mock_repo
                .expect_delete()
                .with(eq(api_key_id))
                .times(1)
                .returning(|_| Err(ApiKeyRepositoryError::Database(sqlx::Error::RowNotFound)));

            let use_case = DeleteApiKeyUseCase::new(Arc::new(mock_repo));

            let result = use_case.execute("tenant-123", &api_key_id.to_string()).await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), ApiKeyUseCaseError::Repository(_)));
        }
    }
}
