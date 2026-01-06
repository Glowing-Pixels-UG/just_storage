use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::{info, Level};

use just_storage::{api::create_router, ApplicationBuilder, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with structured logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(true)
        .with_thread_ids(true)
        .init();

    info!("Starting JustStorage service");

    // Load and validate configuration
    let config = Config::from_env();
    config.validate()?;
    info!("Configuration loaded and validated");

    // Build application using builder pattern
    let listen_addr = config.listen_addr.clone();

    let builder = ApplicationBuilder::new(config).with_database().await?;

    let gc = builder.build_gc()?;
    tokio::spawn(Arc::clone(&gc).run());
    info!("Garbage collector started");

    let (state, api_key_repo, audit_repo) = builder
        .with_infrastructure()
        .await?
        .with_api_keys()
        .await?
        .build()?;

    // Create router
    let app = create_router(state, api_key_repo, audit_repo);

    // Start server with graceful shutdown
    info!("Listening on {}", listen_addr);
    let listener = TcpListener::bind(&listen_addr).await?;

    // Setup graceful shutdown
    let shutdown_signal = async {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        info!("Shutdown signal received, starting graceful shutdown...");
    };

    // Start server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    info!("Server shutdown complete");
    Ok(())
}
