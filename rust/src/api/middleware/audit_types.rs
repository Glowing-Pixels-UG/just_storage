//! Audit types and data structures
//!
//! This module contains the core types and enums used
//! for audit logging functionality.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Audit event types for security monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    // Authentication events
    AuthenticationSuccess,
    AuthenticationFailure,
    ApiKeyUsed,
    ApiKeyExpired,
    ApiKeyRevoked,

    // Authorization events
    AuthorizationGranted,
    AuthorizationDenied,
    PermissionChecked,

    // Resource access events
    ObjectCreated,
    ObjectRead,
    ObjectUpdated,
    ObjectDeleted,
    ApiKeyCreated,
    ApiKeyUpdated,
    ApiKeyDeleted,

    // Security events
    RateLimitExceeded,
    SuspiciousRequest,
    InvalidInput,
    CorsViolation,

    // System events
    HealthCheck,
    ConfigurationChange,
    BackupOperation,
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditEventType::AuthenticationSuccess => write!(f, "authentication_success"),
            AuditEventType::AuthenticationFailure => write!(f, "authentication_failure"),
            AuditEventType::ApiKeyUsed => write!(f, "api_key_used"),
            AuditEventType::ApiKeyExpired => write!(f, "api_key_expired"),
            AuditEventType::ApiKeyRevoked => write!(f, "api_key_revoked"),
            AuditEventType::AuthorizationGranted => write!(f, "authorization_granted"),
            AuditEventType::AuthorizationDenied => write!(f, "authorization_denied"),
            AuditEventType::PermissionChecked => write!(f, "permission_checked"),
            AuditEventType::ObjectCreated => write!(f, "object_created"),
            AuditEventType::ObjectRead => write!(f, "object_read"),
            AuditEventType::ObjectUpdated => write!(f, "object_updated"),
            AuditEventType::ObjectDeleted => write!(f, "object_deleted"),
            AuditEventType::ApiKeyCreated => write!(f, "api_key_created"),
            AuditEventType::ApiKeyUpdated => write!(f, "api_key_updated"),
            AuditEventType::ApiKeyDeleted => write!(f, "api_key_deleted"),
            AuditEventType::RateLimitExceeded => write!(f, "rate_limit_exceeded"),
            AuditEventType::SuspiciousRequest => write!(f, "suspicious_request"),
            AuditEventType::InvalidInput => write!(f, "invalid_input"),
            AuditEventType::CorsViolation => write!(f, "cors_violation"),
            AuditEventType::HealthCheck => write!(f, "health_check"),
            AuditEventType::ConfigurationChange => write!(f, "configuration_change"),
            AuditEventType::BackupOperation => write!(f, "backup_operation"),
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub user_id: Option<String>,
    pub tenant_id: Option<String>,
    pub api_key_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub status_code: Option<u16>,
    pub response_time_ms: Option<u128>,
    pub error_message: Option<String>,
    pub additional_data: Option<serde_json::Value>,
}

/// Audit logger trait for pluggable logging backends
#[async_trait]
pub trait AuditLogger: Send + Sync {
    async fn log_event(&self, entry: AuditLogEntry) -> Result<(), AuditError>;
}

/// Audit logger error
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
