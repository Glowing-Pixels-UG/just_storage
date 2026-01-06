use super::config::ErrorHandlingConfig;
use serde_json::Value;

/// Error sanitization utilities
pub struct ErrorSanitizer;

impl ErrorSanitizer {
    /// Sanitize a JSON value by removing sensitive information
    pub fn sanitize_json_value(value: &mut Value, config: &ErrorHandlingConfig) {
        match value {
            Value::Object(obj) => {
                // Remove sensitive keys
                let sensitive_keys: Vec<String> = obj
                    .keys()
                    .filter(|key| Self::is_sensitive_key(key, &config.sensitive_patterns))
                    .cloned()
                    .collect();

                for key in sensitive_keys {
                    obj.remove(&key);
                }

                // Recursively sanitize nested objects
                for (_, val) in obj.iter_mut() {
                    Self::sanitize_json_value(val, config);
                }
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    Self::sanitize_json_value(item, config);
                }
            }
            _ => {} // Primitives don't need sanitization
        }
    }

    /// Check if a key contains sensitive information
    pub fn is_sensitive_key(
        key: &str,
        sensitive_patterns: &std::collections::HashSet<String>,
    ) -> bool {
        let key_lower = key.to_lowercase();
        let default_patterns = [
            "password",
            "secret",
            "key",
            "token",
            "database",
            "connection",
            "sql",
            "stack",
            "trace",
            "error",
            "internal",
        ];

        // Check custom patterns
        if sensitive_patterns
            .iter()
            .any(|pattern| key_lower.contains(pattern))
        {
            return true;
        }

        // Check default patterns
        default_patterns
            .iter()
            .any(|pattern| key_lower.contains(pattern))
    }

    /// Sanitize an error message by removing sensitive information
    pub fn sanitize_error_message(message: &str, config: &ErrorHandlingConfig) -> String {
        // Check if the message contains sensitive patterns
        for pattern in &config.sensitive_patterns {
            if message.to_lowercase().contains(pattern) {
                return "An error occurred".to_string();
            }
        }

        // If no sensitive patterns found, return the original message
        // but limit length to prevent very long error messages
        if message.len() > 200 {
            format!("{}...", &message[..200])
        } else {
            message.to_string()
        }
    }

    /// Create a generic error response that doesn't leak information
    pub fn create_generic_error_response(
        status: axum::http::StatusCode,
        message: &str,
        code: Option<&str>,
    ) -> axum::response::Response {
        use axum::{response::IntoResponse, Json};
        use serde::Serialize;

        #[derive(Serialize)]
        struct SanitizedErrorResponse {
            error: String,
            code: Option<String>,
            details: Option<String>,
        }

        let error_response = SanitizedErrorResponse {
            error: message.to_string(),
            code: code.map(|s| s.to_string()),
            details: None, // Never include details in production
        };

        (status, Json(error_response)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sensitive_key_detection() {
        let config = ErrorHandlingConfig::default();

        assert!(ErrorSanitizer::is_sensitive_key(
            "password",
            &config.sensitive_patterns
        ));
        assert!(ErrorSanitizer::is_sensitive_key(
            "api_key",
            &config.sensitive_patterns
        ));
        assert!(ErrorSanitizer::is_sensitive_key(
            "database_url",
            &config.sensitive_patterns
        ));
        assert!(ErrorSanitizer::is_sensitive_key(
            "error_stack",
            &config.sensitive_patterns
        ));
        assert!(!ErrorSanitizer::is_sensitive_key(
            "name",
            &config.sensitive_patterns
        ));
        assert!(!ErrorSanitizer::is_sensitive_key(
            "email",
            &config.sensitive_patterns
        ));
    }

    #[test]
    fn test_json_value_sanitization() {
        let config = ErrorHandlingConfig::default();
        let mut value = json!({
            "name": "John",
            "password": "secret123",
            "api_key": "key123",
            "nested": {
                "token": "token456",
                "normal_field": "value"
            },
            "array": [
                {"password": "pass1"},
                {"normal": "value"}
            ]
        });

        ErrorSanitizer::sanitize_json_value(&mut value, &config);

        // Sensitive fields should be removed
        assert!(value.get("password").is_none());
        assert!(value.get("api_key").is_none());

        // Normal fields should remain
        assert_eq!(value.get("name").unwrap(), "John");

        // Nested sensitive fields should be removed
        assert!(value.pointer("/nested/token").is_none());
        assert_eq!(value.pointer("/nested/normal_field").unwrap(), "value");

        // Array items should be sanitized
        assert!(value.pointer("/array/0/password").is_none());
        assert_eq!(value.pointer("/array/1/normal").unwrap(), "value");
    }

    #[test]
    fn test_error_message_sanitization() {
        let config = ErrorHandlingConfig::default();

        // Sensitive messages should be sanitized
        assert_eq!(
            ErrorSanitizer::sanitize_error_message(
                "Database connection failed: postgres://user:pass@host/db",
                &config
            ),
            "An error occurred"
        );

        // Normal messages should be preserved
        assert_eq!(
            ErrorSanitizer::sanitize_error_message("Validation failed", &config),
            "Validation failed"
        );

        // Long messages should be truncated
        let long_message = "a".repeat(300);
        let sanitized = ErrorSanitizer::sanitize_error_message(&long_message, &config);
        assert!(sanitized.len() <= 203); // 200 chars + "..."
        assert!(sanitized.ends_with("..."));
    }
}
