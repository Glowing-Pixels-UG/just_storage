use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tokio::net::TcpListener;
use tracing::{info, Level};

use just_storage::{
    api::{create_router, router::AppState},
    application::{
        gc::GarbageCollector,
        ports::{BlobRepository, BlobStore, ObjectRepository},
        use_cases::{
            DeleteObjectUseCase, DownloadObjectUseCase, ListObjectsUseCase, UploadObjectUseCase,
        },
    },
    infrastructure::{
        persistence::{PostgresBlobRepository, PostgresObjectRepository},
        storage::LocalFilesystemStore,
    },
    Config,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with structured logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(true)
        .with_thread_ids(true)
        .init();

    info!("Starting ActiveStorage service");

    // Load configuration
    let config = Config::from_env();
    config.validate()?;
    info!("Configuration loaded and validated");

    // Initialize database connection pool
    info!("Connecting to database: {}", config.database_url);
    let pool = PgPool::connect(&config.database_url).await.map_err(|e| {
        tracing::error!("Failed to connect to database: {}", e);
        e
    })?;

    // Run database migrations
    info!("Running database migrations");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to run migrations: {}", e);
            e
        })?;

    // Initialize infrastructure layer
    let object_repo: Arc<dyn ObjectRepository> =
        Arc::new(PostgresObjectRepository::new(pool.clone()));
    let blob_repo: Arc<dyn BlobRepository> = Arc::new(PostgresBlobRepository::new(pool.clone()));

    let blob_store = Arc::new(LocalFilesystemStore::new(
        config.hot_storage_root.clone(),
        config.cold_storage_root.clone(),
    ));
    blob_store.init().await?;
    let blob_store: Arc<dyn BlobStore> = blob_store;

    info!("Infrastructure layer initialized");

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

    info!("Application layer initialized");

    // Start garbage collector in background
    let gc = Arc::new(GarbageCollector::new(
        Arc::clone(&blob_repo),
        Arc::clone(&blob_store),
        Duration::from_secs(config.gc_interval_secs),
        config.gc_batch_size,
    ));
    tokio::spawn(Arc::clone(&gc).run());
    info!("Garbage collector started");

    // Create app state
    let state = AppState {
        pool: Arc::new(pool.clone()),
        upload_use_case,
        download_use_case,
        delete_use_case,
        list_use_case,
    };

    // Create router
    let app = create_router(state);

    // Start server
    info!("Listening on {}", config.listen_addr);
    let listener = TcpListener::bind(&config.listen_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
