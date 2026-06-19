use std::sync::Arc;
use std::time::{Duration, Instant};

use openidconnect::core::CoreProviderMetadata;
use openidconnect::{IssuerUrl, JsonWebKey};
use reqwest::Client as ReqwestClient;
use sqlx::postgres::PgPoolOptions;
use tracing::{error, info, warn};

use crate::api::router::AppState;
use crate::application::gc::GarbageCollector;
use crate::application::ports::{
    ApiKeyRepository, AuditRepository, BlobRepository, BlobStore, ObjectRepository,
};
use crate::application::use_cases::{
    CreateApiKeyUseCase, DeleteApiKeyUseCase, DeleteObjectUseCase, DownloadObjectUseCase,
    GetApiKeyUseCase, ListApiKeysUseCase, ListObjectsUseCase, SearchObjectsUseCase,
    TextSearchObjectsUseCase, UpdateApiKeyUseCase, UploadObjectUseCase,
};
use crate::config::Config;
use crate::infrastructure::persistence::{
    PostgresApiKeyRepository, PostgresAuditRepository, PostgresBlobRepository,
    PostgresObjectRepository,
};
use crate::infrastructure::storage::LocalFilesystemStore;

/// Result type for the application builder
pub type BuildResult = Result<
    (
        AppState,
        Arc<dyn ApiKeyRepository>,
        Arc<dyn AuditRepository>,
    ),
    Box<dyn std::error::Error>,
>;

/// Builder for the application container
pub struct ApplicationBuilder {
    config: Config,
    pool: Option<Arc<sqlx::PgPool>>,
    object_repo: Option<Arc<dyn ObjectRepository>>,
    blob_repo: Option<Arc<dyn BlobRepository>>,
    blob_store: Option<Arc<dyn BlobStore>>,
    api_key_repo: Option<Arc<dyn ApiKeyRepository>>,
    audit_repo: Option<Arc<dyn AuditRepository>>,
    gc: Option<Arc<GarbageCollector>>,
    oidc_metadata: Option<CoreProviderMetadata>,
    jwks_cache: Arc<moka::future::Cache<String, jsonwebtoken::DecodingKey>>,
    expected_migration_count: usize,
}

impl ApplicationBuilder {
    /// Create a new builder with the given configuration
    pub fn new(config: Config) -> Self {
        Self {
            config,
            pool: None,
            object_repo: None,
            blob_repo: None,
            blob_store: None,
            api_key_repo: None,
            audit_repo: None,
            gc: None,
            oidc_metadata: None,
            jwks_cache: Arc::new(moka::future::Cache::new(100)),
            expected_migration_count: 0,
        }
    }

    /// Set up database connection pool
    pub async fn with_database(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        info!("Connecting to database");

        use sqlx::postgres::PgConnectOptions;
        use std::str::FromStr;

        // Parse connection options and disable statement cache for PgBouncer compatibility
        let mut connect_options = PgConnectOptions::from_str(&self.config.database_url)
            .map_err(|e| format!("Invalid database URL: {}", e))?;

        // Disable statement cache for PgBouncer compatibility
        connect_options = connect_options.statement_cache_capacity(0);

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
                .connect_with(connect_options.clone())
                .await
            {
                Ok(pool) => break pool,
                Err(e) if retries > 0 => {
                    warn!(
                        "Database connection failed: {}. Retrying in {:?}...",
                        e, delay
                    );
                    tokio::time::sleep(delay).await;
                    retries -= 1;
                    delay *= 2;
                }
                Err(e) => return Err(e.into()),
            }
        };

        // Run database migrations
        info!("Running database migrations");
        let migrator = sqlx::migrate!("./migrations");
        self.expected_migration_count = migrator.migrations.len();
        migrator.run(&pool).await.map_err(|e| {
            error!("Migration failed: {}", e);
            e
        })?;

