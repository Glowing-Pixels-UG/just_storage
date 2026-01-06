//! Tests for input sanitization modules

mod sanitizers_tests {
    use super::super::sanitizers::Sanitizer;
    use super::super::config::InputSanitizationConfig;
    use std::collections::HashSet;

    #[test]
    fn test_string_sanitization() {
        let config = InputSanitizationConfig::default();

        // Test HTML sanitization
        let input = "<script>alert('xss')</script>";
        let sanitized = Sanitizer::sanitize_string(input, &config);
        assert!(sanitized.contains("&lt;"));
        assert!(sanitized.contains("&gt;"));

        // Test null byte removal
        let input_with_null = "test\0string";
        let sanitized = Sanitizer::sanitize_string(input_with_null, &config);
        assert!(!sanitized.contains('\0'));
    }

    #[test]
    fn test_blocked_patterns() {
        let mut blocked = HashSet::new();
        blocked.insert("<script".to_string());
        blocked.insert("../".to_string());

        assert!(Sanitizer::contains_blocked_patterns("<script>alert(1)</script>", &blocked));
        assert!(Sanitizer::contains_blocked_patterns("../../../etc/passwd", &blocked));
        assert!(!Sanitizer::contains_blocked_patterns("normal text", &blocked));
    }

    #[test]
    fn test_sql_sanitization() {
        let input = "user's input\\with'quotes";
        let sanitized = Sanitizer::sanitize_sql_input(input);
        assert_eq!(sanitized, "user''s input\\\\with''quotes");
    }

    #[test]
    fn test_max_length_truncation() {
        let config = InputSanitizationConfig::default();
        let long_input = "a".repeat(20000);
        let sanitized = Sanitizer::sanitize_string(&long_input, &config);
        assert_eq!(sanitized.len(), config.max_string_length);
    }

    #[test]
    fn test_unicode_normalization() {
        let config = InputSanitizationConfig::default();

        // Test control character removal (except common whitespace)
        let input_with_control = "test\x00\x01\x02string\n\t";
        let sanitized = Sanitizer::sanitize_string(input_with_control, &config);

        // Should remove null bytes and other control chars, but keep \n and \t
        assert!(!sanitized.contains('\0'));
        assert!(!sanitized.contains('\x01'));
        assert!(sanitized.contains('\n'));
        assert!(sanitized.contains('\t'));
    }
}

mod validators_tests {
    use super::super::{validators::Validator, config::InputSanitizationConfig};

    #[test]
    fn test_identifier_validation() {
        let config = InputSanitizationConfig::default();

        assert!(Validator::validate_identifier("valid_name_123", "field", &config).is_ok());
        assert!(Validator::validate_identifier("invalid name!", "field", &config).is_err());
        assert!(Validator::validate_identifier("", "field", &config).is_err());
        assert!(Validator::validate_identifier(&"a".repeat(300), "field", &config).is_err());
    }

    #[test]
    fn test_tenant_id_validation() {
        let config = InputSanitizationConfig::default();

        assert!(Validator::validate_tenant_id("valid-tenant_123", &config).is_ok());
        assert!(Validator::validate_tenant_id("ab", &config).is_err()); // Too short
        assert!(Validator::validate_tenant_id("admin", &config).is_err()); // Reserved
        assert!(Validator::validate_tenant_id("invalid tenant!", &config).is_err()); // Invalid chars
    }

    #[test]
    fn test_file_upload_validation() {
        let config = InputSanitizationConfig::default();

        // Valid file
        assert!(Validator::validate_file_upload("document.pdf", Some("application/pdf"), 1024, &config).is_ok());

        // Blocked extension
        assert!(Validator::validate_file_upload("malware.exe", Some("application/x-executable"), 1024, &config).is_err());

        // Too large
        assert!(Validator::validate_file_upload("bigfile.txt", Some("text/plain"), 200 * 1024 * 1024, &config).is_err());

        // Invalid filename
        assert!(Validator::validate_file_upload("<script>evil.js", Some("application/javascript"), 1024, &config).is_err());
    }

    #[test]
    fn test_string_validation_and_sanitization() {
        let config = InputSanitizationConfig::default();

        // Valid input
        let result = Validator::validate_and_sanitize_string("normal text", "field", None, &config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "normal text");

        // Input with XSS
        let result = Validator::validate_and_sanitize_string("<script>alert(1)</script>", "field", None, &config);
        assert!(result.is_err());

        // Too long input
        let long_input = "a".repeat(20000);
        let result = Validator::validate_and_sanitize_string(&long_input, "field", Some(100), &config);
        assert!(result.is_err());
    }
}

