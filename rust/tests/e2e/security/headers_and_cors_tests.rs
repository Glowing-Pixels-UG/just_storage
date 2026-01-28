use axum::body::Body;
use axum::http::{Method, Request};
use tower::ServiceExt;

use crate::common::{environment as env, http};

#[tokio::test]
async fn cors_preflight_returns_cors_headers() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

    let req = Request::builder()
        .method(Method::OPTIONS)
        .uri("/v1/objects")
        .header("origin", "http://localhost:3000")
        .header("access-control-request-method", "POST")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(req).await.unwrap();

    let headers = response.headers();
    assert!(
        headers.contains_key("access-control-allow-origin")
            || headers.contains_key("access-control-allow-headers")
    );
}

#[tokio::test]
async fn security_headers_present() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

    let req = http::get_request("/health");

    let response = app.clone().oneshot(req).await.unwrap();

    let headers = response.headers();
    assert!(headers.contains_key("x-content-type-options"));
    assert!(headers.contains_key("x-frame-options"));
    assert!(headers.contains_key("x-xss-protection"));
}
