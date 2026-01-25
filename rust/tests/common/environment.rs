//! Test environment with builder pattern (Phase 1 skeleton)

use sqlx::PgPool;
use std::sync::Arc;
use tempfile::TempDir;
use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};

use just_storage::application::ports::{BlobRepository, BlobStore, ObjectRepository};
use just_storage::infrastructure::{
    persistence::{PostgresBlobRepository, PostgresObjectRepository},
    storage::LocalFilesystemStore,
};

use super::database::{cleanup_test_data, setup_test_database, setup_test_storage};

/// A single source of truth TestEnvironment used by tests
pub struct TestEnvironment {
    pub pool: PgPool,
    pub object_repo: Arc<dyn ObjectRepository>,
    pub blob_repo: Arc<dyn BlobRepository>,
    pub blob_store: Arc<dyn BlobStore>,
    pub hot_dir: TempDir,
    pub cold_dir: TempDir,
    _container: testcontainers::ContainerAsync<Postgres>,
}

impl TestEnvironment {
    /// Create a full environment using TestContainers and local storage
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

        // Ensure DB is clean before returning
        cleanup_test_data(&pool).await;

        Self {
            pool,
            object_repo,
            blob_repo,
            blob_store,
            hot_dir,
            cold_dir,
            _container: container,
        }
    }

    /// A simple builder entry point for future configuration
    pub fn builder() -> TestEnvironmentBuilder {
        TestEnvironmentBuilder::default()
    }
}

/// Builder skeleton for phased migration
#[derive(Default)]
pub struct TestEnvironmentBuilder {
    pub with_database: bool,
    pub with_use_cases: bool,
    pub with_api_server: bool,
}

impl TestEnvironmentBuilder {
    pub fn with_database(mut self, v: bool) -> Self {
        self.with_database = v;
        self
    }

    pub fn with_use_cases(mut self, v: bool) -> Self {
        self.with_use_cases = v;
        self
    }

    pub fn with_api_server(mut self, v: bool) -> Self {
        self.with_api_server = v;
        self
    }

    /// Build the TestEnvironment according to the flags
    pub async fn build(self) -> TestEnvironment {
        // For Phase 1 we only support database-backed environments
        // Future steps will use flags to enable use-cases and API server
        if self.with_database || (!self.with_use_cases && !self.with_api_server) {
            TestEnvironment::new().await
        } else {
            // Fall back to the default full environment
            TestEnvironment::new().await
        }
    }
}
