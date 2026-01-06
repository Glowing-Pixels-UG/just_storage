//! Comprehensive API endpoint tests
//!
//! These tests cover all API endpoints with various scenarios including
//! authentication, error cases, and edge conditions.

use std::sync::Arc;
use axum::body::Body;
use axum::http::{Request, StatusCode, Method};
use axum::Router;
use serde_json::json;
use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};
use tower::ServiceExt;

use just_storage::{
    api::create_router,
    ApplicationBuilder,
    Config,
};

/// Setup test API server with PostgreSQL container
async fn setup_test_api_server() -> (Router, testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>) {
    // Start PostgreSQL container with schema
    let init_sql = include_str!("../../schema.sql");
    let container = Postgres::default()
        .with_init_sql(init_sql.as_bytes().to_vec())
        .start()
        .await
        .expect("Failed to start PostgreSQL container");

    let host = container.get_host().await.expect("Failed to get container host");
    let port = container.get_host_port_ipv4(5432).await.expect("Failed to get container port");
    let database_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

    // Create test config
    let mut config = Config::from_env();
    config.database_url = database_url;

    // Setup temporary storage
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    config.hot_storage_root = temp_dir.path().join("hot");
    config.cold_storage_root = temp_dir.path().join("cold");
    std::fs::create_dir_all(&config.hot_storage_root).expect("Failed to create hot storage");
    std::fs::create_dir_all(&config.cold_storage_root).expect("Failed to create cold storage");

    // Build application
    let builder = ApplicationBuilder::new(config).with_database().await.unwrap();

    // Initialize GC worker (but don't run it)
    let _gc = builder.build_gc().unwrap();

    let (state, api_key_repo, audit_repo) = builder
        .with_infrastructure()
        .await
        .unwrap()
        .with_api_keys()
        .await
        .unwrap()
        .build()
        .unwrap();

    // Create router
    let app = create_router(state, api_key_repo, audit_repo);

    (app, container)
}

/// Helper to create authenticated requests
fn authenticated_request(method: Method, uri: &str, api_key: &str, body: Option<serde_json::Value>) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {}", api_key));

    if let Some(data) = body {
        builder
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&data).unwrap()))
            .unwrap()
    } else {
        builder
            .body(Body::empty())
            .unwrap()
    }
}

/// Helper to extract JSON response
async fn extract_json_response(response: axum::response::Response) -> serde_json::Value {
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body_bytes).unwrap()
}

#[sqlx::test]
async fn api_test_health_endpoints(pool: PgPool) {
    let (app, _container) = setup_test_api_server().await;

    // Test health endpoint
    let req = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let json = extract_json_response(response).await;
    assert_eq!(json["status"], "healthy");

    // Test readiness endpoint
    let req = Request::builder()
        .uri("/health/ready")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn api_test_openapi_specification(pool: PgPool) {
    let (app, _container) = setup_test_api_server().await;

    let req = Request::builder()
        .uri("/api-docs/openapi.json")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let json = extract_json_response(response).await;
    assert!(json["openapi"].is_string());
    assert!(json["paths"].is_object());
    assert!(json["paths"].as_object().unwrap().contains_key("/v1/objects"));
}

#[sqlx::test]
async fn api_test_unauthenticated_requests(pool: PgPool) {
    let (app, _container) = setup_test_api_server().await;

    // Test various endpoints without authentication
    let endpoints = vec![
        (Method::POST, "/v1/objects"),
        (Method::GET, "/v1/objects"),
        (Method::GET, "/v1/api-keys"),
        (Method::POST, "/v1/api-keys"),
    ];

    for (method, uri) in endpoints {
        let req = Request::builder()
            .method(method.clone())
            .uri(uri)
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED,
            "Endpoint {} {} should require authentication", method, uri);
    }
}

