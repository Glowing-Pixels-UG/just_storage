//! Middleware factory for creating configured middleware stacks
//!
//! This module provides a factory pattern for creating middleware layers
//! with proper dependency injection of configuration.

use once_cell::sync::Lazy;
use std::sync::Arc;

use super::config::MiddlewareConfig;
use super::{auth, cors::create_cors_layer_for_environment, metrics};

/// Cached default middleware configurations for performance
static DEFAULT_MIDDLEWARE_CONFIG: Lazy<MiddlewareConfig> = Lazy::new(MiddlewareConfig::default);
static PRODUCTION_MIDDLEWARE_CONFIG: Lazy<MiddlewareConfig> =
    Lazy::new(MiddlewareConfig::production);
static DEVELOPMENT_MIDDLEWARE_CONFIG: Lazy<MiddlewareConfig> =
    Lazy::new(MiddlewareConfig::development);

/// Middleware factory for creating configured middleware stacks
pub struct MiddlewareFactory {
    config: MiddlewareConfig,
}

impl MiddlewareFactory {
    /// Create a new factory with the given configuration
    pub fn new(config: MiddlewareConfig) -> Self {
        Self { config }
    }

    /// Create CORS layer for the application
    pub fn create_cors_layer(&self) -> tower_http::cors::CorsLayer {
        create_cors_layer_for_environment()
    }

    /// Create auth layer for the application
    pub fn create_auth_layer(
        &self,
        api_key_repo: Arc<dyn crate::application::ports::ApiKeyRepository + Send + Sync>,
    ) -> auth::AuthLayer {
        auth::create_auth_middleware(api_key_repo)
    }

    /// Create metrics layer for the application
    pub fn create_metrics_layer(&self) -> metrics::MetricsLayer {
        metrics::MetricsLayer::new()
    }

    /// Get the middleware configuration
    pub fn config(&self) -> &MiddlewareConfig {
        &self.config
    }
}

impl Default for MiddlewareFactory {
    fn default() -> Self {
        Self::new((*DEFAULT_MIDDLEWARE_CONFIG).clone())
    }
}

impl From<MiddlewareConfig> for MiddlewareFactory {
    fn from(config: MiddlewareConfig) -> Self {
        Self::new(config)
    }
}

impl MiddlewareFactory {
    /// Create a factory with production-ready configuration
    pub fn production() -> Self {
        Self::new((*PRODUCTION_MIDDLEWARE_CONFIG).clone())
    }

    /// Create a factory with development configuration
    pub fn development() -> Self {
        Self::new((*DEVELOPMENT_MIDDLEWARE_CONFIG).clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_factory_creation() {
        let config = MiddlewareConfig::production();
        let factory = MiddlewareFactory::new(config);

        // Verify the factory was created with the config
        assert!(!factory.config().error_handling.include_debug_info);
    }

    #[test]
    fn test_default_factory() {
        let factory = MiddlewareFactory::default();

        // Verify default configuration
        assert!(factory.config().error_handling.include_debug_info); // Debug assertions enabled in tests
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
    fn test_production_config() {
        let config = MiddlewareConfig::production();
        assert!(!config.error_handling.include_debug_info);
        assert_eq!(config.rate_limiting.unauthenticated_requests_per_minute, 60);
    }
}
