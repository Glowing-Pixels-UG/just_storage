mod common;
use common::TestEnvironment;
use axum::http::StatusCode;
use tower::ServiceExt;

#[tokio::test]
async fn test_api_server_via_builder() {
    let env = TestEnvironment::builder().with_api_server(true).build().await;
    let router = env.api_router.expect("api router should be present");

    let req = axum::http::Request::builder().uri("/health").method("GET").body(axum::body::Body::empty()).unwrap();
    let resp = router.oneshot(req).await.expect("request failed");
    assert_eq!(resp.status(), StatusCode::OK);
}
