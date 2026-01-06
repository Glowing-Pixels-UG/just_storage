//! Integration tests using TestContainers
//!
//! This module provides comprehensive integration testing with real PostgreSQL
//! containers, automatic schema setup, and proper test isolation.

use std::sync::Arc;
use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};
use sqlx::{PgPool, Executor};

use just_storage::application::{
    dto::UploadRequest,
    ports::{BlobRepository, BlobStore, ObjectRepository},
    use_cases::{DeleteObjectUseCase, DownloadObjectUseCase, UploadObjectUseCase},
};
use just_storage::domain::value_objects::StorageClass;

/// Test environment using TestContainers
pub struct TestEnvironment {
    pub pool: PgPool,
    pub object_repo: Arc<dyn ObjectRepository>,
    pub blob_repo: Arc<dyn BlobRepository>,
    pub blob_store: Arc<dyn BlobStore>,
    pub upload_use_case: Arc<UploadObjectUseCase>,
    pub download_use_case: Arc<DownloadObjectUseCase>,
    pub delete_use_case: Arc<DeleteObjectUseCase>,
    _container: testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>,
    _temp_dir: tempfile::TempDir, // Keep temp dir alive
}

impl TestEnvironment {
    /// Create a new test environment with PostgreSQL container
    pub async fn new() -> Self {
        // Start PostgreSQL container with custom schema
        let init_sql = include_str!("../../schema.sql");
        let container = Postgres::default()
            .with_init_sql(init_sql.as_bytes().to_vec())
            .start()
            .await
            .expect("Failed to start PostgreSQL container");

        let host = container.get_host().await.expect("Failed to get container host");
        let port = container
            .get_host_port_ipv4(5432)
            .await
            .expect("Failed to get container port");

        let database_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

        // Create connection pool
        let pool = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        // Setup repositories and use cases
        let object_repo: Arc<dyn ObjectRepository> =
            Arc::new(just_storage::infrastructure::persistence::PostgresObjectRepository::new(
                pool.clone(),
            ));
        let blob_repo: Arc<dyn BlobRepository> =
            Arc::new(just_storage::infrastructure::persistence::PostgresBlobRepository::new(
                pool.clone(),
            ));

        // Create temporary storage directories
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let hot_dir = temp_dir.path().join("hot");
        let cold_dir = temp_dir.path().join("cold");
        std::fs::create_dir_all(&hot_dir).expect("Failed to create hot storage dir");
        std::fs::create_dir_all(&cold_dir).expect("Failed to create cold storage dir");

        let store = just_storage::infrastructure::storage::LocalFilesystemStore::new(
            hot_dir,
            cold_dir,
        );
        store.init().await.expect("Failed to init storage");
        let blob_store: Arc<dyn BlobStore> = Arc::new(store);

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

        Self {
            pool,
            object_repo,
            blob_repo,
            blob_store,
            upload_use_case,
            download_use_case,
            delete_use_case,
            _container: container,
            _temp_dir: temp_dir,
        }
    }

    /// Setup database schema for testing
    async fn setup_schema(pool: &PgPool) {
        let schema = include_str!("../../schema.sql");
        let statements: Vec<&str> = schema
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && !s.starts_with("--"))
            .collect();

        for statement in statements {
            if !statement.trim().is_empty() {
                pool.execute(statement)
                    .await
                    .unwrap_or_else(|e| panic!("Failed to execute schema statement: {}\nStatement: {}", e, statement));
            }
        }
    }

    /// Clean up test data between tests
    pub async fn cleanup(&self) {
        sqlx::query("DELETE FROM audit_logs").execute(&self.pool).await.ok();
        sqlx::query("DELETE FROM api_keys").execute(&self.pool).await.ok();
        sqlx::query("DELETE FROM objects").execute(&self.pool).await.ok();
        sqlx::query("DELETE FROM blobs").execute(&self.pool).await.ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use just_storage::domain::value_objects::Namespace;
    use uuid::Uuid;

    /// Full object lifecycle test using testcontainers
    #[tokio::test]
    async fn test_full_object_lifecycle_with_testcontainers() {
        let env = TestEnvironment::new().await;

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
        let object = env.upload_use_case
            .execute(request, reader)
            .await
            .expect("Upload failed");

        assert_eq!(object.namespace, "test");
        assert_eq!(object.key, Some("test_key_containers".to_string()));
        assert!(object.size_bytes.is_some());
        assert!(object.content_hash.is_some());

        let object_id = object.id.parse().expect("Invalid object ID");

        // Test download
        let (metadata, mut reader) = env.download_use_case
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
        env.delete_use_case
            .execute(&object_id)
            .await
            .expect("Delete failed");

        // Verify object is gone
        let result = env.download_use_case.execute_by_id(&object_id).await;
        assert!(result.is_err());
    }

    /// Test namespace and tenant validation
    #[tokio::test]
    async fn test_namespace_validation_with_testcontainers() {
        let env = TestEnvironment::new().await;

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

        let object = env.upload_use_case
            .execute(request, reader)
            .await
            .expect("Upload with valid namespace should succeed");

        assert_eq!(object.namespace, namespace.to_string());

        // Cleanup
        let object_id = object.id.parse().expect("Invalid object ID");
        env.delete_use_case.execute(&object_id).await.ok();
    }

    /// Test multiple objects in same namespace
    #[tokio::test]
    async fn test_multiple_objects_same_namespace() {
        let env = TestEnvironment::new().await;

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

            let object = env.upload_use_case
                .execute(request, reader)
                .await
                .expect(&format!("Upload failed for {}", filename));

            let object_id = object.id.parse().expect("Invalid object ID");
            object_ids.push(object_id);
        }

        // Verify all objects exist
        for object_id in &object_ids {
            let (metadata, _) = env.download_use_case
                .execute_by_id(object_id)
                .await
                .expect(&format!("Download failed for object {}", object_id));

            // Verify the object exists and has the expected size/content_hash
            assert!(metadata.size_bytes > 0);
            assert!(!metadata.content_hash.is_empty());
        }

        // Cleanup
        for object_id in object_ids {
            env.delete_use_case.execute(&object_id).await.ok();
        }
    }

    /// Test storage class behavior
    #[tokio::test]
    async fn test_storage_class_behavior() {
        let env = TestEnvironment::new().await;

        let test_data = b"Storage class test";
        let reader = Box::pin(std::io::Cursor::new(test_data));

        let request = UploadRequest {
            namespace: "storage_test".to_string(),
            tenant_id: Uuid::new_v4().to_string(),
            key: Some("storage_class_file".to_string()),
            storage_class: Some(StorageClass::Cold), // Test cold storage
        };

        let object = env.upload_use_case
            .execute(request, reader)
            .await
            .expect("Upload to cold storage failed");

        // Verify object was created with correct storage class
        assert_eq!(object.storage_class, StorageClass::Cold);

        // Cleanup
        let object_id = object.id.parse().expect("Invalid object ID");
        env.delete_use_case.execute(&object_id).await.ok();
    }
}