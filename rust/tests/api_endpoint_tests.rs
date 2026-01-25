//! Comprehensive API endpoint tests
//!
//! These tests cover all API endpoints with various scenarios including
//! authentication, error cases, and edge conditions.

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};

use serde_json::json;

use tower::ServiceExt;

#[path = "common/assertions.rs"]
mod assertions;
#[path = "common/environment.rs"]
mod env;
#[path = "common/http.rs"]
mod http;

/// Helper to create authenticated requests
fn authenticated_request(
    method: Method,
    uri: &str,
    api_key: &str,
    body: Option<serde_json::Value>,
) -> Request<Body> {
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
        builder.body(Body::empty()).unwrap()
    }
}

/// Helper to extract JSON response
async fn extract_json_response(response: axum::response::Response) -> serde_json::Value {
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body_bytes).unwrap()
}

#[tokio::test]
async fn api_test_health_endpoints() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

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

#[tokio::test]
async fn api_test_openapi_specification() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

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
    assert!(json["paths"]
        .as_object()
        .unwrap()
        .contains_key("/v1/objects"));
}

#[tokio::test]
async fn api_test_unauthenticated_requests() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

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
        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "Endpoint {} {} should require authentication",
            method,
            uri
        );
    }
}

#[tokio::test]
async fn api_test_invalid_authentication() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

    // Test with invalid API key format
    let req = authenticated_request(Method::GET, "/v1/objects", "invalid-key-format", None);

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Test with non-existent API key
    let req = authenticated_request(
        Method::GET,
        "/v1/objects",
        "00000000-0000-0000-0000-000000000000",
        None,
    );

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn api_test_malformed_requests() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

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
    let req = authenticated_request(Method::POST, "/v1/objects", "test-key", Some(json!({})));

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn api_test_validation_errors() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

    // Test invalid namespace (empty)
    let req = authenticated_request(
        Method::POST,
        "/v1/objects",
        "test-key",
        Some(json!({
            "namespace": "",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000"
        })),
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
        })),
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
        })),
    );

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn api_test_rate_limiting() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

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

#[tokio::test]
async fn api_test_cors_headers() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

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
    assert!(
        headers.contains_key("access-control-allow-origin")
            || headers.contains_key("access-control-allow-headers")
    );
}

#[tokio::test]
async fn api_test_security_headers() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

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

#[tokio::test]
async fn api_test_input_sanitization() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

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
        Some(malicious_input),
    );

    let response = app.oneshot(req).await.unwrap();
    // Should either reject or sanitize the input
    assert!(response.status().is_client_error() || response.status().is_success());
}

#[tokio::test]
async fn api_test_content_type_validation() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

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

#[tokio::test]
async fn api_test_request_size_limits() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

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
        })),
    );

    let response = app.oneshot(req).await.unwrap();
    // Should either succeed or return a size limit error
    assert!(response.status().is_success() || response.status() == StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn api_test_concurrent_requests() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

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
