//! Comprehensive error types for the application
//!
//! This module provides a hierarchical error type system that covers
//! all possible error scenarios in the application, from domain errors
//! to infrastructure failures.

use std::fmt;

/// Main application error type that encompasses all possible errors
#[derive(Debug)]
pub enum AppError {
    /// Domain/business logic errors
    Domain(DomainError),
    /// Infrastructure/storage errors
    Infrastructure(InfrastructureError),
    /// Authentication/authorization errors
    Auth(AuthError),
    /// Validation errors
    Validation(ValidationError),
    /// External service errors
    External(ExternalError),
    /// Configuration errors
    Config(ConfigError),
    /// Unexpected internal errors
    Internal(InternalError),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Domain(err) => write!(f, "Domain error: {}", err),
            AppError::Infrastructure(err) => write!(f, "Infrastructure error: {}", err),
            AppError::Auth(err) => write!(f, "Authentication error: {}", err),
            AppError::Validation(err) => write!(f, "Validation error: {}", err),
            AppError::External(err) => write!(f, "External service error: {}", err),
            AppError::Config(err) => write!(f, "Configuration error: {}", err),
            AppError::Internal(err) => write!(f, "Internal error: {}", err),
        }
    }
}

impl std::error::Error for AppError {}

/// Domain/business logic errors
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Validation error in field '{field}': {message}")]
    Validation { field: String, message: String },

    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: crate::domain::value_objects::ObjectStatus,
        to: crate::domain::value_objects::ObjectStatus,
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

    #[error("Resource not found: {resource_type} with id {id}")]
    NotFound { resource_type: String, id: String },

    #[error("Resource already exists: {resource_type} with id {id}")]
    AlreadyExists { resource_type: String, id: String },

    #[error("Insufficient permissions: {required} required, but {actual} provided")]
    InsufficientPermissions { required: String, actual: String },
}

/// Infrastructure/storage layer errors
#[derive(Debug, thiserror::Error)]
pub enum InfrastructureError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Storage error: {message}")]
    Storage { message: String },

    #[error("Connection error: {message}")]
    Connection { message: String },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File system error: {operation} failed for {path}")]
    FileSystem { operation: String, path: String },

    #[error("Cache error: {message}")]
    Cache { message: String },
}

/// Authentication and authorization errors
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Authentication required")]
    AuthenticationRequired,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token: {reason}")]
    InvalidToken { reason: String },

    #[error("Access forbidden: {reason}")]
    AccessForbidden { reason: String },

    #[error("Rate limit exceeded: {retry_after} seconds")]
    RateLimitExceeded { retry_after: u64 },

    #[error("API key not found")]
    ApiKeyNotFound,

    #[error("API key expired")]
    ApiKeyExpired,

    #[error("API key disabled")]
    ApiKeyDisabled,

    #[error("Tenant suspended")]
    TenantSuspended,
}

/// Input validation errors
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Field '{field}' is required")]
    Required { field: String },

    #[error("Field '{field}' is invalid: {message}")]
    Invalid { field: String, message: String },

    #[error("Field '{field}' exceeds maximum length: {max} characters")]
    TooLong { field: String, max: usize },

    #[error("Field '{field}' is too short: minimum {min} characters")]
    TooShort { field: String, min: usize },

    #[error("Field '{field}' contains invalid characters")]
    InvalidCharacters { field: String },

    #[error("Field '{field}' contains blocked content")]
    BlockedContent { field: String },

    #[error("Invalid format for field '{field}': expected {expected}")]
    InvalidFormat { field: String, expected: String },

    #[error("Value out of range for field '{field}': {value} not in {min}..{max}")]
    OutOfRange {
        field: String,
        value: String,
        min: String,
        max: String,
    },
}

/// External service/API errors
#[derive(Debug, thiserror::Error)]
pub enum ExternalError {
    #[error("HTTP request failed: {status} - {message}")]
    HttpRequestFailed { status: u16, message: String },

    #[error("Network timeout: {operation}")]
    NetworkTimeout { operation: String },

    #[error("External service unavailable: {service}")]
    ServiceUnavailable { service: String },

    #[error("External API rate limit exceeded")]
    ApiRateLimitExceeded,

    #[error("Invalid response from external service: {service}")]
    InvalidResponse { service: String },

    #[error("External service authentication failed")]
    AuthenticationFailed,
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required configuration: {key}")]
    Missing { key: String },

    #[error("Invalid configuration value for {key}: {value} - {reason}")]
    Invalid {
        key: String,
        value: String,
        reason: String,
    },

    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    #[error("Configuration file format error: {path} - {reason}")]
    FileFormatError { path: String, reason: String },

    #[error("Environment variable error: {var} - {reason}")]
    EnvironmentError { var: String, reason: String },
}

/// Internal/unexpected errors
#[derive(Debug, thiserror::Error)]
pub enum InternalError {
    #[error("Unexpected error: {message}")]
    Unexpected { message: String },

    #[error("Programming error: {message}")]
    ProgrammingError { message: String },

    #[error("Data inconsistency: {message}")]
    DataInconsistency { message: String },

    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },

    #[error("Operation timeout: {operation}")]
    OperationTimeout { operation: String },

    #[error("Concurrent modification error")]
    ConcurrentModification,

    #[error("State corruption detected")]
    StateCorruption,
}

// Conversion implementations for easier error handling

impl From<DomainError> for AppError {
    fn from(err: DomainError) -> Self {
        AppError::Domain(err)
    }
}

impl From<InfrastructureError> for AppError {
    fn from(err: InfrastructureError) -> Self {
        AppError::Infrastructure(err)
    }
}

