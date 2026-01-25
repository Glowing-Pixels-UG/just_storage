//! Shared test fixtures and utilities for all test types
//!
//! This module provides common test setup patterns to reduce duplication
//! and make tests more maintainable.

use sqlx::{Executor, PgPool};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};
use uuid::Uuid;

use just_storage::application::ports::{
    ApiKeyRepository, AuditRepository, BlobRepository, BlobStore, ObjectRepository,
};
use just_storage::domain::entities::{Blob, Object};
use just_storage::domain::value_objects::{
    ContentHash, Namespace, ObjectId, StorageClass, TenantId,
};
use just_storage::infrastructure::{
    persistence::{PostgresBlobRepository, PostgresObjectRepository},
    storage::LocalFilesystemStore,
};

/// Test environment container with all necessary components
pub struct TestEnvironment {
    pub pool: PgPool,
    pub object_repo: Arc<dyn ObjectRepository>,
    pub blob_repo: Arc<dyn BlobRepository>,
    pub blob_store: Arc<dyn BlobStore>,
    pub hot_dir: TempDir,
    pub cold_dir: TempDir,
    pub api_key_repo: Option<Arc<dyn ApiKeyRepository>>,
    pub audit_repo: Option<Arc<dyn AuditRepository>>,
    _container: testcontainers::ContainerAsync<Postgres>,
}

impl TestEnvironment {
    /// Create a complete test environment with database and storage
    pub async fn new() -> Self {
        let (pool, container) = setup_test_database().await;
        let (hot_dir, cold_dir) = setup_test_storage();

        let object_repo: Arc<dyn ObjectRepository> =
            Arc::new(PostgresObjectRepository::new(pool.clone()));
        let blob_repo: Arc<dyn BlobRepository> =
            Arc::new(PostgresBlobRepository::new(pool.clone()));

        let store =
            LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());
        store.init().await.expect("Failed to init storage");
        let blob_store: Arc<dyn BlobStore> = Arc::new(store);

        Self {
            pool,
            object_repo,
            blob_repo,
            blob_store,
            hot_dir,
            cold_dir,
            api_key_repo: None,
            audit_repo: None,
            _container: container,
        }
    }

    /// Add API key repository to the environment
    pub fn with_api_key_repo(mut self, repo: Arc<dyn ApiKeyRepository>) -> Self {
        self.api_key_repo = Some(repo);
        self
    }

    /// Add audit repository to the environment
    pub fn with_audit_repo(mut self, repo: Arc<dyn AuditRepository>) -> Self {
        self.audit_repo = Some(repo);
        self
    }
}

/// Database setup utilities using testcontainers
pub async fn setup_test_database() -> (PgPool, testcontainers::ContainerAsync<Postgres>) {
    // Start PostgreSQL container with schema initialization
    let init_sql = include_str!("../../schema.sql");
    let container = Postgres::default()
        .with_init_sql(init_sql.as_bytes().to_vec())
        .start()
        .await
        .expect("Failed to start PostgreSQL container");

    let host = container
        .get_host()
        .await
        .expect("Failed to get container host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get container port");

    let database_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

    // Create connection pool
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Clean up any existing test data
    cleanup_test_data(&pool).await;

    (pool, container)
}

/// Clean up test data between tests
pub async fn cleanup_test_data(pool: &PgPool) {
    sqlx::query("DELETE FROM audit_logs")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM api_keys").execute(pool).await.ok();
    sqlx::query("DELETE FROM objects").execute(pool).await.ok();
    sqlx::query("DELETE FROM blobs").execute(pool).await.ok();
}

/// Storage setup utilities
pub fn setup_test_storage() -> (TempDir, TempDir) {
    let hot_dir = TempDir::new().expect("Failed to create temp hot storage dir");
    let cold_dir = TempDir::new().expect("Failed to create temp cold storage dir");
    (hot_dir, cold_dir)
}

/// Test data factories
pub mod factories {
    use super::*;
    use just_storage::domain::entities::Object;
    use just_storage::domain::value_objects::{Namespace, ObjectId, ObjectStatus, TenantId};

    /// Create a test object with default values
    pub fn create_test_object() -> Object {
        let mut obj = Object::new(
            Namespace::new("test".to_string()).unwrap(),
            TenantId::new(Uuid::new_v4()),
            Some("test_key".to_string()),
            StorageClass::Hot,
        );
        // Commit the object to set content hash and size
        let content_hash =
            ContentHash::from_hex("testhash12345678901234567890123456789012".to_string()).unwrap();
        obj.commit(&content_hash, 1024).unwrap();
        obj.set_content_type("application/json".to_string());
        // Metadata is already initialized with default values
        obj
    }

    /// Create a test object with custom parameters
    pub fn create_custom_object(
        namespace: &str,
        tenant_id: &str,
        key: Option<&str>,
        storage_class: StorageClass,
        status: just_storage::domain::value_objects::ObjectStatus,
        content_hash: &str,
        size_bytes: Option<u64>,
    ) -> Object {
        let uuid = Uuid::parse_str(tenant_id).unwrap_or_else(|_| Uuid::new_v4());
        let mut obj = Object::new(
            Namespace::new(namespace.to_string()).unwrap(),
            TenantId::new(uuid),
            key.map(|s| s.to_string()),
            storage_class,
        );

        // If the object should be committed, commit it with the provided hash and size
        if status == just_storage::domain::value_objects::ObjectStatus::Committed {
            if let Some(size) = size_bytes {
                let hash = ContentHash::from_hex(content_hash.to_string()).unwrap();
                obj.commit(&hash, size).unwrap();
            }
        }

        obj.set_content_type("application/octet-stream".to_string());
        obj
    }

    /// Create a test blob
    pub fn create_test_blob(content_hash: &ContentHash, storage_class: StorageClass) -> Blob {
        Blob::new(
            content_hash.clone(),
            storage_class,
            1024, // size_bytes
        )
    }
}

#[path = "common/mod.rs"]
mod common;

pub use common::http;
pub use common::mocks;
pub use crate::common::assertions;