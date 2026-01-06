//! Middleware configuration aggregation
//!
//! This module provides a unified configuration structure for all middleware,
//! enabling proper dependency injection and configuration management.

use serde::{Deserialize, Serialize};

use super::{
    audit_config::AuditConfig, error_handling::ErrorHandlingConfig,
    input_sanitization::InputSanitizationConfig, rate_limiting::RateLimitConfig,
    security_headers::SecurityHeadersConfig, size_limits::SizeLimitConfig,
};

/// Unified middleware configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MiddlewareConfig {
    /// Audit logging configuration
    pub audit: AuditConfig,
    /// Input sanitization configuration
    pub input_sanitization: InputSanitizationConfig,
    /// Error handling configuration
    pub error_handling: ErrorHandlingConfig,
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
    /// Security headers configuration
    pub security_headers: SecurityHeadersConfig,
    /// Size limits configuration
    pub size_limits: SizeLimitConfig,
}


impl MiddlewareConfig {
    /// Create a new middleware config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure audit logging
    pub fn with_audit(mut self, config: AuditConfig) -> Self {
        self.audit = config;
        self
    }

    /// Configure input sanitization
    pub fn with_input_sanitization(mut self, config: InputSanitizationConfig) -> Self {
        self.input_sanitization = config;
        self
    }

    /// Configure error handling
    pub fn with_error_handling(mut self, config: ErrorHandlingConfig) -> Self {
        self.error_handling = config;
        self
    }

    /// Configure rate limiting
    pub fn with_rate_limiting(mut self, config: RateLimitConfig) -> Self {
        self.rate_limiting = config;
        self
    }

    /// Configure security headers
    pub fn with_security_headers(mut self, config: SecurityHeadersConfig) -> Self {
        self.security_headers = config;
        self
    }

    /// Configure size limits
    pub fn with_size_limits(mut self, config: SizeLimitConfig) -> Self {
        self.size_limits = config;
        self
    }

    /// Create a production-ready configuration
    pub fn production() -> Self {
        Self {
            audit: AuditConfig::production(),
            input_sanitization: InputSanitizationConfig::default(),
            error_handling: ErrorHandlingConfig::new().with_debug_info(false),
            rate_limiting: RateLimitConfig::default(),
            security_headers: SecurityHeadersConfig::default(),
            size_limits: SizeLimitConfig::default(),
        }
    }

    /// Create a development configuration with more permissive settings
    pub fn development() -> Self {
        Self {
            audit: AuditConfig::development(),
            input_sanitization: InputSanitizationConfig::default(),
            error_handling: ErrorHandlingConfig::new().with_debug_info(true),
            rate_limiting: RateLimitConfig {
                unauthenticated_requests_per_minute: 1000, // More permissive in dev
                authenticated_requests_per_minute: 5000,
                ..RateLimitConfig::default()
            },
            security_headers: SecurityHeadersConfig::default(),
            size_limits: SizeLimitConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MiddlewareConfig::default();
        // Just verify it creates without panicking
        assert!(config.audit.enabled);
        assert!(config.input_sanitization.max_string_length > 0);
        assert!(config.error_handling.sensitive_patterns.len() > 0);
    }

    #[test]
    fn test_new_config() {
        let config = MiddlewareConfig::new();
        assert_eq!(config.audit, AuditConfig::default());
        assert_eq!(
            config.input_sanitization,
            InputSanitizationConfig::default()
        );
        assert_eq!(config.error_handling, ErrorHandlingConfig::default());
    }

    #[test]
    fn test_config_with_methods() {
        let custom_input_config = InputSanitizationConfig {
            max_string_length: 1000,
            ..Default::default()
        };

        let custom_error_config = ErrorHandlingConfig::new().with_debug_info(false);

        let config = MiddlewareConfig::new()
            .with_input_sanitization(custom_input_config.clone())
            .with_error_handling(custom_error_config.clone());

        assert_eq!(config.input_sanitization.max_string_length, 1000);
        assert_eq!(config.error_handling.include_debug_info, false);
    }

    #[test]
    fn test_production_config() {
        let config = MiddlewareConfig::production();
        assert!(!config.error_handling.include_debug_info);
    }

    #[test]
    fn test_development_config() {
        let config = MiddlewareConfig::development();
        assert!(config.error_handling.include_debug_info);
        assert_eq!(
            config.rate_limiting.unauthenticated_requests_per_minute,
            1000
        );
    }

    #[test]
    fn test_serialization() {
        let config = MiddlewareConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: MiddlewareConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(
            config.input_sanitization.max_string_length,
            deserialized.input_sanitization.max_string_length
        );
    }
}
