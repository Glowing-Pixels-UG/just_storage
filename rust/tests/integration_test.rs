use std::sync::Arc;

use just_storage::{
    application::{
        dto::UploadRequest,
        use_cases::{DeleteObjectUseCase, DownloadObjectUseCase, UploadObjectUseCase},
    },
    domain::value_objects::StorageClass,
};

// Import shared test fixtures
mod common;
use common::{assertions, TestEnvironment};

// Import enhanced test environments
mod api_endpoint_tests;
mod testcontainers_integration;

#[tokio::test]
async fn test_full_lifecycle() {
    // Setup test environment using shared fixtures
    let env = TestEnvironment::builder().with_database(true).build().await;

    // Create use cases using environment components
    let upload_use_case = Arc::new(UploadObjectUseCase::new(
        Arc::clone(&env.object_repo),
        Arc::clone(&env.blob_repo),
        Arc::clone(&env.blob_store),
    ));

    let download_use_case = Arc::new(DownloadObjectUseCase::new(
        Arc::clone(&env.object_repo),
        Arc::clone(&env.blob_store),
    ));

    let delete_use_case = Arc::new(DeleteObjectUseCase::new(
        Arc::clone(&env.object_repo),
        Arc::clone(&env.blob_repo),
        Arc::clone(&env.blob_store),
    ));

    // Test data
    let test_data = b"Hello, ActiveStorage!";
    let reader = Box::pin(std::io::Cursor::new(test_data));

    let request = UploadRequest {
        namespace: "test".to_string(),
        tenant_id: uuid::Uuid::new_v4().to_string(),
        key: Some("test_key".to_string()),
        storage_class: Some(StorageClass::Hot),
    };

    // Test upload
    let object = upload_use_case
        .execute(request, reader)
        .await
        .expect("Upload failed");

    assert_eq!(object.namespace, "test");
    assert_eq!(object.key, Some("test_key".to_string()));
    assert!(object.size_bytes.is_some());
    assert!(object.content_hash.is_some());

    let object_id = object.id.parse().expect("Invalid object ID");

    // Test download
    let (metadata, mut reader) = download_use_case
        .execute_by_id(&object_id)
        .await
        .expect("Download failed");

    assert_eq!(metadata.size_bytes, test_data.len() as u64);

    // Read and verify data
    let mut downloaded = Vec::new();
    tokio::io::copy(&mut reader, &mut downloaded)
        .await
        .expect("Failed to read downloaded data");

    assert_eq!(&downloaded[..], test_data);

    // Test delete
    delete_use_case
        .execute(&object_id)
        .await
        .expect("Delete failed");

    // Verify object is gone
    let result = download_use_case.execute_by_id(&object_id).await;
    assert!(result.is_err());
}

#[cfg(test)]
mod api_tests {
    use super::*;
    use crate::common::{assertions, http};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use just_storage::api::create_router;
    use just_storage::ApplicationBuilder;
    use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};
    use tower::ServiceExt;

    async fn setup_test_server() -> (
        axum::Router,
        testcontainers::ContainerAsync<Postgres>,
        tempfile::TempDir,
    ) {
        common::setup_test_api_server().await
    }

    #[tokio::test]
    async fn test_health_endpoints() {
        let (app, _container, _temp_dir) = setup_test_server().await;

        // Test health endpoint
        let req = Request::builder()
            .uri("/health")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "healthy");

        // Test readiness endpoint
        let req = Request::builder()
            .uri("/health/ready")
            .method("GET")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_openapi_endpoint() {
        let (app, _container, _temp_dir) = setup_test_server().await;

        let req = http::get_request("/api-docs/openapi.json");
        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["openapi"].is_string());
        assert!(json["paths"].is_object());
    }

    #[tokio::test]
    async fn test_object_operations_without_auth() {
        let (app, _container, _temp_dir) = setup_test_server().await;

        // Test upload without auth
        let req = http::post_request("/v1/objects", serde_json::json!({}));
        let response = app.oneshot(req).await.unwrap();
        assertions::assert_error_response(response, StatusCode::UNAUTHORIZED).await;
    }

    #[tokio::test]
    async fn test_api_key_management_without_auth() {
        let (app, _container, _temp_dir) = setup_test_server().await;

        // Test list API keys without auth
        let req = http::get_request("/v1/api-keys");
        let response = app.oneshot(req).await.unwrap();
        assertions::assert_error_response(response, StatusCode::UNAUTHORIZED).await;
    }

    // TODO: Add tests with proper authentication once API key creation is implemented
    // #[tokio::test]
    // async fn test_full_object_lifecycle_with_auth() {
    //     let (_addr, app) = setup_test_server().await;
    //
    //     // Create API key
    //     // Upload object
    //     // Download object
    //     // List objects
    //     // Delete object
    // }
}
