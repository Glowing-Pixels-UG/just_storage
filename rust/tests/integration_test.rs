use std::sync::Arc;

use just_storage::{
    application::{
        dto::UploadRequest,
        ports::{BlobRepository, BlobStore, ObjectRepository},
        use_cases::{DeleteObjectUseCase, DownloadObjectUseCase, UploadObjectUseCase},
    },
    domain::value_objects::StorageClass,
    infrastructure::{
        persistence::{PostgresBlobRepository, PostgresObjectRepository},
        storage::LocalFilesystemStore,
    },
};

#[tokio::test]
#[ignore] // Requires database and filesystem
async fn test_full_lifecycle() {
    // Setup
    let pool = sqlx::PgPool::connect("postgres://postgres:password@localhost/activestorage_test")
        .await
        .expect("Failed to connect to test database");

    let object_repo: Arc<dyn ObjectRepository> =
        Arc::new(PostgresObjectRepository::new(pool.clone()));
    let blob_repo: Arc<dyn BlobRepository> = Arc::new(PostgresBlobRepository::new(pool.clone()));

    let store = LocalFilesystemStore::new(
        std::path::PathBuf::from("/tmp/test_hot"),
        std::path::PathBuf::from("/tmp/test_cold"),
    );

    // Initialize storage
    store.init().await.expect("Failed to init storage");

    let blob_store: Arc<dyn BlobStore> = Arc::new(store);

    // Create use cases
    let upload_use_case = Arc::new(UploadObjectUseCase::new(
        Arc::clone(&object_repo),
        Arc::clone(&blob_repo),
        Arc::clone(&blob_store),
    ));

    let download_use_case = Arc::new(DownloadObjectUseCase::new(
        Arc::clone(&object_repo),
        Arc::clone(&blob_store),
    ));

    let delete_use_case = Arc::new(DeleteObjectUseCase::new(
        Arc::clone(&object_repo),
        Arc::clone(&blob_repo),
        Arc::clone(&blob_store),
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
