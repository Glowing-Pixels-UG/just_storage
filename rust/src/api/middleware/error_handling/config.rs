use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Error handling configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorHandlingConfig {
    /// Whether to include stack traces in development
    pub include_debug_info: bool,
    /// Whether to log sensitive error details
    pub log_sensitive_errors: bool,
    /// Sensitive error patterns to redact
    pub sensitive_patterns: HashSet<String>,
}

impl Default for ErrorHandlingConfig {
    fn default() -> Self {
        let mut sensitive_patterns = HashSet::new();
        sensitive_patterns.insert("password".to_string());
        sensitive_patterns.insert("secret".to_string());
        sensitive_patterns.insert("key".to_string());
        sensitive_patterns.insert("token".to_string());
        sensitive_patterns.insert("database".to_string());
        sensitive_patterns.insert("connection".to_string());
        sensitive_patterns.insert("sql".to_string());

        Self {
            include_debug_info: cfg!(debug_assertions),
            log_sensitive_errors: true,
            sensitive_patterns,
        }
    }
}

impl ErrorHandlingConfig {
    /// Create a new config with custom settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable/disable debug info inclusion
    pub fn with_debug_info(mut self, enabled: bool) -> Self {
        self.include_debug_info = enabled;
        self
    }

    /// Enable/disable sensitive error logging
    pub fn with_sensitive_logging(mut self, enabled: bool) -> Self {
        self.log_sensitive_errors = enabled;
        self
    }

    /// Add a sensitive pattern
    pub fn with_sensitive_pattern(mut self, pattern: String) -> Self {
        self.sensitive_patterns.insert(pattern);
        self
    }
}
