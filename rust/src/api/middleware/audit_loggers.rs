//! Audit logger implementations
//!
//! This module contains the concrete implementations of
//! audit loggers for different backends.

use async_trait::async_trait;
use tracing::{error, info, warn};

use super::audit_types::{AuditError, AuditLogEntry, AuditLogger};

/// Console audit logger (for development)
pub struct ConsoleAuditLogger;

#[async_trait]
impl AuditLogger for ConsoleAuditLogger {
    async fn log_event(&self, entry: AuditLogEntry) -> Result<(), AuditError> {
        let level = match entry.event_type {
            super::audit_types::AuditEventType::AuthenticationFailure
            | super::audit_types::AuditEventType::AuthorizationDenied
            | super::audit_types::AuditEventType::RateLimitExceeded
            | super::audit_types::AuditEventType::SuspiciousRequest => tracing::Level::WARN,
            super::audit_types::AuditEventType::InvalidInput
            | super::audit_types::AuditEventType::CorsViolation => tracing::Level::ERROR,
            _ => tracing::Level::INFO,
        };

        match level {
            tracing::Level::ERROR => error!("AUDIT: {:?}", entry),
            tracing::Level::WARN => warn!("AUDIT: {:?}", entry),
            _ => info!("AUDIT: {:?}", entry),
        }

        Ok(())
    }
}

/// Database audit logger (for production)
pub struct DatabaseAuditLogger {
    repository: std::sync::Arc<dyn crate::application::ports::AuditRepository>,
}

impl DatabaseAuditLogger {
    pub fn new(repository: std::sync::Arc<dyn crate::application::ports::AuditRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl AuditLogger for DatabaseAuditLogger {
    async fn log_event(&self, entry: AuditLogEntry) -> Result<(), AuditError> {
        // Try to store in database, but don't fail the request if it fails
        // Audit logging should not block normal operation
        match self.repository.store(entry.clone()).await {
            Ok(()) => Ok(()),
            Err(e) => {
                // Log the error and fall back to console logging
                tracing::error!("Failed to store audit log in database: {}", e);
                ConsoleAuditLogger.log_event(entry).await
            }
        }
    }
}
