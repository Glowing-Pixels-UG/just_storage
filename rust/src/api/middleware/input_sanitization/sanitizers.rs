use super::config::InputSanitizationConfig;

/// Core sanitization functions
pub struct Sanitizer;

impl Sanitizer {
    /// Sanitize a string according to configuration
    pub fn sanitize_string(input: &str, config: &InputSanitizationConfig) -> String {
        let mut result = input.to_string();

        // Truncate if too long
        if result.len() > config.max_string_length {
            result = result[..config.max_string_length].to_string();
        }

        // Remove null bytes if configured
        if config.remove_null_bytes {
            result = result.replace('\0', "");
        }

        // Normalize Unicode if configured
        if config.normalize_unicode {
            // Basic normalization - remove control characters except common whitespace
            result = result
                .chars()
                .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
                .collect();
        }

        // HTML entity encoding for common dangerous characters
        result = result.replace('<', "&lt;");
        result = result.replace('>', "&gt;");
        result = result.replace('&', "&amp;");
        result = result.replace('"', "&quot;");
        result = result.replace('\'', "&#x27;");

        result
    }

    /// Check if string contains blocked patterns
    pub fn contains_blocked_patterns(
        input: &str,
        blocked_patterns: &std::collections::HashSet<String>,
    ) -> bool {
        let input_lower = input.to_lowercase();
        blocked_patterns
            .iter()
            .any(|pattern| input_lower.contains(pattern))
    }

    /// Sanitize SQL-like inputs (basic protection against SQL injection in text fields)
    pub fn sanitize_sql_input(input: &str) -> String {
        // This is a very basic sanitization - in production, you'd use prepared statements
        // and proper ORM handling. This is just an additional layer of defense.
        input
            .replace('\'', "''") // Escape single quotes for SQL
            .replace('\\', "\\\\") // Escape backslashes
            .replace('\0', "") // Remove null bytes
    }
}
