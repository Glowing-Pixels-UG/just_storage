use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use just_storage::api::middleware::config::MiddlewareConfig;
use just_storage::api::middleware::rate_limiting::RateLimitConfig;
use just_storage::api::{create_router_with_middleware, internal::create_internal_router};
use just_storage::ApplicationBuilder;

#[tokio::test]
async fn rate_limiting_returns_429_when_limit_exceeded() {
    let _ = tracing_subscriber::fmt::try_init();

    // Setup test environment
    let (config, container, temp_dir) = setup_config().await;

    // Set very low rate limit for testing: 3 requests per minute
    let middleware_config = MiddlewareConfig {
        rate_limiting: RateLimitConfig {
            unauthenticated_requests_per_minute: 3,
            authenticated_requests_per_minute: 10,
            max_concurrent_per_user: 10,
            max_concurrent_per_tenant: 10,
            max_concurrent_per_ip: 10,
            window_seconds: 60,
        },
        ..MiddlewareConfig::default()
    };

    let builder = ApplicationBuilder::new(config.clone())
        .with_database()
        .await
        .unwrap();

    let (state, api_key_repo, audit_repo) = builder
        .with_infrastructure()
        .await
        .unwrap()
        .with_api_keys()
        .await
        .unwrap()
        .build()
        .unwrap();

    let app =
        create_router_with_middleware(state, api_key_repo, audit_repo, middleware_config).await;

    // Use a fixed IP for all requests to ensure they hit the same rate limit bucket
    let ip = "1.2.3.4";

    // First 3 requests should succeed (but will be 401 because no auth)
    for i in 0..3 {
        let req = Request::builder()
            .uri("/v1/objects")
            .header("X-Forwarded-For", ip)
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "Request {} should have been 401",
            i + 1
        );
    }

    // 4th request should be rate limited
    let req = Request::builder()
        .uri("/v1/objects")
        .header("X-Forwarded-For", ip)
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(req).await.unwrap();
    let status = response.status();
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

    // Check for Retry-After header
    assert!(response.headers().contains_key("Retry-After"));

    let _ = temp_dir; // keep alive
    let _ = container; // keep alive
}

#[tokio::test]
async fn auth_routes_have_aggressive_rate_limiting() {
    let (config, container, temp_dir) = setup_config().await;

    let builder = ApplicationBuilder::new(config.clone())
        .with_database()
        .await
        .unwrap();

    let (state, _, _) = builder
        .with_infrastructure()
        .await
        .unwrap()
        .with_api_keys()
        .await
        .unwrap()
        .build()
        .unwrap();

    // The internal router has hardcoded 10 requests per minute for auth routes
    let internal_app = create_internal_router(state).await;

    // Use a fixed IP
    let ip = "5.6.7.8";

    // Send 10 requests to /auth/login
    for _ in 0..10 {
        let req = Request::builder()
            .uri("/auth/login")
            .header("X-Forwarded-For", ip)
            .header("hx-request", "true") // Ensure it doesn't redirect but returns 200/HX-Redirect
            .body(Body::empty())
            .unwrap();
        let response = internal_app.clone().oneshot(req).await.unwrap();

        let status = response.status();
        assert_ne!(status, StatusCode::TOO_MANY_REQUESTS);
    }

    // 11th request should be rate limited
    let req = Request::builder()
        .uri("/auth/login")
        .header("X-Forwarded-For", ip)
        .body(Body::empty())
        .unwrap();
    let response = internal_app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    let _ = temp_dir; // keep alive
    let _ = container; // keep alive
}

async fn setup_config() -> (
    just_storage::Config,
    testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>,
    tempfile::TempDir,
) {
    use crate::common::database::setup_test_database;
    let container = setup_test_database().await.1;
    let host = container.get_host().await.unwrap();
    let port = container.get_host_port_ipv4(5432).await.unwrap();
    let database_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

    let mut config = just_storage::Config::from_env();
    config.database_url = database_url;

    let temp_dir = tempfile::TempDir::new().unwrap();
    config.hot_storage_root = temp_dir.path().join("hot");
    config.cold_storage_root = temp_dir.path().join("cold");
    std::fs::create_dir_all(&config.hot_storage_root).unwrap();
    std::fs::create_dir_all(&config.cold_storage_root).unwrap();

    (config, container, temp_dir)
}
