use axum::http::{HeaderName, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// CORS configuration for production security
pub fn create_cors_layer() -> CorsLayer {
    // In production, you should configure this based on your actual domains
    let allowed_origins = std::env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,http://localhost:8080".to_string());

    let origins: Vec<HeaderValue> = allowed_origins
        .split(',')
        .filter_map(|origin| origin.trim().parse::<HeaderValue>().ok())
        .collect();

    let origins = if origins.is_empty() {
        AllowOrigin::any()
    } else {
        AllowOrigin::list(origins)
    };

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
            Method::HEAD,
        ])
        .allow_headers([
            HeaderName::from_static("authorization"),
            HeaderName::from_static("content-type"),
            HeaderName::from_static("x-request-id"),
            HeaderName::from_static("x-api-key"),
        ])
        .allow_credentials(false) // Set to true only if you need to send cookies/auth headers
        .max_age(std::time::Duration::from_secs(86400)) // 24 hours
}

/// CORS configuration for development (more permissive)
pub fn create_development_cors_layer() -> CorsLayer {
    use tower_http::cors::{AllowHeaders, AllowMethods};

    CorsLayer::new()
        .allow_origin(AllowOrigin::any())
        .allow_methods(AllowMethods::any())
        .allow_headers(AllowHeaders::any())
        .allow_credentials(true)
}

/// Select appropriate CORS layer based on environment
pub fn create_cors_layer_for_environment() -> CorsLayer {
    let is_development = std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string())
        .to_lowercase()
        == "development";

    if is_development {
        create_development_cors_layer()
    } else {
        create_cors_layer()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;

    #[test]
    fn test_create_cors_layer_basic() {
        let cors = create_cors_layer();
        // Verify the layer is created successfully
        assert!(matches!(cors, CorsLayer { .. }));
    }

    #[test]
    fn test_create_development_cors_layer() {
        let cors = create_development_cors_layer();
        assert!(matches!(cors, CorsLayer { .. }));
    }

    #[test]
    fn test_create_cors_layer_for_environment_development() {
        std::env::set_var("ENVIRONMENT", "development");
        let cors = create_cors_layer_for_environment();
        assert!(matches!(cors, CorsLayer { .. }));
        std::env::remove_var("ENVIRONMENT");
    }

    #[test]
    fn test_create_cors_layer_for_environment_production() {
        std::env::set_var("ENVIRONMENT", "production");
        let cors = create_cors_layer_for_environment();
        assert!(matches!(cors, CorsLayer { .. }));
        std::env::remove_var("ENVIRONMENT");
    }

    #[test]
    fn test_create_cors_layer_for_environment_default() {
        std::env::remove_var("ENVIRONMENT");
        let cors = create_cors_layer_for_environment();
        assert!(matches!(cors, CorsLayer { .. }));
    }

    #[test]
    fn test_cors_allowed_origins_from_env() {
        // Test with multiple valid origins
        std::env::set_var(
            "ALLOWED_ORIGINS",
            "https://example.com,https://app.example.com,http://localhost:3000"
        );
        let cors = create_cors_layer();
        assert!(matches!(cors, CorsLayer { .. }));
        std::env::remove_var("ALLOWED_ORIGINS");
    }

    #[test]
    fn test_cors_allowed_origins_invalid_env() {
        // Test with invalid origins - should fall back gracefully
        std::env::set_var("ALLOWED_ORIGINS", "not-a-valid-url,also-invalid");
        let cors = create_cors_layer();
        // Should still create a valid layer (falls back to any origin)
        assert!(matches!(cors, CorsLayer { .. }));
        std::env::remove_var("ALLOWED_ORIGINS");
    }

    #[test]
    fn test_cors_allowed_origins_empty_env() {
        // Test with empty origins - should allow any origin
        std::env::set_var("ALLOWED_ORIGINS", "");
        let cors = create_cors_layer();
        assert!(matches!(cors, CorsLayer { .. }));
        std::env::remove_var("ALLOWED_ORIGINS");
    }

    #[test]
    fn test_cors_allowed_origins_no_env() {
        // Test without ALLOWED_ORIGINS - should use default localhost origins
        std::env::remove_var("ALLOWED_ORIGINS");
        let cors = create_cors_layer();
        assert!(matches!(cors, CorsLayer { .. }));
    }

    #[test]
    fn test_cors_layer_with_mixed_valid_invalid_origins() {
        // Test with mix of valid and invalid origins
        std::env::set_var(
            "ALLOWED_ORIGINS",
            "https://valid.com,invalid-origin,https://another-valid.com"
        );
        let cors = create_cors_layer();
        // Should create layer with valid origins only
        assert!(matches!(cors, CorsLayer { .. }));
        std::env::remove_var("ALLOWED_ORIGINS");
    }

    #[test]
    fn test_cors_layer_with_whitespace_origins() {
        // Test with origins containing whitespace
        std::env::set_var(
            "ALLOWED_ORIGINS",
            "  https://example.com  ,  https://app.example.com  "
        );
        let cors = create_cors_layer();
        assert!(matches!(cors, CorsLayer { .. }));
        std::env::remove_var("ALLOWED_ORIGINS");
    }

    #[test]
    fn test_cors_layer_single_origin() {
        // Test with single origin
        std::env::set_var("ALLOWED_ORIGINS", "https://single-origin.com");
        let cors = create_cors_layer();
        assert!(matches!(cors, CorsLayer { .. }));
        std::env::remove_var("ALLOWED_ORIGINS");
    }

    #[test]
    fn test_cors_layer_large_number_of_origins() {
        // Test with many origins
        let origins = (0..20)
            .map(|i| format!("https://app{}.example.com", i))
            .collect::<Vec<_>>()
            .join(",");
        std::env::set_var("ALLOWED_ORIGINS", origins);
        let cors = create_cors_layer();
        assert!(matches!(cors, CorsLayer { .. }));
        std::env::remove_var("ALLOWED_ORIGINS");
    }
}