impl From<AuthError> for AppError {
    fn from(err: AuthError) -> Self {
        AppError::Auth(err)
    }
}

impl From<ValidationError> for AppError {
    fn from(err: ValidationError) -> Self {
        AppError::Validation(err)
    }
}

impl From<ExternalError> for AppError {
    fn from(err: ExternalError) -> Self {
        AppError::External(err)
    }
}

impl From<ConfigError> for AppError {
    fn from(err: ConfigError) -> Self {
        AppError::Config(err)
    }
}

impl From<InternalError> for AppError {
    fn from(err: InternalError) -> Self {
        AppError::Internal(err)
    }
}

// HTTP status code mapping for API responses
impl AppError {
    /// Get the appropriate HTTP status code for this error
    pub fn http_status(&self) -> axum::http::StatusCode {
        match self {
            AppError::Domain(err) => match err {
                DomainError::Validation { .. } => axum::http::StatusCode::BAD_REQUEST,
                DomainError::NotFound { .. } => axum::http::StatusCode::NOT_FOUND,
                DomainError::AlreadyExists { .. } => axum::http::StatusCode::CONFLICT,
                DomainError::InsufficientPermissions { .. } => axum::http::StatusCode::FORBIDDEN,
                _ => axum::http::StatusCode::BAD_REQUEST,
            },
            AppError::Infrastructure(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Auth(err) => match err {
                AuthError::AuthenticationRequired => axum::http::StatusCode::UNAUTHORIZED,
                AuthError::AccessForbidden { .. } => axum::http::StatusCode::FORBIDDEN,
                AuthError::RateLimitExceeded { .. } => axum::http::StatusCode::TOO_MANY_REQUESTS,
                _ => axum::http::StatusCode::UNAUTHORIZED,
            },
            AppError::Validation(_) => axum::http::StatusCode::BAD_REQUEST,
            AppError::External(err) => match err {
                ExternalError::HttpRequestFailed { status, .. } => {
                    axum::http::StatusCode::from_u16(*status)
                        .unwrap_or(axum::http::StatusCode::BAD_GATEWAY)
                }
                ExternalError::ServiceUnavailable { .. } => axum::http::StatusCode::BAD_GATEWAY,
                ExternalError::ApiRateLimitExceeded => axum::http::StatusCode::TOO_MANY_REQUESTS,
                _ => axum::http::StatusCode::BAD_GATEWAY,
            },
            AppError::Config(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Internal(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Check if this error should be logged as an error (vs warning/info)
    pub fn should_log_error(&self) -> bool {
        match self {
            AppError::Infrastructure(_) => true,
            AppError::Internal(_) => true,
            AppError::Config(_) => true,
            AppError::External(err) => matches!(err, ExternalError::ServiceUnavailable { .. }),
            _ => false,
        }
    }

    /// Get a safe error message for client responses (no sensitive information)
    pub fn safe_message(&self) -> &str {
        match self {
            AppError::Domain(err) => match err {
                DomainError::NotFound { .. } => "Resource not found",
                DomainError::Validation { .. } => "Validation failed",
                DomainError::AlreadyExists { .. } => "Resource already exists",
                _ => "Bad request",
            },
            AppError::Auth(err) => match err {
                AuthError::AuthenticationRequired => "Authentication required",
                AuthError::AccessForbidden { .. } => "Access forbidden",
                AuthError::RateLimitExceeded { .. } => "Rate limit exceeded",
                _ => "Authentication failed",
            },
            AppError::Validation(_) => "Validation failed",
            AppError::Infrastructure(_) => "Internal server error",
            AppError::External(_) => "External service error",
            AppError::Config(_) => "Configuration error",
            AppError::Internal(_) => "Internal server error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_hierarchy() {
        let domain_err = DomainError::NotFound {
            resource_type: "Object".to_string(),
            id: "123".to_string(),
        };
        let app_err: AppError = domain_err.into();

        assert!(matches!(app_err, AppError::Domain(_)));
        assert_eq!(app_err.http_status(), axum::http::StatusCode::NOT_FOUND);
        assert_eq!(app_err.safe_message(), "Resource not found");
    }

    #[test]
    fn test_http_status_mapping() {
        // Domain errors
        let validation_err = AppError::Domain(DomainError::Validation {
            field: "test".to_string(),
            message: "invalid".to_string(),
        });
        assert_eq!(
            validation_err.http_status(),
            axum::http::StatusCode::BAD_REQUEST
        );

        let not_found_err = AppError::Domain(DomainError::NotFound {
            resource_type: "test".to_string(),
            id: "123".to_string(),
        });
        assert_eq!(
            not_found_err.http_status(),
            axum::http::StatusCode::NOT_FOUND
        );

        // Auth errors
        let auth_err = AppError::Auth(AuthError::AuthenticationRequired);
        assert_eq!(auth_err.http_status(), axum::http::StatusCode::UNAUTHORIZED);

        let rate_limit_err = AppError::Auth(AuthError::RateLimitExceeded { retry_after: 60 });
        assert_eq!(
            rate_limit_err.http_status(),
            axum::http::StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn test_safe_messages() {
        let internal_err =
            AppError::Infrastructure(InfrastructureError::Database(sqlx::Error::RowNotFound));
        assert_eq!(internal_err.safe_message(), "Internal server error");
        assert!(internal_err.should_log_error());

        let client_err = AppError::Validation(ValidationError::Required {
            field: "name".to_string(),
        });
        assert_eq!(client_err.safe_message(), "Validation failed");
        assert!(!client_err.should_log_error());
    }
}
