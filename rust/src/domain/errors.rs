use thiserror::Error;

use super::value_objects::ObjectStatus;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: ObjectStatus,
        to: ObjectStatus,
    },

    #[error("Cannot delete object in non-committed state")]
    CannotDeleteNonCommitted,

    #[error("Object already committed")]
    AlreadyCommitted,

    #[error("Invalid namespace: {0}")]
    InvalidNamespace(String),

    #[error("Invalid tenant ID: {0}")]
    InvalidTenantId(String),

    #[error("Content hash mismatch: expected {expected}, got {actual}")]
    ContentHashMismatch { expected: String, actual: String },

    #[error("Object size exceeds maximum allowed: {size} > {max}")]
    SizeExceedsMaximum { size: u64, max: u64 },
}
