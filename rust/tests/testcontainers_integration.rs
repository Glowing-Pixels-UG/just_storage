//! Integration tests using TestContainers
//!
//! This module provides comprehensive integration testing with real PostgreSQL
//! containers, automatic schema setup, and proper test isolation.

use sqlx::{Executor, PgPool};
use std::sync::Arc;
use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};

#[path = "common/environment.rs"]
mod env;

use just_storage::application::{
    dto::UploadRequest,
    ports::{BlobRepository, BlobStore, ObjectRepository},
    use_cases::{DeleteObjectUseCase, DownloadObjectUseCase, UploadObjectUseCase},
};
use just_storage::domain::value_objects::StorageClass;

// We use the shared TestEnvironment, and construct use-cases locally
// to keep a compatibility layer while consolidating infrastructure.

#[cfg(test)]
mod tests {
    use super::*;
    use just_storage::domain::value_objects::Namespace;
    use uuid::Uuid;

    /// Full object lifecycle test using testcontainers
    #[tokio::test]
    async fn test_full_object_lifecycle_with_testcontainers() {
        let common_env = env::TestEnvironment::new().await;

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

    /// Test namespace and tenant validation
    #[tokio::test]
    async fn test_namespace_validation_with_testcontainers() {
        let common_env = env::TestEnvironment::new().await;

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

    /// Test multiple objects in same namespace
    #[tokio::test]
    async fn test_multiple_objects_same_namespace() {
        let common_env = env::TestEnvironment::new().await;

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
        let tenant_id = Uuid::new_v4();

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

    /// Test storage class behavior
    #[tokio::test]
    async fn test_storage_class_behavior() {
        let common_env = env::TestEnvironment::new().await;

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
}
