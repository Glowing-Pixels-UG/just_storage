//! E2E API tests (health & openapi)

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

#[path = "../common/assertions.rs"]
mod assertions;
#[path = "../common/environment.rs"]
mod env;
#[path = "../common/http.rs"]
mod http;

#[tokio::test]
async fn api_test_health_endpoints() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

    // Test health endpoint
    let req = http::get_request("/health");

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let json = http::extract_json_response(response).await;
    assert_eq!(json["status"], "healthy");

    // Test readiness endpoint
    let req = http::get_request("/health/ready");

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn api_test_openapi_specification() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

    let req = http::get_request("/api-docs/openapi.json");

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let json = http::extract_json_response(response).await;
    assert!(json["openapi"].is_string());
    assert!(json["paths"].is_object());
    assert!(json["paths"].as_object().unwrap().contains_key("/v1/objects"));
}
