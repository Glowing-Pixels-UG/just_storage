use std::sync::Arc;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tracing::info;

use crate::api::router::AppState;
use crate::application::{
    gc::GarbageCollector,
    ports::{ApiKeyRepository, AuditRepository, BlobRepository, BlobStore, ObjectRepository},
    use_cases::{
        CreateApiKeyUseCase, DeleteApiKeyUseCase, DeleteObjectUseCase, DownloadObjectUseCase,
        GetApiKeyUseCase, ListApiKeysUseCase, ListObjectsUseCase, SearchObjectsUseCase,
        TextSearchObjectsUseCase, UpdateApiKeyUseCase, UploadObjectUseCase,
    },
};
use crate::config::Config;

/// Type alias for the complex build result tuple
type BuildResult = Result<
    (
        AppState,
        Arc<dyn ApiKeyRepository>,
        Arc<dyn AuditRepository>,
    ),
    Box<dyn std::error::Error>,
>;

use crate::infrastructure::{
    persistence::{
        PostgresApiKeyRepository, PostgresAuditRepository, PostgresBlobRepository,
        PostgresObjectRepository,
    },
    storage::LocalFilesystemStore,
};

/// Application builder for clean dependency injection and setup
pub struct ApplicationBuilder {
    config: Config,
    pool: Option<sqlx::PgPool>,
    object_repo: Option<Arc<dyn ObjectRepository>>,
    blob_repo: Option<Arc<dyn BlobRepository>>,
    blob_store: Option<Arc<dyn BlobStore>>,
    api_key_repo: Option<Arc<dyn ApiKeyRepository>>,
    audit_repo: Option<Arc<dyn AuditRepository>>,
}

