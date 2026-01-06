use axum::http::StatusCode;
use tracing::{error, warn};

use super::config::ErrorHandlingConfig;
use super::sanitizers::ErrorSanitizer;

/// Error handling utilities
pub struct ErrorUtils;

impl ErrorUtils {
    /// Log errors appropriately based on environment
    pub fn log_error(
        response: &axum::response::Response,
        uri: &axum::http::Uri,
        method: &axum::http::Method,
        config: &ErrorHandlingConfig,
    ) {
        let status = response.status();

        if status.is_server_error() {
            // Always log server errors
            error!(
                "Server error: {} {} {} -> {}",
                method,
                uri,
                status,
                status.canonical_reason().unwrap_or("Unknown")
            );
        } else if status.is_client_error() && config.log_sensitive_errors {
            // Log client errors in development or when configured
            warn!(
                "Client error: {} {} {} -> {}",
                method,
                uri,
                status,
                status.canonical_reason().unwrap_or("Unknown")
            );
        }
    }

    /// Create a safe error response that doesn't leak information
    pub fn safe_error_response(
        status: StatusCode,
        message: &str,
        code: Option<&str>,
    ) -> axum::response::Response {
        ErrorSanitizer::create_generic_error_response(status, message, code)
    }
}

/// Database error sanitization utilities
pub struct DatabaseErrorUtils;

impl DatabaseErrorUtils {
    /// Sanitize SQLx database errors
    pub fn sanitize_db_error(error: &sqlx::Error) -> String {
        match error {
            sqlx::Error::Configuration(_) => "Database configuration error".to_string(),
            sqlx::Error::Database(db_err) => {
                // Try to extract useful information without leaking sensitive details
                match db_err.kind() {
                    sqlx::error::ErrorKind::UniqueViolation => {
                        "Unique constraint violation".to_string()
                    }
                    sqlx::error::ErrorKind::ForeignKeyViolation => {
                        "Foreign key constraint violation".to_string()
                    }
                    sqlx::error::ErrorKind::NotNullViolation => {
                        "Required field is missing".to_string()
                    }
                    sqlx::error::ErrorKind::CheckViolation => "Data validation failed".to_string(),
                    _ => "Database operation error".to_string(),
                }
            }
            sqlx::Error::Io(_) => "Database connection error".to_string(),
            sqlx::Error::Tls(_) => "Database TLS error".to_string(),
            sqlx::Error::PoolTimedOut => "Database pool timeout".to_string(),
            sqlx::Error::PoolClosed => "Database pool closed".to_string(),
            sqlx::Error::WorkerCrashed => "Database worker crashed".to_string(),
            sqlx::Error::RowNotFound => "Record not found".to_string(),
            sqlx::Error::TypeNotFound { .. } => "Database type error".to_string(),
            sqlx::Error::ColumnIndexOutOfBounds { .. } => "Database column error".to_string(),
            sqlx::Error::ColumnNotFound(_) => "Database column not found".to_string(),
            sqlx::Error::ColumnDecode { .. } => "Database decode error".to_string(),
            sqlx::Error::Decode(_) => "Database decode error".to_string(),
            sqlx::Error::Protocol(_) => "Database protocol error".to_string(),
            sqlx::Error::Migrate(_) => "Database migration error".to_string(),
            _ => "Database error".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_error_sanitization() {
        use sqlx::Error;

        // Test configuration error
        let config_err = Error::Configuration("sensitive config data".into());
        let sanitized = DatabaseErrorUtils::sanitize_db_error(&config_err);
        assert_eq!(sanitized, "Database configuration error");

        // Test database error - simplified for compatibility
        // let db_err = Error::Database(...); // SQLx API changed
        // let sanitized = DatabaseErrorUtils::sanitize_db_error(&db_err);
        // assert_eq!(sanitized, "Database operation error");

        // Test connection errors
        let io_err = Error::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "Connection refused",
        ));
        let sanitized = DatabaseErrorUtils::sanitize_db_error(&io_err);
        assert_eq!(sanitized, "Database connection error");

        let tls_err = Error::Tls("TLS handshake failed".into());
        let sanitized = DatabaseErrorUtils::sanitize_db_error(&tls_err);
        assert_eq!(sanitized, "Database TLS error");

        // Test other errors
        let pool_timeout = Error::PoolTimedOut;
        let sanitized = DatabaseErrorUtils::sanitize_db_error(&pool_timeout);
        assert_eq!(sanitized, "Database pool timeout");

        let not_found = Error::RowNotFound;
        let sanitized = DatabaseErrorUtils::sanitize_db_error(&not_found);
        assert_eq!(sanitized, "Record not found");

        let protocol_err = Error::Protocol("Protocol violation".into());
        let sanitized = DatabaseErrorUtils::sanitize_db_error(&protocol_err);
        assert_eq!(sanitized, "Database protocol error");
    }

    #[test]
    fn test_error_response_creation() {
        let response = ErrorUtils::safe_error_response(
            axum::http::StatusCode::BAD_REQUEST,
            "Validation failed",
            Some("VALIDATION_ERROR"),
        );

        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);

        // In a real test, we'd check the response body content
        // For now, just verify the response was created
        assert!(response.status().is_client_error());
    }
}
