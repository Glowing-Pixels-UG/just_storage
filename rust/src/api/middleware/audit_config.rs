use serde::{Deserialize, Serialize};

/// Audit logging configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Whether audit logging is enabled
    pub enabled: bool,
    /// Whether to log to console (for development)
    pub console_logging: bool,
    /// Whether to log to database (for production)
    pub database_logging: bool,
    /// Log level for audit events
    pub log_level: String,
    /// Whether to include request/response bodies in audit logs
    pub include_bodies: bool,
    /// Maximum body size to log (in bytes)
    pub max_body_size: usize,
    /// Events to exclude from logging
    pub exclude_events: Vec<String>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            console_logging: cfg!(debug_assertions),
            database_logging: !cfg!(debug_assertions),
            log_level: "info".to_string(),
            include_bodies: false,
            max_body_size: 1024, // 1KB
            exclude_events: vec!["HealthCheck".to_string()],
        }
    }
}

impl AuditConfig {
    /// Create audit config for production
    pub fn production() -> Self {
        Self {
            enabled: true,
            console_logging: false,
            database_logging: true,
            log_level: "info".to_string(),
            include_bodies: false,
            max_body_size: 1024,
            exclude_events: vec!["HealthCheck".to_string(), "ObjectRead".to_string()],
        }
    }

    /// Create audit config for development
    pub fn development() -> Self {
        Self {
            enabled: true,
            console_logging: true,
            database_logging: false,
            log_level: "debug".to_string(),
            include_bodies: true,
            max_body_size: 4096, // 4KB for dev
            exclude_events: vec![],
        }
    }

    /// Check if an event type should be logged
    pub fn should_log_event(&self, event_type: &str) -> bool {
        !self.exclude_events.contains(&event_type.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AuditConfig::default();
        assert!(config.enabled);
        assert!(!config.include_bodies);
        assert_eq!(config.max_body_size, 1024);
    }

    #[test]
    fn test_production_config() {
        let config = AuditConfig::production();
        assert!(config.database_logging);
        assert!(!config.console_logging);
        assert!(config.exclude_events.contains(&"HealthCheck".to_string()));
    }

    #[test]
    fn test_development_config() {
        let config = AuditConfig::development();
        assert!(config.console_logging);
        assert!(!config.database_logging);
        assert!(config.include_bodies);
    }

    #[test]
    fn test_should_log_event() {
        let config = AuditConfig {
            exclude_events: vec!["HealthCheck".to_string()],
            ..Default::default()
        };

        assert!(!config.should_log_event("HealthCheck"));
        assert!(config.should_log_event("ObjectCreated"));
    }
}
