//! Storage class behavior integration tests

use crate::common::environment as env;
use std::sync::Arc;

use just_storage::application::{
    dto::UploadRequest,
    use_cases::{DeleteObjectUseCase, DownloadObjectUseCase, UploadObjectUseCase},
};
use just_storage::domain::value_objects::StorageClass;
use uuid::Uuid;

#[tokio::test]
async fn test_storage_class_behavior() {
    let common_env = env::TestEnvironment::builder()
        .with_database(true)
        .build()
        .await;

    let upload_use_case = Arc::new(UploadObjectUseCase::new(
        Arc::clone(&common_env.object_repo),
        Arc::clone(&common_env.blob_repo),
        Arc::clone(&common_env.blob_store),
    ));

    let _download_use_case = Arc::new(DownloadObjectUseCase::new(
        Arc::clone(&common_env.object_repo),
        Arc::clone(&common_env.blob_store),
    ));

    let delete_use_case = Arc::new(DeleteObjectUseCase::new(
        Arc::clone(&common_env.object_repo),
        Arc::clone(&common_env.blob_repo),
        Arc::clone(&common_env.blob_store),
    ));

    let test_data = b"Storage class test";
    let reader = Box::pin(std::io::Cursor::new(test_data));

    let request = UploadRequest {
        namespace: "storage_test".to_string(),
        tenant_id: Uuid::new_v4().to_string(),
        key: Some("storage_class_file".to_string()),
        storage_class: Some(StorageClass::Cold), // Test cold storage
    };

    let object = upload_use_case
        .execute(request, reader)
        .await
        .expect("Upload to cold storage failed");

    // Verify object was created with correct storage class
    assert_eq!(object.storage_class, StorageClass::Cold);

    // Cleanup
    let object_id = object.id.parse().expect("Invalid object ID");
    delete_use_case.execute(&object_id).await.ok();
}
