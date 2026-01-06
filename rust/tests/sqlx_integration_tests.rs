//! Integration tests using SQLx's #[sqlx::test] macro
//!
//! These tests automatically create and manage isolated test databases,
//! providing better test isolation and reliability than manual database setup.

use std::sync::Arc;
use tempfile::TempDir;
use sqlx::PgPool;

use just_storage::application::{
    dto::UploadRequest,
    ports::{BlobRepository, BlobStore, ObjectRepository},
    use_cases::{DeleteObjectUseCase, DownloadObjectUseCase, UploadObjectUseCase},
};
use just_storage::domain::value_objects::StorageClass;
use just_storage::infrastructure::{
    persistence::{PostgresBlobRepository, PostgresObjectRepository},
    storage::LocalFilesystemStore,
};



/// Helper function to setup test environment for #[sqlx::test] functions
/// Schema is automatically loaded via SQLx fixtures
async fn setup_test_environment(pool: &PgPool) -> (Arc<UploadObjectUseCase>, Arc<DownloadObjectUseCase>, Arc<DeleteObjectUseCase>, TempDir) {

    // Setup storage
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let hot_dir = temp_dir.path().join("hot");
    let cold_dir = temp_dir.path().join("cold");
    std::fs::create_dir_all(&hot_dir).expect("Failed to create hot storage dir");
    std::fs::create_dir_all(&cold_dir).expect("Failed to create cold storage dir");

    let store = LocalFilesystemStore::new(hot_dir, cold_dir);
    store.init().await.expect("Failed to init storage");
    let blob_store: Arc<dyn BlobStore> = Arc::new(store);

    // Setup repositories
    let object_repo: Arc<dyn ObjectRepository> =
        Arc::new(PostgresObjectRepository::new(pool.clone()));
    let blob_repo: Arc<dyn BlobRepository> =
        Arc::new(PostgresBlobRepository::new(pool.clone()));

    // Setup use cases
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

    (upload_use_case, download_use_case, delete_use_case, temp_dir)
}

    #[sqlx::test]
    async fn sqlx_test_basic_upload_download(pool: PgPool) {
        let (upload_use_case, download_use_case, delete_use_case, _temp_dir) = setup_test_environment(&pool).await;

    // Test data
    let test_data = b"Hello, SQLx Test!";
    let reader = Box::pin(std::io::Cursor::new(test_data));

    let request = UploadRequest {
        namespace: "sqlx_test".to_string(),
        tenant_id: uuid::Uuid::new_v4().to_string(),
        key: Some("basic_file.txt".to_string()),
        storage_class: Some(StorageClass::Hot),
    };

    // Upload
    let object = upload_use_case
        .execute(request, reader)
        .await
        .expect("Upload failed");

    assert_eq!(object.namespace, "sqlx_test");
    assert_eq!(object.key, Some("basic_file.txt".to_string()));

    // Download and verify
    let object_id = object.id.parse().expect("Invalid object ID");
    let (metadata, mut reader) = download_use_case
        .execute_by_id(&object_id)
        .await
        .expect("Download failed");

    assert_eq!(metadata.size_bytes, test_data.len() as u64);

    let mut downloaded = Vec::new();
    tokio::io::copy(&mut reader, &mut downloaded).await.expect("Failed to read data");
    assert_eq!(&downloaded[..], test_data);

    // Cleanup
    delete_use_case.execute(&object_id).await.expect("Delete failed");
}

    #[sqlx::test]
    async fn sqlx_test_object_listing(pool: PgPool) {
        let (upload_use_case, download_use_case, delete_use_case, _temp_dir) = setup_test_environment(&pool).await;

    let namespace = "list_test";
    let tenant_id = uuid::Uuid::new_v4();

    // Upload multiple objects
    let mut object_ids = Vec::new();
    for i in 0..3 {
        let request = UploadRequest {
            namespace: namespace.to_string(),
            tenant_id: tenant_id.to_string(),
            key: Some(format!("file_{}.txt", i)),
            storage_class: Some(StorageClass::Hot),
        };

        let test_data = format!("Content {}", i).into_bytes();
        let reader = Box::pin(std::io::Cursor::new(test_data));

        let object = upload_use_case.execute(request, reader).await.expect("Upload failed");
        let object_id = object.id.parse().expect("Invalid object ID");
        object_ids.push(object_id);
    }

    // List objects (this would require adding a list use case if not already present)
    // For now, just verify they exist by downloading
    for object_id in &object_ids {
        let (metadata, _) = download_use_case
            .execute_by_id(object_id)
            .await
            .expect(&format!("Object {} should exist", object_id));

        // Verify the object exists and has the expected size/content_hash
        assert!(metadata.size_bytes > 0);
        assert!(!metadata.content_hash.is_empty());
    }

    // Cleanup
    for object_id in object_ids {
        delete_use_case.execute(&object_id).await.ok();
    }
}

    #[sqlx::test]
    async fn sqlx_test_storage_classes(pool: PgPool) {
    let (upload_use_case, download_use_case, delete_use_case, _temp_dir) = setup_test_environment(&pool).await;

    let tenant_id = uuid::Uuid::new_v4();

    // Test both storage classes
    for storage_class in [StorageClass::Hot, StorageClass::Cold] {
        let request = UploadRequest {
            namespace: "storage_class_test".to_string(),
            tenant_id: tenant_id.to_string(),
            key: Some(format!("file_{:?}.txt", storage_class)),
            storage_class: Some(storage_class),
        };

        let test_data = format!("Data for {:?}", storage_class).into_bytes();
        let reader = Box::pin(std::io::Cursor::new(test_data));

        let object = upload_use_case.execute(request, reader).await.expect("Upload failed");
        assert_eq!(object.storage_class, storage_class);

        // Cleanup
        let object_id = object.id.parse().expect("Invalid object ID");
        delete_use_case.execute(&object_id).await.ok();
    }
}

    #[sqlx::test]
    async fn sqlx_test_error_cases(pool: PgPool) {
        let (upload_use_case, _download_use_case, _delete_use_case, _temp_dir) = setup_test_environment(&pool).await;

    // Test invalid namespace
    let request = UploadRequest {
        namespace: "".to_string(), // Invalid empty namespace
        tenant_id: uuid::Uuid::new_v4().to_string(),
        key: Some("error_test.txt".to_string()),
        storage_class: Some(StorageClass::Hot),
    };

    let test_data = b"Error test data";
    let reader = Box::pin(std::io::Cursor::new(test_data));

    let result = upload_use_case.execute(request, reader).await;
    assert!(result.is_err(), "Should fail with invalid namespace");
}

    #[sqlx::test]
    async fn sqlx_test_large_file_handling(pool: PgPool) {
        let (upload_use_case, download_use_case, delete_use_case, _temp_dir) = setup_test_environment(&pool).await;

    // Create a larger test file (1MB)
    let test_data = vec![0u8; 1024 * 1024];
    let reader = Box::pin(std::io::Cursor::new(test_data.clone()));

    let request = UploadRequest {
        namespace: "large_file_test".to_string(),
        tenant_id: uuid::Uuid::new_v4().to_string(),
        key: Some("large_file.dat".to_string()),
        storage_class: Some(StorageClass::Hot),
    };

    // Upload large file
    let object = upload_use_case.execute(request, reader).await.expect("Large file upload failed");
    assert_eq!(object.size_bytes, Some(1024 * 1024));

    // Download and verify
    let object_id = object.id.parse().expect("Invalid object ID");
    let (metadata, mut reader) = download_use_case.execute_by_id(&object_id).await.expect("Download failed");

    assert_eq!(metadata.size_bytes, 1024 * 1024);

    let mut downloaded = Vec::new();
    tokio::io::copy(&mut reader, &mut downloaded).await.expect("Failed to read large file");
    assert_eq!(downloaded.len(), 1024 * 1024);

    // Cleanup
    delete_use_case.execute(&object_id).await.ok();
}

    #[sqlx::test]
    async fn sqlx_test_concurrent_operations(pool: PgPool) {
        let (upload_use_case, download_use_case, delete_use_case, _temp_dir) = setup_test_environment(&pool).await;

    let namespace = "concurrent_test";
    let tenant_id = uuid::Uuid::new_v4();

    // Upload multiple files concurrently
    let mut handles = Vec::new();
    let mut expected_objects = Vec::new();

    for i in 0..5 {
        let upload_uc = Arc::clone(&upload_use_case);
        let ns = namespace.to_string();
        let tid = tenant_id.to_string();
        let key = format!("concurrent_{}.txt", i);
        let data = format!("Concurrent data {}", i).into_bytes();

        let handle = tokio::spawn(async move {
            let request = UploadRequest {
                namespace: ns,
                tenant_id: tid,
                key: Some(key.clone()),
                storage_class: Some(StorageClass::Hot),
            };

            let reader = Box::pin(std::io::Cursor::new(data));
            let object = upload_uc.execute(request, reader).await.expect("Concurrent upload failed");
            (key, object.id.parse().expect("Invalid object ID"))
        });

        handles.push(handle);
    }

    // Wait for all uploads to complete
    for handle in handles {
        let (key, object_id) = handle.await.expect("Task failed");
        expected_objects.push((key, object_id));
    }

    // Verify all objects exist
    for (_key, object_id) in expected_objects {
        let (metadata, _) = download_use_case.execute_by_id(&object_id).await
            .expect(&format!("Object {} should exist", object_id));

        // Verify the object exists and has the expected size/content_hash
        assert!(metadata.size_bytes > 0);
        assert!(!metadata.content_hash.is_empty());

        // Cleanup
        delete_use_case.execute(&object_id).await.ok();
    }
}