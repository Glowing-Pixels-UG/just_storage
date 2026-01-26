#![allow(dead_code, unused_imports)]

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

// Use-case types for optional wiring by the TestEnvironmentBuilder
use just_storage::application::use_cases::{
    DeleteObjectUseCase, DownloadObjectUseCase, UploadObjectUseCase,
};

// Inline minimal DB & storage helpers to avoid fragile module resolution during phased migration
async fn start_postgres_with_schema() -> (PgPool, testcontainers::ContainerAsync<Postgres>) {
    let init_sql = include_str!("../../../schema.sql");
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

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Clean up any existing test data
    sqlx::query("DELETE FROM audit_logs")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM api_keys")
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM objects").execute(&pool).await.ok();
    sqlx::query("DELETE FROM blobs").execute(&pool).await.ok();

    (pool, container)
}

fn create_temp_storage_dirs() -> (tempfile::TempDir, tempfile::TempDir) {
    let hot_dir = tempfile::TempDir::new().expect("Failed to create temp hot storage dir");
    let cold_dir = tempfile::TempDir::new().expect("Failed to create temp cold storage dir");
    (hot_dir, cold_dir)
}

/// A single source of truth TestEnvironment used by tests
pub struct TestEnvironment {
    pub pool: PgPool,
    pub object_repo: Arc<dyn ObjectRepository>,
    pub blob_repo: Arc<dyn BlobRepository>,
    pub blob_store: Arc<dyn BlobStore>,
    pub hot_dir: TempDir,
    pub cold_dir: TempDir,
    _container: testcontainers::ContainerAsync<Postgres>,

    // Optional higher-level helpers created by the builder
    pub upload_use_case: Option<std::sync::Arc<UploadObjectUseCase>>,
    pub download_use_case: Option<std::sync::Arc<DownloadObjectUseCase>>,
    pub delete_use_case: Option<std::sync::Arc<DeleteObjectUseCase>>,
    pub api_router: Option<axum::Router>,
    pub api_container:
        Option<testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>>,
    pub api_temp_dir: Option<tempfile::TempDir>,
}

impl TestEnvironment {
    /// Create a full environment using TestContainers and local storage
    pub async fn new() -> Self {
        let (pool, container) = start_postgres_with_schema().await;
        let (hot_dir, cold_dir) = create_temp_storage_dirs();

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
            _container: container,

            upload_use_case: None,
            download_use_case: None,
            delete_use_case: None,
            api_router: None,
            api_container: None,
            api_temp_dir: None,
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
        // For Phase 1 we use the DB-backed environment as the base
        let mut env = TestEnvironment::new().await;

        // Wire use-cases if requested
        if self.with_use_cases {
            let upload_uc = std::sync::Arc::new(UploadObjectUseCase::new(
                std::sync::Arc::clone(&env.object_repo),
                std::sync::Arc::clone(&env.blob_repo),
                std::sync::Arc::clone(&env.blob_store),
            ));

            let download_uc = std::sync::Arc::new(DownloadObjectUseCase::new(
                std::sync::Arc::clone(&env.object_repo),
                std::sync::Arc::clone(&env.blob_store),
            ));

            let delete_uc = std::sync::Arc::new(DeleteObjectUseCase::new(
                std::sync::Arc::clone(&env.object_repo),
                std::sync::Arc::clone(&env.blob_repo),
                std::sync::Arc::clone(&env.blob_store),
            ));

            env.upload_use_case = Some(upload_uc);
            env.download_use_case = Some(download_uc);
            env.delete_use_case = Some(delete_uc);
        }

        // Start a lightweight API server for testing if requested (separate container)
        if self.with_api_server {
            let (router, container, temp_dir) = setup_test_api_server().await;
            env.api_router = Some(router);
            env.api_container = Some(container);
            env.api_temp_dir = Some(temp_dir);
        }

        env
    }
}

/// Helper to create an API server (Router) wired with application state for tests
/// Returns (Router, container, temp_dir) where `temp_dir` must be kept alive by caller
pub async fn setup_test_api_server() -> (
    axum::Router,
    testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>,
    tempfile::TempDir,
) {
    use just_storage::{api::create_router, ApplicationBuilder, Config};

    // Start PostgreSQL container (migrations will be run by ApplicationBuilder)
    let container = Postgres::default()
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

    // Create test config
    let mut config = Config::from_env();
    config.database_url = database_url;

    // Setup temporary storage
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    config.hot_storage_root = temp_dir.path().join("hot");
    config.cold_storage_root = temp_dir.path().join("cold");
    std::fs::create_dir_all(&config.hot_storage_root).expect("Failed to create hot storage");
    std::fs::create_dir_all(&config.cold_storage_root).expect("Failed to create cold storage");

    // Build application
    let builder = ApplicationBuilder::new(config)
        .with_database()
        .await
        .unwrap();

    let (state, api_key_repo, audit_repo) = builder
        .with_infrastructure()
        .await
        .unwrap()
        .with_api_keys()
        .await
        .unwrap()
        .build()
        .unwrap();

    let app = create_router(state, api_key_repo, audit_repo);

    (app, container, temp_dir)
}
