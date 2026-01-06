//! Input sanitization middleware for security and data validation
//!
//! This module provides comprehensive input sanitization and validation
//! to protect against common web vulnerabilities like XSS, injection attacks,
//! and malformed data.

pub mod config;
pub mod middleware;
pub mod sanitizers;
pub mod validators;

// Re-export main types for convenience
pub use config::InputSanitizationConfig;
pub use middleware::create_input_sanitization_middleware;
pub use sanitizers::Sanitizer;
pub use validators::Validator;

// Re-export utility functions for backward compatibility
pub use validators::Validator as InputValidator;

/// Utility functions for input validation and sanitization
///
/// These functions provide a convenient interface for common validation tasks.
/// They use the default configuration but can be customized as needed.
/// Validate and sanitize a string input
pub fn validate_and_sanitize_string(
    input: &str,
    field_name: &str,
    max_length: Option<usize>,
) -> Result<String, String> {
    let config = InputSanitizationConfig::default();
    Validator::validate_and_sanitize_string(input, field_name, max_length, &config)
}

/// Validate an identifier (like tenant_id, object key, etc.)
pub fn validate_identifier(input: &str, field_name: &str) -> Result<(), String> {
    let config = InputSanitizationConfig::default();
    Validator::validate_identifier(input, field_name, &config)
}

/// Validate a tenant ID specifically
pub fn validate_tenant_id(tenant_id: &str) -> Result<(), String> {
    let config = InputSanitizationConfig::default();
    Validator::validate_tenant_id(tenant_id, &config)
}

/// Sanitize SQL-like inputs (basic protection against SQL injection in text fields)
pub fn sanitize_sql_input(input: &str) -> String {
    Sanitizer::sanitize_sql_input(input)
}

/// Validate file upload metadata
pub fn validate_file_upload(
    filename: &str,
    content_type: Option<&str>,
    size: u64,
) -> Result<(), String> {
    let config = InputSanitizationConfig::default();
    Validator::validate_file_upload(filename, content_type, size, &config)
}
