use super::config::InputSanitizationConfig;
use super::sanitizers::Sanitizer;
use once_cell::sync::Lazy;
use regex::Regex;

/// Cached regex pattern for identifier validation
static IDENTIFIER_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_-]+$").expect("Invalid regex pattern for identifiers"));

/// Input validation utilities
pub struct Validator;

impl Validator {
    /// Validate and sanitize a string input
    pub fn validate_and_sanitize_string(
        input: &str,
        field_name: &str,
        max_length: Option<usize>,
        config: &InputSanitizationConfig,
    ) -> Result<String, String> {
        // Check length
        let max_len = max_length.unwrap_or(config.max_string_length);
        if input.len() > max_len {
            return Err(format!(
                "{} exceeds maximum length of {} characters",
                field_name, max_len
            ));
        }

        // Check for blocked patterns
        if Sanitizer::contains_blocked_patterns(input, &config.blocked_patterns) {
            return Err(format!("{} contains invalid content", field_name));
        }

        // Sanitize the input
        let sanitized = Sanitizer::sanitize_string(input, config);

        Ok(sanitized)
    }

    /// Validate an identifier (like tenant_id, object key, etc.)
    pub fn validate_identifier(
        input: &str,
        field_name: &str,
        config: &InputSanitizationConfig,
    ) -> Result<(), String> {
        if input.is_empty() {
            return Err(format!("{} cannot be empty", field_name));
        }

        if input.len() > 100 {
            // Reasonable max length for identifiers
            return Err(format!("{} is too long (max 100 characters)", field_name));
        }

        if !Self::is_valid_identifier(input, &config.allowed_identifier_chars) {
            return Err(format!("{} contains invalid characters", field_name));
        }

        if Sanitizer::contains_blocked_patterns(input, &config.blocked_patterns) {
            return Err(format!("{} contains blocked content", field_name));
        }

        Ok(())
    }

    /// Validate a tenant ID specifically
    pub fn validate_tenant_id(
        tenant_id: &str,
        config: &InputSanitizationConfig,
    ) -> Result<(), String> {
        Self::validate_identifier(tenant_id, "tenant_id", config)?;

        // Additional tenant ID specific validation
        if tenant_id.len() < 3 {
            return Err("tenant_id must be at least 3 characters long".to_string());
        }

        // Check for reserved tenant IDs
        let reserved = ["admin", "system", "root", "superuser"];
        if reserved.contains(&tenant_id.to_lowercase().as_str()) {
            return Err("tenant_id uses a reserved name".to_string());
        }

        Ok(())
    }

    /// Validate file upload metadata
    pub fn validate_file_upload(
        filename: &str,
        content_type: Option<&str>,
        size: u64,
        config: &InputSanitizationConfig,
    ) -> Result<(), String> {
        // Validate filename
        Self::validate_identifier(filename, "filename", config)?;

        // Check file extension (basic)
        let blocked_extensions = [".exe", ".bat", ".cmd", ".scr", ".pif", ".com"];
        if let Some(ext) = filename.rsplit('.').next() {
            if blocked_extensions.contains(&format!(".{}", ext.to_lowercase()).as_str()) {
                return Err("File type not allowed".to_string());
            }
        }

        // Validate content type if provided
        if let Some(ct) = content_type {
            let blocked_types = ["application/x-executable", "application/x-msdownload"];
            if blocked_types.contains(&ct.to_lowercase().as_str()) {
                return Err("Content type not allowed".to_string());
            }
        }

        // Check file size (100MB limit)
        if size > 100 * 1024 * 1024 {
            return Err("File too large (max 100MB)".to_string());
        }

        Ok(())
    }

    /// Check if string is a valid identifier according to regex pattern
    fn is_valid_identifier(input: &str, pattern: &str) -> bool {
        // Try to use the cached regex if it matches the default pattern
        if pattern == r"^[a-zA-Z0-9_-]+$" {
            IDENTIFIER_REGEX.is_match(input)
        } else {
            // For custom patterns, compile on demand (less common case)
            if let Ok(regex) = Regex::new(pattern) {
                regex.is_match(input)
            } else {
                // If regex is invalid, allow the input (fail-safe)
                true
            }
        }
    }
}
