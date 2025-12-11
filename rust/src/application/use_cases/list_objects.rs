use std::sync::Arc;
use thiserror::Error;

use crate::application::dto::{ListRequest, ListResponse, ObjectDto};
use crate::application::ports::{ObjectRepository, RepositoryError};
use crate::domain::errors::DomainError;
use crate::domain::value_objects::{Namespace, TenantId};

#[derive(Debug, Error)]
pub enum ListError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// Use case: List objects
pub struct ListObjectsUseCase {
    object_repo: Arc<dyn ObjectRepository>,
}

impl ListObjectsUseCase {
    pub fn new(object_repo: Arc<dyn ObjectRepository>) -> Self {
        Self { object_repo }
    }

    /// Execute list with pagination
    pub async fn execute(&self, request: ListRequest) -> Result<ListResponse, ListError> {
        // 1. Parse and validate
        let namespace = Namespace::new(request.namespace)
            .map_err(|e| ListError::InvalidRequest(e.to_string()))?;

        let tenant_id = TenantId::from_string(&request.tenant_id)
            .map_err(|e| ListError::InvalidRequest(e.to_string()))?;

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
