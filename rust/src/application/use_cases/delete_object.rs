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
