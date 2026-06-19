#![allow(dead_code, unused_imports)]

//! Test environment with builder pattern (Phase 1 skeleton)

use sqlx::PgPool;
use std::sync::Arc;
use tempfile::TempDir;
use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};

use just_storage::application::ports::{
    ApiKeyRepository, AuditRepository, BlobRepository, BlobStore, ObjectRepository,
};
use just_storage::infrastructure::{
    persistence::{PostgresBlobRepository, PostgresObjectRepository},
    storage::LocalFilesystemStore,
};

use crate::common::database::{cleanup_test_data, setup_test_database, setup_test_storage};

// Use-case types for optional wiring by the TestEnvironmentBuilder
use just_storage::application::use_cases::{
    DeleteObjectUseCase, DownloadObjectUseCase, UploadObjectUseCase,
};

/// A single source of truth TestEnvironment used by tests
pub struct TestEnvironment {
    pub pool: PgPool,
    pub database_url: String,
    pub object_repo: Arc<dyn ObjectRepository>,
    pub blob_repo: Arc<dyn BlobRepository>,
    pub blob_store: Arc<dyn BlobStore>,
    pub hot_dir: TempDir,
    pub cold_dir: TempDir,
    pub api_key_repo: Option<Arc<dyn ApiKeyRepository>>,
    pub audit_repo: Option<Arc<dyn AuditRepository>>,
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
        let (pool, container) = setup_test_database().await;
        let (hot_dir, cold_dir) = setup_test_storage();

        let host = container.get_host().await.unwrap();
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let database_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

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
            database_url,
            object_repo,
            blob_repo,
            blob_store,
            hot_dir,
            cold_dir,
            api_key_repo: None,
            audit_repo: None,
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
            let (router, _internal_router, container, temp_dir) = setup_test_api_server().await;
            env.api_router = Some(router);
            env.api_container = Some(container);
            env.api_temp_dir = Some(temp_dir);
        }

        env
    }
}

/// Helper to create an API server (Router) wired with application state for tests
/// Returns (Router, internal_router, container, temp_dir) where `temp_dir` must be kept alive by caller
pub async fn setup_test_api_server() -> (
    axum::Router,
    axum::Router,
    testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>,
    tempfile::TempDir,
) {
    use just_storage::{
        api::{
            create_router, create_router_with_middleware, internal::create_internal_router,
            middleware::config::MiddlewareConfig,
        },
        ApplicationBuilder, Config,
    };

    // Start PostgreSQL container (migrations will be run by ApplicationBuilder)
    let container = setup_test_database().await.1;

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
    config.admin_token = Some("test-key".to_string()); // Ensure tests can use test-key

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
        .unwrap()
        .with_infrastructure()
        .await
        .unwrap()
        .with_api_keys()
        .await
        .unwrap()
        .with_oidc()
        .await
        .unwrap();

    let (state, api_key_repo, audit_repo) = builder.build().unwrap();

    let app = create_router(state.clone(), api_key_repo, audit_repo).await;
    let internal_app = create_internal_router(state).await;

    (app, internal_app, container, temp_dir)
}

/// Helper to create an API server with OIDC configuration
pub async fn setup_test_api_server_with_oidc(
    oidc_issuer_url: String,
) -> (
    axum::Router,
    axum::Router,
    testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>,
    tempfile::TempDir,
) {
    use just_storage::{
        api::{
            create_router, create_router_with_middleware, internal::create_internal_router,
            middleware::config::MiddlewareConfig,
        },
        ApplicationBuilder, Config,
    };

    // Start PostgreSQL container
    let container = setup_test_database().await.1;

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
    config.admin_token = Some("test-key".to_string());
    config.oidc_issuer_url = Some(oidc_issuer_url);
    config.oidc_client_id = Some("test-client".to_string());
    config.oidc_client_secret = Some("test-secret".to_string());
    config.oidc_audience = Some("test-client".to_string());
    config.oidc_redirect_url = Some("http://localhost/dashboard/auth/callback".to_string());

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
        .unwrap()
        .with_infrastructure()
        .await
        .unwrap()
        .with_api_keys()
        .await
        .unwrap()
        .with_oidc()
        .await
        .unwrap();

    let (state, api_key_repo, audit_repo) = builder.build().unwrap();

    let app = create_router(state.clone(), api_key_repo, audit_repo).await;
    let internal_app = create_internal_router(state).await;

    (app, internal_app, container, temp_dir)
}
