//! Error handling middleware for sanitizing error responses
//!
//! This module provides comprehensive error handling and sanitization
//! to prevent information leakage in error responses.
//!
//! The module is split into focused components:
//! - config.rs: Configuration structures
//! - middleware.rs: HTTP middleware implementation
//! - sanitizers.rs: Error response sanitization logic
//! - utils.rs: Error handling utilities and database error sanitization

pub mod config;
pub mod middleware;
pub mod sanitizers;
pub mod utils;

// Re-export main types for convenience
pub use config::ErrorHandlingConfig;
pub use middleware::{create_error_handling_middleware, ErrorHandlingLayer};
pub use sanitizers::ErrorSanitizer;
pub use utils::{DatabaseErrorUtils, ErrorUtils};

// Re-export utility functions for backward compatibility
pub use utils::ErrorUtils as ErrorHandler;

/// Utility functions for error handling
///
/// These functions provide a convenient interface for common error handling tasks.
/// Create a safe error response that doesn't leak information
pub fn safe_error_response(
    status: axum::http::StatusCode,
    message: &str,
    code: Option<&str>,
) -> axum::response::Response {
    ErrorUtils::safe_error_response(status, message, code)
}

/// Sanitize an error message by removing sensitive information
pub fn sanitize_error_message(message: &str) -> String {
    let config = ErrorHandlingConfig::default();
    ErrorSanitizer::sanitize_error_message(message, &config)
}

/// Sanitize database errors
pub fn sanitize_db_error(error: &sqlx::Error) -> String {
    DatabaseErrorUtils::sanitize_db_error(error)
}
