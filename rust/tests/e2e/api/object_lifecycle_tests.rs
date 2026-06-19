use axum::http::{Method, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use crate::common::{environment as env, http};

#[tokio::test]
async fn object_lifecycle_full_flow_succeeds() {
    let (app, _, _container, _temp_dir) = env::setup_test_api_server().await;
    let api_key = "test-key";

    // 1. Upload an object
    let upload_req = http::authenticated_json_request(
        Method::POST,
        "/v1/objects",
        api_key,
        json!({
            "namespace": "test",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
            "key": "test-file.txt",
            "data": "Hello, E2E!"
        }),
    );

    let response = app.clone().oneshot(upload_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = http::extract_json_response(response).await;
    let object_id = body.get("id").unwrap().as_str().unwrap().to_string();

    // 2. List objects
    let list_req = http::authenticated_request(
        Method::GET,
        "/v1/objects?namespace=test&tenant_id=550e8400-e29b-41d4-a716-446655440000",
        api_key,
    );
    let response = app.clone().oneshot(list_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = http::extract_json_response(response).await;
    let objects = body.get("objects").unwrap().as_array().unwrap();
    assert!(objects
        .iter()
        .any(|obj| obj.get("id").unwrap().as_str().unwrap() == object_id));

    // 3. Download object by ID
    let download_req = http::authenticated_request(
        Method::GET,
        &format!(
            "/v1/objects/{}?tenant_id=550e8400-e29b-41d4-a716-446655440000",
            object_id
        ),
        api_key,
    );
    let response = app.clone().oneshot(download_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body_bytes, "Hello, E2E!");

    // 4. Download object by Key
    let download_key_req = http::authenticated_request(
        Method::GET,
        "/v1/objects/by-key/test/550e8400-e29b-41d4-a716-446655440000/test-file.txt",
        api_key,
    );
    let response = app.clone().oneshot(download_key_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body_bytes, "Hello, E2E!");

    // 5. Delete object
    let delete_req = http::authenticated_request(
        Method::DELETE,
        &format!(
            "/v1/objects/{}?tenant_id=550e8400-e29b-41d4-a716-446655440000",
            object_id
        ),
        api_key,
    );
    let response = app.clone().oneshot(delete_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // 6. Verify deletion
    let get_req = http::authenticated_request(
        Method::GET,
        &format!(
            "/v1/objects/{}?tenant_id=550e8400-e29b-41d4-a716-446655440000",
            object_id
        ),
        api_key,
    );
    let response = app.oneshot(get_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
