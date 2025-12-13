use axum::{
    middleware as axum_middleware,
    routing::{delete, get, post},
    Router,
};
use sqlx::PgPool;
use std::sync::Arc;

use crate::api::handlers::{
    delete_handler, download_by_key_handler, download_handler, health_handler, list_handler,
    readiness_handler, search_handler, text_search_handler, upload_handler,
};
use crate::api::middleware::{auth, metrics};
use crate::application::use_cases::{
    DeleteObjectUseCase, DownloadObjectUseCase, ListObjectsUseCase, SearchObjectsUseCase,
    TextSearchObjectsUseCase, UploadObjectUseCase,
};

/// Application state container
pub struct AppState {
    pub pool: Arc<PgPool>,
    pub upload_use_case: Arc<UploadObjectUseCase>,
    pub download_use_case: Arc<DownloadObjectUseCase>,
    pub delete_use_case: Arc<DeleteObjectUseCase>,
    pub list_use_case: Arc<ListObjectsUseCase>,
    pub search_use_case: Arc<SearchObjectsUseCase>,
    pub text_search_use_case: Arc<TextSearchObjectsUseCase>,
}

/// Create router with all routes and middleware
pub fn create_router(state: AppState) -> Router {
    let upload_state = Arc::clone(&state.upload_use_case);
    let download_state = Arc::clone(&state.download_use_case);
    let delete_state = Arc::clone(&state.delete_use_case);
    let list_state = Arc::clone(&state.list_use_case);
    let search_state = Arc::clone(&state.search_use_case);
    let text_search_state = Arc::clone(&state.text_search_use_case);

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
            "/v1/objects/{id}",
            get(download_handler).with_state(Arc::clone(&download_state)),
        )
        .route(
            "/v1/objects/by-key/{namespace}/{tenant_id}/{key}",
            get(download_by_key_handler).with_state(download_state),
        )
        .route(
            "/v1/objects/{id}",
            delete(delete_handler).with_state(delete_state),
        )
        .route(
            "/v1/objects/search",
            post(search_handler).with_state(search_state),
        )
        .route(
            "/v1/objects/search/text",
            post(text_search_handler).with_state(text_search_state),
        )
        // Apply middleware layers (auth + metrics)
        .layer(axum_middleware::from_fn(auth::auth_middleware))
        .layer(axum_middleware::from_fn(metrics::metrics_middleware))
}
