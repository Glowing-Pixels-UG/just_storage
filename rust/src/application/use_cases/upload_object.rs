use std::sync::Arc;
use thiserror::Error;

use crate::application::dto::{ObjectDto, UploadRequest};
use crate::application::ports::{
    BlobReader, BlobRepository, BlobStore, ObjectRepository, RepositoryError, StorageError,
};
use crate::domain::entities::Object;
use crate::domain::errors::DomainError;
use crate::domain::value_objects::{Namespace, TenantId};

#[derive(Debug, Error)]
pub enum UploadError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// Use case: Upload an object
pub struct UploadObjectUseCase {
    object_repo: Arc<dyn ObjectRepository>,
    blob_repo: Arc<dyn BlobRepository>,
    blob_store: Arc<dyn BlobStore>,
}

impl UploadObjectUseCase {
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

    /// Execute upload workflow
    pub async fn execute(
        &self,
        request: UploadRequest,
        reader: BlobReader,
    ) -> Result<ObjectDto, UploadError> {
        // 1. Parse and validate request
        let namespace = Namespace::new(request.namespace)
            .map_err(|e| UploadError::InvalidRequest(e.to_string()))?;

        let tenant_id = TenantId::from_string(&request.tenant_id)
            .map_err(|e| UploadError::InvalidRequest(e.to_string()))?;

        let storage_class = request.storage_class.unwrap_or_default();

        // 2. Create domain entity in WRITING state
        let mut object = Object::new(namespace, tenant_id, request.key, storage_class);

        // 3. Reserve in DB (status=WRITING)
        self.object_repo.save(&object).await?;

        // 4. Write blob to storage (computes hash during write)
        let (content_hash, size_bytes) = self.blob_store.write(reader, storage_class).await?;

        // 5. Get or create blob entry with ref counting
        self.blob_repo
            .get_or_create(&content_hash, storage_class, size_bytes)
            .await?;

        // 6. Commit: update object state to COMMITTED
        object.commit(content_hash, size_bytes)?;
        self.object_repo.save(&object).await?;

        // 7. Return DTO
        Ok(ObjectDto::from(object))
    }
}