impl ApplicationBuilder {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            pool: None,
            object_repo: None,
            blob_repo: None,
            blob_store: None,
            api_key_repo: None,
            audit_repo: None,
        }
    }

    /// Initialize database connection pool with retry logic
    pub async fn with_database(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Connecting to database: {}", self.config.database_url);

        // Retry connection with exponential backoff
        let mut retries = 3;
        let mut delay = Duration::from_secs(1);
        let pool = loop {
            match PgPoolOptions::new()
                .max_connections(self.config.db_max_connections)
                .min_connections(self.config.db_min_connections)
                .acquire_timeout(Duration::from_secs(self.config.db_acquire_timeout_secs))
                .idle_timeout(Some(Duration::from_secs(self.config.db_idle_timeout_secs)))
                .max_lifetime(Some(Duration::from_secs(self.config.db_max_lifetime_secs)))
                .connect(&self.config.database_url)
                .await
            {
                Ok(pool) => break pool,
                Err(e) if retries > 0 => {
                    retries -= 1;
                    tracing::warn!(
                        "Database connection failed, retrying in {:?} ({} retries left): {}",
                        delay,
                        retries,
                        e
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
                Err(e) => {
                    tracing::error!("Failed to connect to database after retries: {}", e);
                    return Err(Box::new(e));
                }
            }
        };

        info!(
            "Database pool configured: max={}, min={}, acquire_timeout={}s, idle_timeout={}s, max_lifetime={}s",
            self.config.db_max_connections,
            self.config.db_min_connections,
            self.config.db_acquire_timeout_secs,
            self.config.db_idle_timeout_secs,
            self.config.db_max_lifetime_secs
        );

        // Run database migrations
        info!("Running database migrations");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to run migrations: {}", e);
                e
            })?;

        self.pool = Some(pool);
        Ok(self)
    }

    /// Initialize infrastructure layer (repositories and storage)
    pub async fn with_infrastructure(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = self.pool.as_ref().ok_or("Database pool not initialized")?;

        let object_repo: Arc<dyn ObjectRepository> =
            Arc::new(PostgresObjectRepository::new(pool.clone()));
        let blob_repo: Arc<dyn BlobRepository> =
            Arc::new(PostgresBlobRepository::new(pool.clone()));
        let audit_repo: Arc<dyn AuditRepository> =
            Arc::new(PostgresAuditRepository::new(pool.clone()));

        let blob_store = Arc::new(LocalFilesystemStore::with_full_config(
            self.config.hot_storage_root.clone(),
            self.config.cold_storage_root.clone(),
            true, // durable_writes
            true, // precreate_dirs
            self.config.concurrent_cache_threshold,
            self.config.adaptive_buffering_enabled,
        ));
        blob_store.init().await?;
        let blob_store: Arc<dyn BlobStore> = blob_store;

        self.object_repo = Some(object_repo);
        self.blob_repo = Some(blob_repo);
        self.blob_store = Some(blob_store);
        self.audit_repo = Some(audit_repo);

        info!("Infrastructure layer initialized");
        Ok(self)
    }

    /// Set up API key repository
    pub async fn with_api_keys(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = self.pool.as_ref().ok_or("Database pool not initialized")?;
        let api_key_repo = Arc::new(PostgresApiKeyRepository::new(pool.clone()));
        self.api_key_repo = Some(api_key_repo);
        info!("API key repository initialized");
        Ok(self)
    }

    /// Build application state with all use cases
    pub fn build(self) -> BuildResult {
        let pool = self.pool.ok_or("Database pool not initialized")?;
        let object_repo = self
            .object_repo
            .ok_or("Object repository not initialized")?;
        let blob_repo = self.blob_repo.ok_or("Blob repository not initialized")?;
        let blob_store = self.blob_store.ok_or("Blob store not initialized")?;
        let api_key_repo = self
            .api_key_repo
            .ok_or("API key repository not initialized")?;
        let audit_repo = self.audit_repo.ok_or("Audit repository not initialized")?;

        // Initialize use cases (application layer)
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

        let list_use_case = Arc::new(ListObjectsUseCase::new(Arc::clone(&object_repo)));

        let search_use_case = Arc::new(SearchObjectsUseCase::new(Arc::clone(&object_repo)));

        let text_search_use_case =
            Arc::new(TextSearchObjectsUseCase::new(Arc::clone(&object_repo)));

        // API key use cases
        let create_api_key_use_case = Arc::new(CreateApiKeyUseCase::new(Arc::clone(&api_key_repo)));
        let list_api_keys_use_case = Arc::new(ListApiKeysUseCase::new(Arc::clone(&api_key_repo)));
        let get_api_key_use_case = Arc::new(GetApiKeyUseCase::new(Arc::clone(&api_key_repo)));
        let update_api_key_use_case = Arc::new(UpdateApiKeyUseCase::new(Arc::clone(&api_key_repo)));
        let delete_api_key_use_case = Arc::new(DeleteApiKeyUseCase::new(Arc::clone(&api_key_repo)));

        info!("Application layer initialized");

        let pool_arc = Arc::new(pool);
        let app_state = AppState {
            pool: pool_arc.clone(),
            upload_use_case,
            download_use_case,
            delete_use_case,
            list_use_case,
            search_use_case,
            text_search_use_case,
            create_api_key_use_case,
            list_api_keys_use_case,
            get_api_key_use_case,
            update_api_key_use_case,
            delete_api_key_use_case,
            config: self.config.clone(),
        };

        Ok((app_state, api_key_repo, audit_repo))
    }

    /// Get garbage collector instance
    pub fn build_gc(&self) -> Result<Arc<GarbageCollector>, Box<dyn std::error::Error>> {
        let blob_repo = self
            .blob_repo
            .as_ref()
            .ok_or("Blob repository not initialized")?;
        let blob_store = self
            .blob_store
            .as_ref()
            .ok_or("Blob store not initialized")?;
        let object_repo = self.object_repo.as_ref();

        Ok(Arc::new(GarbageCollector::with_object_repo(
            Arc::clone(blob_repo),
            Arc::clone(blob_store),
            object_repo.map(Arc::clone),
            Duration::from_secs(self.config.gc_interval_secs),
            self.config.gc_batch_size,
            1, // Clean up WRITING objects older than 1 hour
        )))
    }

    /// Get configuration
    pub fn config(&self) -> &Config {
        &self.config
    }
}
