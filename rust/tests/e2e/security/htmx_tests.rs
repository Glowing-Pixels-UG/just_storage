use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use crate::common::environment as env;

#[tokio::test]
async fn htmx_requests_get_hx_redirect_instead_of_302() {
    let _ = tracing_subscriber::fmt::try_init();
    
    // We can use the internal app which handles redirects for OIDC login
    let (_app, internal_app, _container, _temp_dir) = env::setup_test_api_server().await;

    // 1. Regular request to /auth/login (should be 303 or 302 redirect)
    // Note: Since OIDC is not enabled in setup_test_api_server, it might redirect to /dashboard/login
    let req = Request::builder()
        .method("GET")
        .uri("/auth/login")
        .body(Body::empty())
        .unwrap();

    let response = internal_app.clone().oneshot(req).await.unwrap();
    assert!(response.status().is_redirection());
    assert!(response.headers().contains_key("location"));
    assert!(!response.headers().contains_key("HX-Redirect"));

    // 2. HTMX request to /auth/login (should be 200 with HX-Redirect)
    let req = Request::builder()
        .method("GET")
        .uri("/auth/login")
        .header("hx-request", "true")
        .body(Body::empty())
        .unwrap();

    let response = internal_app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("HX-Redirect"));
    assert!(!response.headers().contains_key("location"));
    
    let hx_redirect = response.headers().get("HX-Redirect").unwrap().to_str().unwrap();
    assert!(hx_redirect.contains("/dashboard/login") || hx_redirect.contains("authorize"));
}
