use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Input sanitization configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputSanitizationConfig {
    /// Maximum string length for text inputs
    pub max_string_length: usize,
    /// Maximum depth for nested structures
    pub max_depth: usize,
    /// Whether to remove null bytes
    pub remove_null_bytes: bool,
    /// Whether to normalize Unicode
    pub normalize_unicode: bool,
    /// Custom patterns to block
    pub blocked_patterns: HashSet<String>,
    /// Allowed characters for identifiers (regex)
    pub allowed_identifier_chars: String,
}

impl Default for InputSanitizationConfig {
    fn default() -> Self {
        let mut blocked_patterns = HashSet::new();
        blocked_patterns.insert("<script".to_string());
        blocked_patterns.insert("javascript:".to_string());
        blocked_patterns.insert("vbscript:".to_string());
        blocked_patterns.insert("data:".to_string());
        blocked_patterns.insert("onload=".to_string());
        blocked_patterns.insert("onerror=".to_string());
        blocked_patterns.insert("eval(".to_string());
        blocked_patterns.insert("../".to_string());
        blocked_patterns.insert("..\\".to_string());

        Self {
            max_string_length: 10000, // 10KB max for text inputs
            max_depth: 10,
            remove_null_bytes: true,
            normalize_unicode: true,
            blocked_patterns,
            allowed_identifier_chars: r"^[a-zA-Z0-9_-]+$".to_string(),
        }
    }
}

impl InputSanitizationConfig {
    /// Create a new config with custom settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum string length
    pub fn with_max_string_length(mut self, length: usize) -> Self {
        self.max_string_length = length;
        self
    }

    /// Set maximum depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Enable/disable null byte removal
    pub fn with_null_byte_removal(mut self, enabled: bool) -> Self {
        self.remove_null_bytes = enabled;
        self
    }

    /// Enable/disable Unicode normalization
    pub fn with_unicode_normalization(mut self, enabled: bool) -> Self {
        self.normalize_unicode = enabled;
        self
    }

    /// Add a blocked pattern
    pub fn with_blocked_pattern(mut self, pattern: String) -> Self {
        self.blocked_patterns.insert(pattern);
        self
    }

    /// Set allowed identifier regex
    pub fn with_identifier_regex(mut self, regex: String) -> Self {
        self.allowed_identifier_chars = regex;
        self
    }
}
