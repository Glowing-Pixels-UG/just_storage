use axum::http::{Method, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use crate::common::{environment as env, http};

#[tokio::test]
async fn api_key_management_full_flow_succeeds() {
    let (app, _, _container, _temp_dir) = env::setup_test_api_server().await;
    let api_key = "test-key";

    // 1. Create an API key
    let create_req = http::authenticated_json_request(
        Method::POST,
        "/v1/api-keys",
        api_key,
        json!({
            "name": "New Test Key",
            "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
            "permissions": {
                "read": true,
                "write": true,
                "delete": false,
                "admin": false
            }
        }),
    );

    let response = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = http::extract_json_response(response).await;
    let key_id = body.get("id").unwrap().as_str().unwrap().to_string();
    let key_secret = body.get("key").unwrap().as_str().unwrap().to_string();

    // 2. List API keys
    let list_req = http::authenticated_request(Method::GET, "/v1/api-keys", api_key);
    let response = app.clone().oneshot(list_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = http::extract_json_response(response).await;
    let keys = body.get("api_keys").unwrap().as_array().unwrap();
    assert!(keys
        .iter()
        .any(|k| k.get("id").unwrap().as_str().unwrap() == key_id));

    // 3. Update API key
    let update_req = http::authenticated_json_request(
        Method::PUT,
        &format!("/v1/api-keys/{}", key_id),
        api_key,
        json!({
            "name": "Updated Test Key",
            "is_active": false
        }),
    );
    let response = app.clone().oneshot(update_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 4. Use the new (disabled) key - should fail
    let req = http::authenticated_request(Method::GET, "/v1/objects", &key_secret);
    let response = app.clone().oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // 5. Delete API key
    let delete_req =
        http::authenticated_request(Method::DELETE, &format!("/v1/api-keys/{}", key_id), api_key);
    let response = app.clone().oneshot(delete_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // 6. Verify deletion
    let get_req =
        http::authenticated_request(Method::GET, &format!("/v1/api-keys/{}", key_id), api_key);
    let response = app.oneshot(get_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
