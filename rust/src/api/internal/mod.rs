use axum::{
    routing::{get, post},
    Router,
    middleware as axum_middleware,
    http::header::{HeaderValue, CACHE_CONTROL, CONTENT_SECURITY_POLICY, X_FRAME_OPTIONS},
};
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};
use tower::ServiceBuilder;

use crate::api::router::AppState;
use crate::api::internal::auth::internal_admin_auth;
use crate::api::internal::handlers::health::health_page;
use crate::api::internal::handlers::actions::{clear_cache, reindex};

pub mod auth;
pub mod handlers;
pub mod templates;

/// Create the internal ops router
pub fn create_internal_router(state: AppState) -> Router {
    let security_layer = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(CACHE_CONTROL, "no-store, max-age=0".parse::<HeaderValue>().unwrap()))
        .layer(SetResponseHeaderLayer::overriding(CONTENT_SECURITY_POLICY, "default-src 'self'; script-src 'self' https://unpkg.com; style-src 'self' 'unsafe-inline';".parse::<HeaderValue>().unwrap()))
        .layer(SetResponseHeaderLayer::overriding(X_FRAME_OPTIONS, "DENY".parse::<HeaderValue>().unwrap()));

    Router::new()
        .route("/health", get(health_page))
        .route("/actions/cache/clear", post(clear_cache))
        .route("/actions/reindex", post(reindex))
        .nest_service("/static", ServeDir::new("internal_static"))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            internal_admin_auth,
        ))
        .layer(security_layer)
        .with_state(state)
}
