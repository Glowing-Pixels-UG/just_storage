use axum::http::StatusCode;

use crate::common::{environment as env, http};
use tower::ServiceExt;

#[tokio::test]
async fn health_endpoint_returns_ok_with_healthy_status() {
    let (app, _, _container, _temp_dir) = env::setup_test_api_server().await;

    let req = http::get_request("/health");

    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let json = http::extract_json_response(response).await;
    assert_eq!(json["status"], "healthy");

    // readiness
    let req = http::get_request("/health/ready");
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
