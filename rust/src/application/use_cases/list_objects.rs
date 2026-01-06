use std::sync::Arc;

use crate::application::dto::{ListRequest, ListResponse, ObjectDto};
use crate::application::errors::ObjectUseCaseError;
use crate::application::ports::ObjectRepository;
use crate::application::validation::validate_namespace_and_tenant;

/// Use case: List objects
pub struct ListObjectsUseCase {
    object_repo: Arc<dyn ObjectRepository>,
}

impl ListObjectsUseCase {
    pub fn new(object_repo: Arc<dyn ObjectRepository>) -> Self {
        Self { object_repo }
    }

    /// Execute list with pagination
    pub async fn execute(&self, request: ListRequest) -> Result<ListResponse, ObjectUseCaseError> {
        // 1. Parse and validate
        let (namespace, tenant_id) =
            validate_namespace_and_tenant(&request.namespace, &request.tenant_id)?;

        let limit = request.limit.unwrap_or(100).min(1000); // Cap at 1000
        let offset = request.offset.unwrap_or(0);

        // 2. Query repository
        let objects = self
            .object_repo
            .list(&namespace, &tenant_id, limit, offset)
            .await?;

        // 3. Convert to DTOs
        let dtos: Vec<ObjectDto> = objects.into_iter().map(ObjectDto::from).collect();

        let total = dtos.len();

        Ok(ListResponse {
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
            Some("key".to_string()),
            StorageClass::Hot,
        )
    }

    #[tokio::test]
    async fn test_list_objects_happy_path() {
        // Arrange
        let mut mock_object_repo = MockObjectRepository::new();
        let request = ListRequest {
            namespace: "test".to_string(),
            tenant_id: Uuid::new_v4().to_string(),
            limit: Some(10),
            offset: Some(0),
        };

        let objects = vec![create_test_object(), create_test_object()];
        mock_object_repo
            .expect_list()
            .times(1)
            .returning(move |_, _, _, _| Ok(objects.clone()));

        let use_case = ListObjectsUseCase::new(Arc::new(mock_object_repo));

        // Act
        let result = use_case.execute(request).await;

        // Assert
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.objects.len(), 2);
        assert_eq!(response.total, 2);
    }

    #[tokio::test]
    async fn test_list_objects_empty_result() {
        // Arrange
        let mut mock_object_repo = MockObjectRepository::new();
        let request = ListRequest {
            namespace: "test".to_string(),
            tenant_id: Uuid::new_v4().to_string(),
            limit: Some(10),
            offset: Some(0),
        };

        mock_object_repo
            .expect_list()
            .times(1)
            .returning(|_, _, _, _| Ok(vec![]));

        let use_case = ListObjectsUseCase::new(Arc::new(mock_object_repo));

        // Act
        let result = use_case.execute(request).await;

        // Assert
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.objects.len(), 0);
        assert_eq!(response.total, 0);
    }
}
