//! Namespace and tenant validation integration tests

use crate::common::environment as env;
use std::sync::Arc;

use just_storage::application::{
    dto::UploadRequest,
    use_cases::{DeleteObjectUseCase, DownloadObjectUseCase, UploadObjectUseCase},
};
use just_storage::domain::value_objects::Namespace;
use just_storage::domain::value_objects::StorageClass;
use uuid::Uuid;

#[tokio::test]
async fn test_namespace_validation_with_testcontainers() {
    let common_env = env::TestEnvironment::builder()
        .with_database(true)
        .build()
        .await;

    let upload_use_case = Arc::new(UploadObjectUseCase::new(
        Arc::clone(&common_env.object_repo),
        Arc::clone(&common_env.blob_repo),
        Arc::clone(&common_env.blob_store),
    ));

    let download_use_case = Arc::new(DownloadObjectUseCase::new(
        Arc::clone(&common_env.object_repo),
        Arc::clone(&common_env.blob_store),
    ));

    let delete_use_case = Arc::new(DeleteObjectUseCase::new(
        Arc::clone(&common_env.object_repo),
        Arc::clone(&common_env.blob_repo),
        Arc::clone(&common_env.blob_store),
    ));

    // Test valid namespace and tenant
    let namespace = Namespace::new("valid_namespace".to_string()).unwrap();
    let tenant_id = Uuid::new_v4();

    let request = UploadRequest {
        namespace: namespace.to_string(),
        tenant_id: tenant_id.to_string(),
        key: Some("validation_test".to_string()),
        storage_class: Some(StorageClass::Cold),
    };

    let test_data = b"Validation test data";
    let reader = Box::pin(std::io::Cursor::new(test_data));

    let object = upload_use_case
        .execute(request, reader)
        .await
        .expect("Upload with valid namespace should succeed");

    assert_eq!(object.namespace, namespace.to_string());

    // Cleanup
    let object_id = object.id.parse().expect("Invalid object ID");
    delete_use_case.execute(&object_id).await.ok();
}
