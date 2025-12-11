use axum::{
    middleware as axum_middleware,
    routing::{delete, get, post},
    Router,
};
use sqlx::PgPool;
use std::sync::Arc;

use crate::api::handlers::{
    delete_handler, download_handler, health_handler, list_handler, readiness_handler,
    upload_handler,
};
use crate::api::middleware::{auth, metrics};
use crate::application::use_cases::{
    DeleteObjectUseCase, DownloadObjectUseCase, ListObjectsUseCase, UploadObjectUseCase,
};

/// Application state container
pub struct AppState {
    pub pool: Arc<PgPool>,
    pub upload_use_case: Arc<UploadObjectUseCase>,
    pub download_use_case: Arc<DownloadObjectUseCase>,
    pub delete_use_case: Arc<DeleteObjectUseCase>,
    pub list_use_case: Arc<ListObjectsUseCase>,
}

/// Create router with all routes and middleware
pub fn create_router(state: AppState) -> Router {
    let upload_state = Arc::clone(&state.upload_use_case);
    let download_state = Arc::clone(&state.download_use_case);
    let delete_state = Arc::clone(&state.delete_use_case);
    let list_state = Arc::clone(&state.list_use_case);

    Router::new()
        // Health check (no auth required)
        .route("/health", get(health_handler))
        .route(
            "/health/ready",
            get(readiness_handler).with_state(Arc::clone(&state.pool)),
        )
        // Protected API routes
        .route("/v1/objects", post(upload_handler).with_state(upload_state))
        .route("/v1/objects", get(list_handler).with_state(list_state))
        .route(
            "/v1/objects/:id",
            get(download_handler).with_state(download_state),
        )
        .route(
            "/v1/objects/:id",
            delete(delete_handler).with_state(delete_state),
        )
        // Apply middleware layers (auth + metrics)
        .layer(axum_middleware::from_fn(auth::auth_middleware))
        .layer(axum_middleware::from_fn(metrics::metrics_middleware))
}
