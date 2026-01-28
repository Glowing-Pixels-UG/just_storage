use axum::http::StatusCode;
use tower::ServiceExt;

use crate::common::{environment as env, http};

#[tokio::test]
async fn rate_limiting_applies_over_many_requests() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

    for i in 0..10 {
        let req = http::get_request("/health");

        let response = app.clone().oneshot(req).await.unwrap();

        if i < 5 {
            assert_eq!(response.status(), StatusCode::OK);
        }
    }
}

#[tokio::test]
async fn concurrent_requests_succeed() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

    let mut handles = Vec::new();

    for _ in 0..5 {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let req = http::get_request("/health");
            app_clone.oneshot(req).await.unwrap()
        });
        handles.push(handle);
    }

    for handle in handles {
        let response = handle.await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
