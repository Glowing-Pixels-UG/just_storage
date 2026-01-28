use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use tower::ServiceExt;

use crate::common::{environment as env, http};

#[tokio::test]
async fn unauthenticated_requests_require_authentication() {
    let (app, _, _container, _temp_dir) = env::setup_test_api_server().await;

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
async fn invalid_api_key_returns_unauthorized() {
    let (app, _, _container, _temp_dir) = env::setup_test_api_server().await;

    // invalid format
    let req = http::authenticated_request(Method::GET, "/v1/objects", "invalid-key-format");
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // non-existent key
    let req = http::authenticated_request(
        Method::GET,
        "/v1/objects",
        "00000000-0000-0000-0000-000000000000",
    );
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