        self.pool = Some(Arc::new(pool));
        Ok(self)
    }

    /// Set up infrastructure components (repositories and stores)
    pub async fn with_infrastructure(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = self.pool.as_ref().ok_or("Database pool not initialized")?;

        let object_repo = Arc::new(PostgresObjectRepository::new(
            Arc::clone(pool).as_ref().clone(),
        ));
        let blob_repo = Arc::new(PostgresBlobRepository::new(
            Arc::clone(pool).as_ref().clone(),
        ));
        let audit_repo = Arc::new(PostgresAuditRepository::new(
            Arc::clone(pool).as_ref().clone(),
        ));

        let blob_store = Arc::new(LocalFilesystemStore::new(
            self.config.hot_storage_root.clone(),
            self.config.cold_storage_root.clone(),
        ));

        // Initialize storage directories
        blob_store
            .init()
            .await
            .map_err(|e| format!("Failed to initialize blob store: {}", e))?;

        self.object_repo = Some(object_repo);
        self.blob_repo = Some(blob_repo);
        self.audit_repo = Some(audit_repo);
        self.blob_store = Some(blob_store);

        Ok(self)
    }

    /// Set up API key repository
    pub async fn with_api_keys(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = self.pool.as_ref().ok_or("Database pool not initialized")?;
        let api_key_repo = Arc::new(PostgresApiKeyRepository::new(
            Arc::clone(pool).as_ref().clone(),
        ));
        self.api_key_repo = Some(api_key_repo);
        Ok(self)
    }

    /// Set up garbage collector
    pub fn with_gc(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        let gc = self.build_gc()?;
        self.gc = Some(gc);
        info!("Garbage collector initialized");
        Ok(self)
    }

    /// Set up OIDC metadata
    pub async fn with_oidc(mut self) -> Result<Self, Box<dyn std::error::Error>> {
        if let Some(issuer_url_str) = &self.config.oidc_issuer_url {
            info!("Initializing OIDC for issuer: {}", issuer_url_str);

            let issuer_url = IssuerUrl::new(issuer_url_str.clone())?;

            // Configure HTTP client with SSRF protection (no redirects)
            let http_client = ReqwestClient::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(10))
                .build()?;

            let provider_metadata =
                CoreProviderMetadata::discover_async(issuer_url.clone(), &http_client).await?;
            self.oidc_metadata = Some(provider_metadata.clone());

            // Initial JWKS fetch and cache population
            Self::refresh_jwks(&http_client, &provider_metadata, &self.jwks_cache).await?;

            // Spawn background refresh task (every hour)
            let jwks_cache = Arc::clone(&self.jwks_cache);
            let http_client_clone = http_client.clone();
            let provider_metadata_clone = provider_metadata.clone();

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(3600));
                loop {
                    interval.tick().await;
                    match Self::refresh_jwks(
                        &http_client_clone,
                        &provider_metadata_clone,
                        &jwks_cache,
                    )
                    .await
                    {
                        Ok(_) => info!("JWKS cache refreshed successfully"),
                        Err(e) => error!("Failed to refresh JWKS cache: {}", e),
                    }
                }
            });
        } else {
            warn!("OIDC issuer URL not configured, SSO will be disabled");
        }

        Ok(self)
    }

    /// Internal helper to refresh JWKS cache
    async fn refresh_jwks(
        http_client: &ReqwestClient,
        provider_metadata: &CoreProviderMetadata,
        jwks_cache: &Arc<moka::future::Cache<String, jsonwebtoken::DecodingKey>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let jwks_url = provider_metadata.jwks_uri().url();
        let jwks_response = http_client.get(jwks_url.as_str()).send().await?;
        let jwks: openidconnect::core::CoreJsonWebKeySet = jwks_response.json().await?;

        for jwk in jwks.keys() {
            if let Some(kid) = jwk.key_id() {
                let jwk_json: serde_json::Value = serde_json::to_value(jwk)?;
                if let (Some(n), Some(e)) = (jwk_json["n"].as_str(), jwk_json["e"].as_str()) {
                    if let Ok(decoding_key) = jsonwebtoken::DecodingKey::from_rsa_components(n, e) {
                        jwks_cache.insert(kid.to_string(), decoding_key).await;
                    }
                }
            }
        }
        Ok(())
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
        let upload_use_case = Arc::new(UploadObjectUseCase::with_max_upload_size_bytes(
            Arc::clone(&object_repo),
            Arc::clone(&blob_repo),
            Arc::clone(&blob_store),
            self.config.max_upload_size_bytes,
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

        let create_api_key_use_case = Arc::new(CreateApiKeyUseCase::new(Arc::clone(&api_key_repo)));
        let list_api_keys_use_case = Arc::new(ListApiKeysUseCase::new(Arc::clone(&api_key_repo)));
        let get_api_key_use_case = Arc::new(GetApiKeyUseCase::new(Arc::clone(&api_key_repo)));
        let update_api_key_use_case = Arc::new(UpdateApiKeyUseCase::new(Arc::clone(&api_key_repo)));
        let delete_api_key_use_case = Arc::new(DeleteApiKeyUseCase::new(Arc::clone(&api_key_repo)));

        let app_state = AppState {
            pool: Arc::clone(&pool),
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
            audit_repo: Arc::clone(&audit_repo),
            blob_store: Arc::clone(&blob_store),
            gc: self.gc,
            config: self.config.clone(),
            oidc_metadata: self.oidc_metadata,
            jwks_cache: self.jwks_cache,
            expected_migration_count: self.expected_migration_count,
            start_time: Instant::now(),
        };

        Ok((app_state, api_key_repo, audit_repo))
    }

    /// Internal helper to build garbage collector
    fn build_gc(&self) -> Result<Arc<GarbageCollector>, Box<dyn std::error::Error>> {
        let blob_repo = self
            .blob_repo
            .as_ref()
            .ok_or("Blob repository not initialized")?;
        let blob_store = self
            .blob_store
            .as_ref()
            .ok_or("Blob store not initialized")?;
        let object_repo = self.object_repo.clone();

        let gc = GarbageCollector::with_object_repo(
            Arc::clone(blob_repo),
            Arc::clone(blob_store),
            object_repo,
            Duration::from_secs(self.config.gc_interval_secs),
            self.config.gc_batch_size,
            24, // 24 hours
        );

        Ok(Arc::new(gc))
    }
}
