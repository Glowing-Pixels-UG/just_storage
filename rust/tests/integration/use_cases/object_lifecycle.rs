//! Full object lifecycle integration tests (upload → download → delete)

use std::sync::Arc;

#[path = "../../common/environment.rs"]
mod env;

use just_storage::application::{
    dto::UploadRequest,
    use_cases::{DeleteObjectUseCase, DownloadObjectUseCase, UploadObjectUseCase},
};
use just_storage::domain::value_objects::StorageClass;
use uuid::Uuid;

#[tokio::test]
async fn test_full_object_lifecycle_with_testcontainers() {
    let common_env = env::TestEnvironment::builder()
        .with_database(true)
        .build()
        .await;

    // Create use cases using common environment components
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

    // Test data
    let test_data = b"Hello, TestContainers Storage!";
    let reader = Box::pin(std::io::Cursor::new(test_data));

    let request = UploadRequest {
        namespace: "test".to_string(),
        tenant_id: Uuid::new_v4().to_string(),
        key: Some("test_key_containers".to_string()),
        storage_class: Some(StorageClass::Hot),
    };

    // Test upload
    let object = upload_use_case
        .execute(request, reader)
        .await
        .expect("Upload failed");

    assert_eq!(object.namespace, "test");
    assert_eq!(object.key, Some("test_key_containers".to_string()));
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
