//! Multiple objects in same namespace integration tests

use std::sync::Arc;

#[path = "../../common/environment.rs"]
mod env;

use just_storage::application::{
    dto::UploadRequest,
    use_cases::{DeleteObjectUseCase, DownloadObjectUseCase, UploadObjectUseCase},
};
use just_storage::domain::value_objects::StorageClass;

#[tokio::test]
async fn test_multiple_objects_same_namespace() {
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

    let namespace = "multi_test";
    let tenant_id = uuid::Uuid::new_v4();

    // Upload multiple objects
    let objects = vec!["file1.txt", "file2.txt", "file3.txt"];
    let mut object_ids = Vec::new();

    for filename in &objects {
        let request = UploadRequest {
            namespace: namespace.to_string(),
            tenant_id: tenant_id.to_string(),
            key: Some(filename.to_string()),
            storage_class: Some(StorageClass::Hot),
        };

        let test_data = format!("Content of {}", filename).into_bytes();
        let reader = Box::pin(std::io::Cursor::new(test_data));

        let object = upload_use_case
            .execute(request, reader)
            .await
            .expect(&format!("Upload failed for {}", filename));

        let object_id = object.id.parse().expect("Invalid object ID");
        object_ids.push(object_id);
    }

    // Verify all objects exist
    for object_id in &object_ids {
        let (metadata, _) = download_use_case
            .execute_by_id(object_id)
            .await
            .expect(&format!("Download failed for object {}", object_id));

        // Verify the object exists and has the expected size/content_hash
        assert!(metadata.size_bytes > 0);
        assert!(!metadata.content_hash.is_empty());
    }

    // Cleanup
    for object_id in object_ids {
        delete_use_case.execute(&object_id).await.ok();
    }
}
