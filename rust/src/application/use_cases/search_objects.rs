use std::sync::Arc;

use crate::application::dto::{ObjectDto, SearchRequest, SearchResponse};
use crate::application::errors::ObjectUseCaseError;
use crate::application::ports::ObjectRepository;
use crate::application::validation::validate_namespace_and_tenant;

/// Use case: Advanced search for objects with filters
pub struct SearchObjectsUseCase {
    object_repo: Arc<dyn ObjectRepository>,
}

impl SearchObjectsUseCase {
    pub fn new(object_repo: Arc<dyn ObjectRepository>) -> Self {
        Self { object_repo }
    }

    /// Execute advanced search with filters
    pub async fn execute(
        &self,
        request: SearchRequest,
    ) -> Result<SearchResponse, ObjectUseCaseError> {
        // 1. Parse and validate namespace and tenant_id for logging/security
        let (_namespace, _tenant_id) =
            validate_namespace_and_tenant(&request.namespace, &request.tenant_id)?;

        // Note: We don't validate the search request here as it's optional filters

        // 2. Query repository with search filters
        let objects = self.object_repo.search(&request).await?;

        // 3. Convert to DTOs
        let dtos: Vec<ObjectDto> = objects.into_iter().map(ObjectDto::from).collect();

        let total = dtos.len();
        let limit = request.limit.unwrap_or(100).min(1000);
        let offset = request.offset.unwrap_or(0);

        Ok(SearchResponse {
            objects: dtos,
            total,
            limit,
            offset,
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
    async fn test_search_objects_happy_path() {
        // Arrange
        let mut mock_object_repo = MockObjectRepository::new();
        let request = SearchRequest {
            namespace: "test".to_string(),
            tenant_id: Uuid::new_v4().to_string(),
            limit: Some(10),
            offset: Some(0),
            sort_by: None,
            sort_direction: None,
            key_contains: None,
            content_type: None,
            storage_class: None,
            size_range: None,
            created_at_range: None,
            updated_at_range: None,
            metadata_filters: None,
        };

        let objects = vec![create_test_object(), create_test_object()];
        mock_object_repo
            .expect_search()
            .times(1)
            .returning(move |_| Ok(objects.clone()));

        let use_case = SearchObjectsUseCase::new(Arc::new(mock_object_repo));

        // Act
        let result = use_case.execute(request).await;

        // Assert
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.objects.len(), 2);
        assert_eq!(response.total, 2);
    }
}
