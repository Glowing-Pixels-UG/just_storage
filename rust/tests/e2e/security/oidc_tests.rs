use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use crate::common::{environment as env, oidc};

#[tokio::test]
async fn oidc_login_redirects_to_idp() {
    let _ = tracing_subscriber::fmt::try_init();
    let mock_oidc = oidc::MockOidcServer::new().await;
    
    let (_app, internal_app, _container, _temp_dir) = 
        env::setup_test_api_server_with_oidc(mock_oidc.issuer_url.clone()).await;

    let req = Request::builder()
        .method("GET")
        .uri("/auth/login")
        .body(Body::empty())
        .unwrap();

    let response = internal_app.oneshot(req).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    assert!(location.starts_with(&mock_oidc.issuer_url));
    assert!(location.contains("response_type=code"));
    assert!(location.contains("client_id=test-client"));
}

#[tokio::test]
async fn oidc_callback_without_session_fails() {
    let mock_oidc = oidc::MockOidcServer::new().await;
    
    let (_app, internal_app, _container, _temp_dir) = 
        env::setup_test_api_server_with_oidc(mock_oidc.issuer_url.clone()).await;

    // Call callback without state in session
    let req = Request::builder()
        .method("GET")
        .uri("/auth/callback?code=test-code&state=invalid-state")
        .body(Body::empty())
        .unwrap();

    let response = internal_app.oneshot(req).await.unwrap();
    
    // Should redirect to login page due to missing state in session
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get("location").unwrap().to_str().unwrap(), "/dashboard/login");
}

#[tokio::test]
async fn api_request_with_valid_oidc_token_succeeds() {
    let mock_oidc = oidc::MockOidcServer::new().await;
    
    let (app, _internal_app, _container, _temp_dir) = 
        env::setup_test_api_server_with_oidc(mock_oidc.issuer_url.clone()).await;

    // Generate valid OIDC token
    let token = mock_oidc.generate_token("test-user", "test-tenant", vec!["user".to_string()]);

    let req = Request::builder()
        .method("GET")
        .uri("/v1/objects")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    
    // Should be allowed (or at least not unauthorized/forbidden by auth middleware)
    // It might return 200 (empty list) or other depending on database state
    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
    assert_ne!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn oidc_token_with_roles_and_tenant_mapped_correctly() {
    let mock_oidc = oidc::MockOidcServer::new().await;
    
    let (app, _internal_app, _container, _temp_dir) = 
        env::setup_test_api_server_with_oidc(mock_oidc.issuer_url.clone()).await;

    // Generate token with specific roles and tenant
    let token = mock_oidc.generate_token("test-user", "custom-tenant", vec!["admin".to_string()]);

    let req = Request::builder()
        .method("GET")
        .uri("/v1/api-keys") // Admin-only endpoint
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    
    // Should be allowed because role "admin" has "api_keys:read" permission
    // Even if it returns 200 OK or something else, it should NOT be 401 or 403
    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
    assert_ne!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn api_request_with_invalid_oidc_token_fails() {
    let mock_oidc = oidc::MockOidcServer::new().await;
    
    let (app, _internal_app, _container, _temp_dir) = 
        env::setup_test_api_server_with_oidc(mock_oidc.issuer_url.clone()).await;

    let req = Request::builder()
        .method("GET")
        .uri("/v1/objects")
        .header("Authorization", "Bearer invalid-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    
    // Auth middleware should fall back to unauthorized if no auth method succeeds
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

