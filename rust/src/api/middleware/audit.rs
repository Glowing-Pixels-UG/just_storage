// Re-export types and functions from the split modules for backward compatibility

pub use super::audit_config::AuditConfig;
pub use super::audit_loggers::{ConsoleAuditLogger, DatabaseAuditLogger};
pub use super::audit_middleware::AuditMiddleware;
pub use super::audit_types::{AuditError, AuditEventType, AuditLogEntry, AuditLogger};
