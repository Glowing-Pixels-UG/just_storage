use axum::http::{Method, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use crate::common::{environment as env, http};

#[tokio::test]
async fn search_endpoints_work_as_expected() {
    let (app, _, _container, _temp_dir) = env::setup_test_api_server().await;
    let api_key = "test-key";

    // 1. Upload objects for search
    let objects = vec![
        json!({
            "namespace": "search-test",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
            "key": "rust-programming.txt",
            "data": "Rust language content"
        }),
        json!({
            "namespace": "search-test",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
            "key": "go-programming.txt",
            "data": "Go language content"
        }),
    ];

    for obj in objects {
        let req = http::authenticated_json_request(Method::POST, "/v1/objects", api_key, obj);
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // 2. Metadata search
    let search_req = http::authenticated_json_request(
        Method::POST,
        "/v1/objects/search",
        api_key,
        json!({
            "namespace": "search-test",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000"
        }),
    );
    let response = app.clone().oneshot(search_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = http::extract_json_response(response).await;
    let objects = body.get("objects").unwrap().as_array().unwrap();
    assert_eq!(objects.len(), 2);

    // 3. Text search
    let text_search_req = http::authenticated_json_request(
        Method::POST,
        "/v1/objects/search/text",
        api_key,
        json!({
            "namespace": "search-test",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
            "query": "Rust"
        }),
    );
    let response = app.oneshot(text_search_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = http::extract_json_response(response).await;
    let results = body.get("objects").unwrap().as_array().unwrap();
    assert!(results.iter().any(|r| r.get("key").unwrap().as_str().unwrap() == "rust-programming.txt"));
}
