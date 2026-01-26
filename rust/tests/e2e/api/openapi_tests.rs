use axum::http::StatusCode;

use crate::common::{environment as env, http};

#[tokio::test]
async fn openapi_spec_contains_objects_path() {
    let (app, _container, _temp_dir) = env::setup_test_api_server().await;

    let req = http::get_request("/api-docs/openapi.json");

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let json = http::extract_json_response(response).await;
    assert!(json["openapi"].is_string());
    assert!(json["paths"].is_object());
    assert!(json["paths"]
        .as_object()
        .unwrap()
        .contains_key("/v1/objects"));
}
