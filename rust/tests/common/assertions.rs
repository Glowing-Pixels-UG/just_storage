#![allow(dead_code)]

//! Custom assertion helpers for tests (Phase 1)

use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::Response;
use serde_json::Value;

/// Basic helper to assert a result is an error
pub fn assert_is_err<T, E>(res: Result<T, E>) {
    assert!(res.is_err());
}

/// Assert that a response contains valid JSON with expected keys
pub async fn assert_json_response(response: Response, expected_keys: &[&str]) {
    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();

    for key in expected_keys {
        assert!(
            json.get(key).is_some(),
            "Expected JSON response to contain key '{}'",
            key
        );
    }
}

/// Assert that a response contains an error message and expected status
pub async fn assert_error_response(response: Response, expected_status: StatusCode) {
    assert_eq!(response.status(), expected_status);
    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(json.get("error").is_some(), "Expected error response");
}

/// Assert that a response has the expected status code
pub async fn assert_status(response: Response, expected: StatusCode) {
    assert_eq!(
        response.status(),
        expected,
        "Expected status {}, got {}",
        expected,
        response.status()
    );
}
