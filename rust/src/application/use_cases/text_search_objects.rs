use std::sync::Arc;

use crate::application::dto::{ObjectDto, TextSearchRequest, TextSearchResponse};
use crate::application::errors::TextSearchUseCaseError;
use crate::application::ports::ObjectRepository;
use crate::application::validation::{
    validate_namespace_and_tenant_for_text_search, validate_search_query,
};

/// Use case: Full-text search across object metadata and keys
pub struct TextSearchObjectsUseCase {
    object_repo: Arc<dyn ObjectRepository>,
}

impl TextSearchObjectsUseCase {
    pub fn new(object_repo: Arc<dyn ObjectRepository>) -> Self {
        Self { object_repo }
    }

    /// Execute full-text search
    pub async fn execute(
        &self,
        request: TextSearchRequest,
    ) -> Result<TextSearchResponse, TextSearchUseCaseError> {
        // 1. Parse and validate
        let (_namespace, _tenant_id) =
            validate_namespace_and_tenant_for_text_search(&request.namespace, &request.tenant_id)?;
        validate_search_query(&request.query)?;

        // 2. Query repository with text search
        let objects = self.object_repo.text_search(&request).await?;

        // 3. Convert to DTOs
        let dtos: Vec<ObjectDto> = objects.into_iter().map(ObjectDto::from).collect();

        let total = dtos.len();
        let limit = request.limit.unwrap_or(100).min(1000);
        let offset = request.offset.unwrap_or(0);

        Ok(TextSearchResponse {
            objects: dtos,
            total,
            limit,
            offset,
            query: request.query,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::MockObjectRepository;
    use crate::domain::entities::Object;
    use crate::domain::value_objects::{Namespace, StorageClass, TenantId};
    use std::str::FromStr;
    use std::sync::Arc;
    use uuid::Uuid;

    fn create_test_object() -> Object {
        Object::new(
            Namespace::from_str("test").unwrap(),
            TenantId::new(Uuid::new_v4()),
            Some("test-key".to_string()),
            StorageClass::Hot,
        )
    }

    #[tokio::test]
    async fn test_text_search_objects_happy_path() {
        // Arrange
        let mut mock_object_repo = MockObjectRepository::new();
        let request = TextSearchRequest {
            namespace: "test".to_string(),
            tenant_id: Uuid::new_v4().to_string(),
            limit: Some(10),
            offset: Some(0),
            query: "llama".to_string(),
            search_in_metadata: Some(true),
            search_in_key: Some(true),
        };

        let objects = vec![create_test_object(), create_test_object()];
        mock_object_repo
            .expect_text_search()
            .times(1)
            .returning(move |_| Ok(objects.clone()));

        let use_case = TextSearchObjectsUseCase::new(Arc::new(mock_object_repo));

        // Act
        let result = use_case.execute(request).await;

        // Assert
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.objects.len(), 2);
        assert_eq!(response.total, 2);
        assert_eq!(response.query, "llama");
    }

    #[tokio::test]
    async fn test_text_search_empty_query() {
        // Arrange
        let mock_object_repo = MockObjectRepository::new();
        let request = TextSearchRequest {
            namespace: "test".to_string(),
            tenant_id: Uuid::new_v4().to_string(),
            limit: Some(10),
            offset: Some(0),
            query: "".to_string(),
            search_in_metadata: Some(true),
            search_in_key: Some(true),
        };

        let use_case = TextSearchObjectsUseCase::new(Arc::new(mock_object_repo));

        // Act
        let result = use_case.execute(request).await;

        // Assert
        assert!(matches!(result, Err(TextSearchUseCaseError::InvalidRequest(_))));
    }
}
