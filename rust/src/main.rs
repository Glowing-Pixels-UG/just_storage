use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::{error, info, Level};

use just_storage::api::internal::create_internal_router;
use just_storage::{api::create_router, ApplicationBuilder, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    let admin_port = config.admin_port;

    let builder = ApplicationBuilder::new(config)
        .with_database()
        .await?
        .with_infrastructure()
        .await?
        .with_api_keys()
        .await?
        .with_gc()?
        .with_oidc()
        .await?;

    let (state, api_key_repo, audit_repo) = builder.build()?;

    if let Some(gc) = &state.gc {
        tokio::spawn(Arc::clone(gc).run());
        info!("Garbage collector started");
    }

    // Create main router
    let app = create_router(state.clone(), api_key_repo, audit_repo).await;

    // Setup graceful shutdown signal
    let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);
    let mut shutdown_rx = shutdown_tx.subscribe();

    let shutdown_signal = async move {
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
        let _ = shutdown_tx.send(());
    };

    // Spawn signal handler
    tokio::spawn(shutdown_signal);

    // Prepare main server
    info!("Listening on {}", listen_addr);
    let listener = TcpListener::bind(&listen_addr).await?;
    let main_shutdown_rx = shutdown_rx;
    shutdown_rx = main_shutdown_rx.resubscribe();

    let main_server = async move {
        if let Err(e) = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = main_shutdown_rx.resubscribe().recv().await;
            })
            .await
        {
            error!("Main server error: {}", e);
        }
    };

    // Start servers
    if let Some(port) = admin_port {
        let admin_addr = format!("0.0.0.0:{}", port);
        info!("Internal admin listening on {}", admin_addr);
        let admin_listener = TcpListener::bind(&admin_addr).await?;
        let admin_router = create_internal_router(state).await;
        let admin_shutdown_rx = shutdown_rx;

        let admin_server = async move {
            if let Err(e) = axum::serve(admin_listener, admin_router)
                .with_graceful_shutdown(async move {
                    let _ = admin_shutdown_rx.resubscribe().recv().await;
                })
                .await
            {
                error!("Admin server error: {}", e);
            }
        };

        tokio::join!(main_server, admin_server);
    } else {
        main_server.await;
    }

    info!("Server shutdown complete");
    Ok(())
}
