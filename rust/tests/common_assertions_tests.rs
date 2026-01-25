use axum::body::Body;
use axum::http::Response;
use axum::http::StatusCode;

mod common;
use common::assertions;

#[tokio::test]
async fn test_assertions_helpers() {
    // JSON response with keys
    let body =
        Body::from(serde_json::to_string(&serde_json::json!({"ok": true, "data": {}})).unwrap());
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(body)
        .unwrap();

    assertions::assert_json_response(response, &["ok", "data"]).await;

    // Error response
    let err_body = Body::from(r#"{"error":"unauthorized"}"#);
    let err_resp = Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(err_body)
        .unwrap();

    assertions::assert_error_response(err_resp, StatusCode::UNAUTHORIZED).await;

    // Status assertion
    let resp = Response::builder()
        .status(StatusCode::CREATED)
        .body(Body::empty())
        .unwrap();
    assertions::assert_status(resp, StatusCode::CREATED).await;
}