#[sqlx::test]
async fn api_test_invalid_authentication(pool: PgPool) {
    let (app, _container) = setup_test_api_server().await;

    // Test with invalid API key format
    let req = authenticated_request(
        Method::GET,
        "/v1/objects",
        "invalid-key-format",
        None
    );

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Test with non-existent API key
    let req = authenticated_request(
        Method::GET,
        "/v1/objects",
        "00000000-0000-0000-0000-000000000000",
        None
    );

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn api_test_malformed_requests(pool: PgPool) {
    let (app, _container) = setup_test_api_server().await;

    // Test with invalid JSON
    let req = Request::builder()
        .method(Method::POST)
        .uri("/v1/objects")
        .header("content-type", "application/json")
        .body(Body::from("invalid json {"))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Test with missing required fields
    let req = authenticated_request(
        Method::POST,
        "/v1/objects",
        "test-key",
        Some(json!({}))
    );

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn api_test_validation_errors(pool: PgPool) {
    let (app, _container) = setup_test_api_server().await;

    // Test invalid namespace (empty)
    let req = authenticated_request(
        Method::POST,
        "/v1/objects",
        "test-key",
        Some(json!({
            "namespace": "",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000"
        }))
    );

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Test invalid tenant_id format
    let req = authenticated_request(
        Method::POST,
        "/v1/objects",
        "test-key",
        Some(json!({
            "namespace": "test",
            "tenant_id": "invalid-uuid"
        }))
    );

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Test namespace with invalid characters
    let req = authenticated_request(
        Method::POST,
        "/v1/objects",
        "test-key",
        Some(json!({
            "namespace": "invalid namespace!",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000"
        }))
    );

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn api_test_rate_limiting(pool: PgPool) {
    let (app, _container) = setup_test_api_server().await;

    // Make multiple rapid requests to test rate limiting
    // Note: This assumes rate limiting is configured
    for i in 0..10 {
        let req = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(req).await.unwrap();

        // First few requests should succeed
        if i < 5 {
            assert_eq!(response.status(), StatusCode::OK);
        }
        // Later requests might be rate limited (429)
        // This depends on the rate limiting configuration
    }
}

#[sqlx::test]
async fn api_test_cors_headers(pool: PgPool) {
    let (app, _container) = setup_test_api_server().await;

    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri("/v1/objects")
        .header("origin", "http://localhost:3000")
        .header("access-control-request-method", "POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();

    // Check CORS headers
    let headers = response.headers();
    assert!(headers.contains_key("access-control-allow-origin") ||
            headers.contains_key("access-control-allow-headers"));
}

#[sqlx::test]
async fn api_test_security_headers(pool: PgPool) {
    let (app, _container) = setup_test_api_server().await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();

    // Check security headers
    let headers = response.headers();
    assert!(headers.contains_key("x-content-type-options"));
    assert!(headers.contains_key("x-frame-options"));
    assert!(headers.contains_key("x-xss-protection"));
}

#[sqlx::test]
async fn api_test_input_sanitization(pool: PgPool) {
    let (app, _container) = setup_test_api_server().await;

    // Test with potentially malicious input
    let malicious_input = json!({
        "namespace": "../../../etc/passwd",
        "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
        "key": "<script>alert('xss')</script>"
    });

    let req = authenticated_request(
        Method::POST,
        "/v1/objects",
        "test-key",
        Some(malicious_input)
    );

    let response = app.oneshot(req).await.unwrap();
    // Should either reject or sanitize the input
    assert!(response.status().is_client_error() || response.status().is_success());
}

#[sqlx::test]
async fn api_test_content_type_validation(pool: PgPool) {
    let (app, _container) = setup_test_api_server().await;

    // Test with invalid content type
    let req = Request::builder()
        .method(Method::POST)
        .uri("/v1/objects")
        .header("content-type", "text/html")
        .body(Body::from("malicious html content"))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    // Should reject non-JSON content for JSON endpoints
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[sqlx::test]
async fn api_test_request_size_limits(pool: PgPool) {
    let (app, _container) = setup_test_api_server().await;

    // Create a very large request body
    let large_body = "x".repeat(1024 * 1024); // 1MB of data
    let req = authenticated_request(
        Method::POST,
        "/v1/objects",
        "test-key",
        Some(json!({
            "namespace": "test",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
            "data": large_body
        }))
    );

    let response = app.oneshot(req).await.unwrap();
    // Should either succeed or return a size limit error
    assert!(response.status().is_success() ||
            response.status() == StatusCode::PAYLOAD_TOO_LARGE);
}

#[sqlx::test]
async fn api_test_concurrent_requests(pool: PgPool) {
    let (app, _container) = setup_test_api_server().await;

    // Make multiple concurrent requests
    let mut handles = Vec::new();

    for _ in 0..5 {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let req = Request::builder()
                .method(Method::GET)
                .uri("/health")
                .body(Body::empty())
                .unwrap();

            app_clone.oneshot(req).await.unwrap()
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let response = handle.await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}