use axum::body::to_bytes;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use serde_json::json;

mod common;
use common::{assertions, http, TestEnvironment};

#[tokio::test]
async fn smoke_assertions_helpers() {
    // Prepare an example JSON error body (unused variable removed)
    let _body = Body::from(r#"{"error":"unauthorized"}"#);

    // Use helpers
    let response_for_status = Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(Body::from(r#"{"error":"unauthorized"}"#))
        .unwrap();
    let response_for_error = Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(Body::from(r#"{"error":"unauthorized"}"#))
        .unwrap();

    assertions::assert_status(response_for_status, StatusCode::UNAUTHORIZED).await;
    assertions::assert_error_response(response_for_error, StatusCode::UNAUTHORIZED).await;

    // Prepare a JSON response with extra keys
    let body = Body::from(serde_json::to_string(&json!({ "ok": true, "data": {}})).unwrap());
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(body)
        .unwrap();
    assertions::assert_json_response(response, &["ok", "data"]).await;
}

#[tokio::test]
async fn smoke_http_helpers() {
    // json_body correctly serializes
    let body = http::json_body(json!({ "key": "value" }));
    let bytes = to_bytes(body, usize::MAX).await.unwrap();
    assert_eq!(
        bytes,
        serde_json::to_vec(&json!({ "key": "value" })).unwrap()
    );

    // authenticated_json_request sets headers and body
    let req: Request<Body> = http::authenticated_json_request(
        axum::http::Method::POST,
        "/test",
        "my-key",
        json!({ "a": 1 }),
    );

    assert_eq!(req.method(), axum::http::Method::POST);
    assert_eq!(req.uri().path(), "/test");
    assert_eq!(req.headers().get("authorization").unwrap(), "Bearer my-key");
    assert_eq!(
        req.headers().get("content-type").unwrap(),
        "application/json"
    );
}

#[tokio::test]
async fn smoke_builder() {
    // Ensure builder constructs a TestEnvironment without panicking and exercise all flags
    let _env = TestEnvironment::builder()
        .with_database(true)
        .with_use_cases(true)
        .with_api_server(true)
        .build()
        .await;
}

#[tokio::test]
async fn smoke_setup_api_server_and_factories() {
    // Exercise setup_test_api_server (starts a container, returns router + temp dir).
    let (_router, _container, _temp_dir) = common::setup_test_api_server().await;

    // Exercise factory helpers
    let obj = common::create_test_object();
    // Use public getters to access domain object values
    assert_eq!(obj.namespace().to_string(), "test");
}
