use std::sync::Arc;
use thiserror::Error;

use crate::application::ports::{
    BlobRepository, BlobStore, ObjectRepository, RepositoryError, StorageError,
};
use crate::domain::errors::DomainError;
use crate::domain::value_objects::ObjectId;

#[derive(Debug, Error)]
pub enum DeleteError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Object not found: {0}")]
    NotFound(String),
}

/// Use case: Delete an object
pub struct DeleteObjectUseCase {
    object_repo: Arc<dyn ObjectRepository>,
    blob_repo: Arc<dyn BlobRepository>,
    blob_store: Arc<dyn BlobStore>,
}

impl DeleteObjectUseCase {
    pub fn new(
        object_repo: Arc<dyn ObjectRepository>,
        blob_repo: Arc<dyn BlobRepository>,
        blob_store: Arc<dyn BlobStore>,
    ) -> Self {
        Self {
            object_repo,
            blob_repo,
            blob_store,
        }
    }

    /// Execute delete workflow
    pub async fn execute(&self, object_id: &ObjectId) -> Result<(), DeleteError> {
        // 1. Find object
        let mut object = match self.object_repo.find_by_id(object_id).await {
            Ok(Some(obj)) => obj,
            Ok(None) => return Err(DeleteError::NotFound(object_id.to_string())),
            Err(crate::application::ports::RepositoryError::SerializationError(e)) => {
                tracing::error!(%e, "Repository serialization error while loading object {}", object_id);
                return Err(DeleteError::NotFound(object_id.to_string()));
            }
            Err(e) => return Err(DeleteError::Repository(e)),
        };

        // 2. Mark for deletion (domain validation)
        object.mark_for_deletion()?;
        self.object_repo.save(&object).await?;

        // 3. Decrement blob ref count
        if let Some(content_hash) = object.content_hash() {
            let ref_count = self.blob_repo.decrement_ref(content_hash).await?;

            // 4. If no more references, delete blob file
            if ref_count == 0 {
                self.blob_store
                    .delete(content_hash, object.storage_class())
                    .await?;

                // Delete blob entry
                self.blob_repo.delete(content_hash).await?;
            }
        }

        // 5. Mark as deleted
        object.mark_deleted()?;
        self.object_repo.save(&object).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{MockBlobRepository, MockBlobStore, MockObjectRepository};
    use crate::domain::entities::Object;
    use crate::domain::value_objects::{ContentHash, Namespace, ObjectId, StorageClass, TenantId};
    use std::str::FromStr;
    use std::sync::Arc;
    use uuid::Uuid;

    fn create_test_object() -> Object {
        let mut object = Object::new(
            Namespace::from_str("test").unwrap(),
            TenantId::new(Uuid::new_v4()),
            Some("key".to_string()),
            StorageClass::Hot,
        );
        let content_hash = ContentHash::from_str(&"a".repeat(64)).unwrap();
        object.commit(content_hash, 123).unwrap();
        object
    }

    #[tokio::test]
    async fn test_delete_object_happy_path() {
        // Arrange
        let mut mock_object_repo = MockObjectRepository::new();
        let mut mock_blob_repo = MockBlobRepository::new();
        let mut mock_blob_store = MockBlobStore::new();

        let object = create_test_object();
        let object_id = *object.id();

        mock_object_repo
            .expect_find_by_id()
            .withf(move |id| id == &object_id)
            .times(1)
            .returning(move |_| Ok(Some(object.clone())));

        mock_object_repo
            .expect_save()
            .times(2)
            .returning(|_| Ok(()));

        mock_blob_repo
            .expect_decrement_ref()
            .times(1)
            .returning(|_| Ok(0)); // ref_count becomes 0

        mock_blob_store
            .expect_delete()
            .times(1)
            .returning(|_, _| Ok(()));

        mock_blob_repo
            .expect_delete()
            .times(1)
            .returning(|_| Ok(()));

        let use_case = DeleteObjectUseCase::new(
            Arc::new(mock_object_repo),
            Arc::new(mock_blob_repo),
            Arc::new(mock_blob_store),
        );

        // Act
        let result = use_case.execute(&object_id).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_object_not_found() {
        // Arrange
        let mut mock_object_repo = MockObjectRepository::new();
        let mock_blob_repo = MockBlobRepository::new();
        let mock_blob_store = MockBlobStore::new();
        let object_id = ObjectId::new();

        mock_object_repo
            .expect_find_by_id()
            .withf(move |id| id == &object_id)
            .times(1)
            .returning(|_| Ok(None));

        let use_case = DeleteObjectUseCase::new(
            Arc::new(mock_object_repo),
            Arc::new(mock_blob_repo),
            Arc::new(mock_blob_store),
        );

        // Act
        let result = use_case.execute(&object_id).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DeleteError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_delete_object_shared_blob() {
        // Arrange
        let mut mock_object_repo = MockObjectRepository::new();
        let mut mock_blob_repo = MockBlobRepository::new();
        let mock_blob_store = MockBlobStore::new(); // Note: No delete expectation

        let object = create_test_object();
        let object_id = *object.id();

        mock_object_repo
            .expect_find_by_id()
            .withf(move |id| id == &object_id)
            .times(1)
            .returning(move |_| Ok(Some(object.clone())));

        mock_object_repo
            .expect_save()
            .times(2)
            .returning(|_| Ok(()));

        mock_blob_repo
            .expect_decrement_ref()
            .times(1)
            .returning(|_| Ok(1)); // ref_count > 0

        let use_case = DeleteObjectUseCase::new(
            Arc::new(mock_object_repo),
            Arc::new(mock_blob_repo),
            Arc::new(mock_blob_store),
        );

        // Act
        let result = use_case.execute(&object_id).await;

        // Assert
        assert!(result.is_ok());
    }
}