mod config_tests {
    use super::super::config::InputSanitizationConfig;

    #[test]
    fn test_config_defaults() {
        let config = InputSanitizationConfig::default();

        assert_eq!(config.max_string_length, 10000);
        assert_eq!(config.max_depth, 10);
        assert!(config.remove_null_bytes);
        assert!(config.normalize_unicode);
        assert!(!config.blocked_patterns.is_empty());
        assert_eq!(config.allowed_identifier_chars, r"^[a-zA-Z0-9_-]+$");
    }

    #[test]
    fn test_config_builder() {
        let config = InputSanitizationConfig::new()
            .with_max_string_length(5000)
            .with_max_depth(5)
            .with_null_byte_removal(false)
            .with_blocked_pattern("test".to_string());

        assert_eq!(config.max_string_length, 5000);
        assert_eq!(config.max_depth, 5);
        assert!(!config.remove_null_bytes);
        assert!(config.blocked_patterns.contains("test"));
    }
}

mod integration_tests {
    use super::super::{config::InputSanitizationConfig, sanitizers::Sanitizer, validators::Validator};

    #[test]
    fn test_comprehensive_input_validation() {
        let config = InputSanitizationConfig::default();

        // Test tenant ID validation
        assert!(Validator::validate_tenant_id("valid-tenant_123", &config).is_ok());
        assert!(Validator::validate_tenant_id("invalid tenant!", &config).is_err());
        assert!(Validator::validate_tenant_id("ab", &config).is_err()); // Too short
        assert!(Validator::validate_tenant_id("admin", &config).is_err()); // Reserved

        // Test identifier validation
        assert!(Validator::validate_identifier("valid_name_123", "field", &config).is_ok());
        assert!(Validator::validate_identifier("invalid name!", "field", &config).is_err());
        assert!(Validator::validate_identifier("", "field", &config).is_err());
        assert!(Validator::validate_identifier(&"a".repeat(300), "field", &config).is_err()); // Too long
    }

    #[test]
    fn test_input_sanitization_edge_cases() {
        let config = InputSanitizationConfig::default();

        // Test empty input
        let sanitized = Sanitizer::sanitize_string("", &config);
        assert_eq!(sanitized, "");

        // Test input with only special characters
        let input = "<>\"'&";
        let sanitized = Sanitizer::sanitize_string(input, &config);
        assert_eq!(sanitized, "&lt;&gt;&quot;&#x27;&amp;");
    }

    #[test]
    fn test_malicious_pattern_comprehensive() {
        use std::collections::HashSet;

        // Test various XSS patterns
        assert!(Sanitizer::contains_blocked_patterns("<script>alert(1)</script>", &HashSet::from(["<script".to_string()])));
        assert!(Sanitizer::contains_blocked_patterns("javascript:alert('xss')", &HashSet::from(["javascript:".to_string()])));
        assert!(Sanitizer::contains_blocked_patterns("vbscript:msgbox('test')", &HashSet::from(["vbscript:".to_string()])));
        assert!(Sanitizer::contains_blocked_patterns("data:text/html,<script>", &HashSet::from(["data:".to_string()])));
        assert!(Sanitizer::contains_blocked_patterns("onload=evilFunction()", &HashSet::from(["onload=".to_string()])));
        assert!(Sanitizer::contains_blocked_patterns("eval(console.log(1))", &HashSet::from(["eval(".to_string()])));

        // Test directory traversal
        assert!(Sanitizer::contains_blocked_patterns("../../../etc/passwd", &HashSet::from(["../".to_string()])));
        assert!(Sanitizer::contains_blocked_patterns("..\\windows\\system32", &HashSet::from(["..\\".to_string()])));

        // Test case insensitive matching
        assert!(Sanitizer::contains_blocked_patterns("JAVASCRIPT:alert(1)", &HashSet::from(["javascript:".to_string()])));

        // Test safe content
        assert!(!Sanitizer::contains_blocked_patterns("normal text content", &HashSet::from(["<script".to_string()])));
        assert!(!Sanitizer::contains_blocked_patterns("email@domain.com", &HashSet::from(["javascript:".to_string()])));
    }
}