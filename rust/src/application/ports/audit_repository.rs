use crate::api::middleware::audit::AuditLogEntry;
use async_trait::async_trait;

/// Repository for storing and retrieving audit logs
#[async_trait]
pub trait AuditRepository: Send + Sync {
    /// Store an audit log entry
    async fn store(&self, entry: AuditLogEntry) -> Result<(), AuditRepositoryError>;

    /// Query audit logs with pagination
    async fn query(
        &self,
        filter: AuditQueryFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLogEntry>, AuditRepositoryError>;

    /// Count audit logs matching a filter
    async fn count(&self, filter: AuditQueryFilter) -> Result<i64, AuditRepositoryError>;

    /// Clean up old audit logs (retention policy)
    async fn cleanup_old_logs(&self, retention_days: i32) -> Result<i64, AuditRepositoryError>;
}

/// Filter for querying audit logs
#[derive(Debug, Clone, Default)]
pub struct AuditQueryFilter {
    pub event_types: Option<Vec<String>>,
    pub user_id: Option<String>,
    pub tenant_id: Option<String>,
    pub api_key_id: Option<String>,
    pub ip_address: Option<String>,
    pub path_pattern: Option<String>,
    pub status_code_min: Option<i32>,
    pub status_code_max: Option<i32>,
    pub from_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub to_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub has_error: Option<bool>,
}

/// Error type for audit repository operations
#[derive(Debug, thiserror::Error)]
pub enum AuditRepositoryError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid filter: {0}")]
    InvalidFilter(String),
}
