use std::sync::Arc;
use thiserror::Error;

use crate::application::dto::DownloadMetadata;
use crate::application::ports::{
    BlobReader, BlobStore, ObjectRepository, RepositoryError, StorageError,
};
use crate::domain::value_objects::ObjectId;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Object not found: {0}")]
    NotFound(String),

    #[error("Object not readable (status: {0})")]
    NotReadable(String),
}

/// Use case: Download an object
pub struct DownloadObjectUseCase {
    object_repo: Arc<dyn ObjectRepository>,
    blob_store: Arc<dyn BlobStore>,
}

impl DownloadObjectUseCase {
    pub fn new(object_repo: Arc<dyn ObjectRepository>, blob_store: Arc<dyn BlobStore>) -> Self {
        Self {
            object_repo,
            blob_store,
        }
    }

    /// Execute download by ID
    pub async fn execute_by_id(
        &self,
        object_id: &ObjectId,
    ) -> Result<(DownloadMetadata, BlobReader), DownloadError> {
        // 1. Find object by ID
        let object = match self.object_repo.find_by_id(object_id).await {
            Ok(Some(obj)) => obj,
            Ok(None) => return Err(DownloadError::NotFound(object_id.to_string())),
            Err(crate::application::ports::RepositoryError::SerializationError(e)) => {
                tracing::error!(%e, "Repository serialization error while loading object {}", object_id);
                return Err(DownloadError::NotFound(object_id.to_string()));
            }
            Err(e) => return Err(DownloadError::Repository(e)),
        };

        // 2. Verify object is readable
        if !object.is_readable() {
            return Err(DownloadError::NotReadable(object.status().to_string()));
        }

        // 3. Extract metadata
        let content_hash = object
            .content_hash()
            .ok_or_else(|| DownloadError::NotReadable("No content hash".to_string()))?;

        let size_bytes = object
            .size_bytes()
            .ok_or_else(|| DownloadError::NotReadable("No size".to_string()))?;

        // 4. Open blob for reading
        let reader = self
            .blob_store
            .read(content_hash, object.storage_class())
            .await?;

        // 5. Return metadata + stream
        let metadata = DownloadMetadata {
            object_id: *object.id(),
            size_bytes,
            content_hash: content_hash.to_string(),
        };

        Ok((metadata, reader))
    }

    /// Execute download by key (namespace + tenant + key)
    pub async fn execute_by_key(
        &self,
        namespace: &str,
        tenant_id: &str,
        key: &str,
    ) -> Result<(DownloadMetadata, BlobReader), DownloadError> {
        use crate::domain::value_objects::{Namespace, TenantId};

        // Parse namespace and tenant
        let namespace = Namespace::new(namespace.to_string())
            .map_err(|e| DownloadError::NotFound(e.to_string()))?;

        let tenant_id =
            TenantId::from_string(tenant_id).map_err(|e| DownloadError::NotFound(e.to_string()))?;

        // Find by key
        let object = match self
            .object_repo
            .find_by_key(&namespace, &tenant_id, key)
            .await
        {
            Ok(Some(obj)) => obj,
            Ok(None) => {
                return Err(DownloadError::NotFound(format!(
                    "{}/{}/{}",
                    namespace, tenant_id, key
                )))
            }
            Err(crate::application::ports::RepositoryError::SerializationError(e)) => {
                tracing::error!(%e, "Repository serialization error while loading object by key {}/{}/{}", namespace, tenant_id, key);
                return Err(DownloadError::NotFound(format!(
                    "{}/{}/{}",
                    namespace, tenant_id, key
                )));
            }
            Err(e) => return Err(DownloadError::Repository(e)),
        };

        // Reuse by_id logic
        self.execute_by_id(object.id()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::{MockBlobStore, MockObjectRepository};
    use crate::domain::entities::Object;
    use crate::domain::value_objects::{
        ContentHash, Namespace, ObjectId, ObjectStatus, StorageClass, TenantId,
    };
    use std::io::Cursor;
    use std::str::FromStr;
    use std::sync::Arc;
    use uuid::Uuid;

    fn create_test_object(status: ObjectStatus) -> Object {
        let mut object = Object::new(
            Namespace::from_str("test").unwrap(),
            TenantId::new(Uuid::new_v4()),
            Some("key".to_string()),
            StorageClass::Hot,
        );
        if status != ObjectStatus::Writing {
            let content_hash = ContentHash::from_str(&"a".repeat(64)).unwrap();
            object.commit(content_hash, 123).unwrap();
        }
        if status == ObjectStatus::Deleting || status == ObjectStatus::Deleted {
            object.mark_for_deletion().unwrap();
        }
        if status == ObjectStatus::Deleted {
            object.mark_deleted().unwrap();
        }
        object
    }

    #[tokio::test]
    async fn test_download_by_id_happy_path() {
        // Arrange
        let mut mock_object_repo = MockObjectRepository::new();
        let mut mock_blob_store = MockBlobStore::new();
        let object = create_test_object(ObjectStatus::Committed);
        let object_id = *object.id();

        mock_object_repo
            .expect_find_by_id()
            .withf(move |id| id == &object_id)
            .times(1)
            .returning(move |_| Ok(Some(object.clone())));

        mock_blob_store
            .expect_read()
            .times(1)
            .returning(|_, _| {
                let reader = Box::pin(Cursor::new("test data"));
                Ok(reader)
            });

        let use_case =
            DownloadObjectUseCase::new(Arc::new(mock_object_repo), Arc::new(mock_blob_store));

        // Act
        let result = use_case.execute_by_id(&object_id).await;

        // Assert
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_download_by_id_not_found() {
        // Arrange
        let mut mock_object_repo = MockObjectRepository::new();
        let mock_blob_store = MockBlobStore::new();
        let object_id = ObjectId::new();

        mock_object_repo
            .expect_find_by_id()
            .withf(move |id| id == &object_id)
            .times(1)
            .returning(|_| Ok(None));

        let use_case =
            DownloadObjectUseCase::new(Arc::new(mock_object_repo), Arc::new(mock_blob_store));

        // Act
        let result = use_case.execute_by_id(&object_id).await;

        // Assert
        assert!(matches!(result, Err(DownloadError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_download_by_id_not_readable() {
        // Arrange
        let mut mock_object_repo = MockObjectRepository::new();
        let mock_blob_store = MockBlobStore::new();
        let object = create_test_object(ObjectStatus::Writing);
        let object_id = *object.id();

        mock_object_repo
            .expect_find_by_id()
            .withf(move |id| id == &object_id)
            .times(1)
            .returning(move |_| Ok(Some(object.clone())));

        let use_case =
            DownloadObjectUseCase::new(Arc::new(mock_object_repo), Arc::new(mock_blob_store));

        // Act
        let result = use_case.execute_by_id(&object_id).await;

        // Assert
        assert!(matches!(result, Err(DownloadError::NotReadable(_))));
    }
}
