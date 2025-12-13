use async_trait::async_trait;
use thiserror::Error;

use crate::application::dto::{SearchRequest, TextSearchRequest};
use crate::domain::entities::Object;
use crate::domain::value_objects::{Namespace, ObjectId, TenantId};
#[cfg(test)]
use mockall::{automock, predicate::*};

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("Object not found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Port for object persistence operations
#[cfg_attr(test, automock)]
#[async_trait]
pub trait ObjectRepository: Send + Sync {
    /// Save or update an object
    async fn save(&self, object: &Object) -> Result<(), RepositoryError>;

    /// Find object by ID (only COMMITTED objects)
    async fn find_by_id(&self, id: &ObjectId) -> Result<Option<Object>, RepositoryError>;

    /// Find object by key (namespace + tenant + key)
    async fn find_by_key(
        &self,
        namespace: &Namespace,
        tenant_id: &TenantId,
        key: &str,
    ) -> Result<Option<Object>, RepositoryError>;

    /// List objects with pagination
    async fn list(
        &self,
        namespace: &Namespace,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Object>, RepositoryError>;

    /// Advanced search with filters
    async fn search(&self, request: &SearchRequest) -> Result<Vec<Object>, RepositoryError>;

    /// Full-text search across metadata and keys
    async fn text_search(
        &self,
        request: &TextSearchRequest,
    ) -> Result<Vec<Object>, RepositoryError>;

    /// Delete object (hard delete from DB)
    async fn delete(&self, id: &ObjectId) -> Result<(), RepositoryError>;
}
