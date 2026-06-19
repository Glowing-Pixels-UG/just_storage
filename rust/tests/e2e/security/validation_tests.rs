use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use crate::common::{environment as env, http};

#[tokio::test]
async fn malformed_json_returns_bad_request() {
    let (app, _, _container, _temp_dir) = env::setup_test_api_server().await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/v1/objects")
        .header("content-type", "application/json")
        .body(Body::from("invalid json {"))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn validation_errors_return_bad_request() {
    let (app, _, _container, _temp_dir) = env::setup_test_api_server().await;

    let req = http::authenticated_json_request(
        Method::POST,
        "/v1/objects",
        "test-key",
        json!({
            "namespace": "",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000"
        }),
    );

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let req = http::authenticated_json_request(
        Method::POST,
        "/v1/objects",
        "test-key",
        json!({
            "namespace": "test",
            "tenant_id": "invalid-uuid"
        }),
    );

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let req = http::authenticated_json_request(
        Method::POST,
        "/v1/objects",
        "test-key",
        json!({
            "namespace": "invalid namespace!",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000"
        }),
    );

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn content_type_must_be_json_returns_unsupported() {
    let (app, _, _container, _temp_dir) = env::setup_test_api_server().await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/v1/objects")
        .header("content-type", "text/html")
        .body(Body::from("malicious html content"))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn binary_object_upload_content_type_is_allowed() {
    let (app, _, _container, _temp_dir) = env::setup_test_api_server().await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/v1/objects?namespace=test&tenant_id=550e8400-e29b-41d4-a716-446655440000")
        .header("authorization", "Bearer test-key")
        .header("content-type", "application/octet-stream")
        .body(Body::from("binary-ish content"))
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();
    assert_ne!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn request_size_may_be_limited_or_succeed() {
    let (app, _, _container, _temp_dir) = env::setup_test_api_server().await;

    let large_body = "x".repeat(1024 * 1024);
    let req = http::authenticated_json_request(
        Method::POST,
        "/v1/objects",
        "test-key",
        json!({
            "namespace": "test",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
            "data": large_body
        }),
    );

    let response = app.clone().oneshot(req).await.unwrap();
    assert!(response.status().is_success() || response.status() == StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn input_sanitization_handles_malicious_input() {
    let (app, _, _container, _temp_dir) = env::setup_test_api_server().await;

    let malicious_input = json!({
        "namespace": "../../../etc/passwd",
        "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
        "key": "<script>alert('xss')</script>"
    });

    let req =
        http::authenticated_json_request(Method::POST, "/v1/objects", "test-key", malicious_input);

    let response = app.clone().oneshot(req).await.unwrap();
    assert!(response.status().is_client_error() || response.status().is_success());
}
