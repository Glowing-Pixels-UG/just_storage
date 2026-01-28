use axum::{
    http::header::{HeaderValue, CACHE_CONTROL, CONTENT_SECURITY_POLICY, X_FRAME_OPTIONS},
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;
use crate::api::internal::auth::internal_admin_auth;
use crate::api::internal::handlers::actions::{clear_cache, reindex};
use crate::api::internal::handlers::health::health_page;
use crate::api::internal::handlers::login::{login_handler, login_page};
use crate::api::router::AppState;

pub mod auth;
pub mod handlers;
pub mod templates;

/// Create the internal ops router
pub async fn create_internal_router(state: AppState) -> Router {
    let security_layer = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(CACHE_CONTROL, "no-store, max-age=0".parse::<HeaderValue>().unwrap()))
        .layer(SetResponseHeaderLayer::overriding(CONTENT_SECURITY_POLICY, "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';".parse::<HeaderValue>().unwrap()))
        .layer(SetResponseHeaderLayer::overriding(X_FRAME_OPTIONS, "DENY".parse::<HeaderValue>().unwrap()));

    // Set up session storage
    let session_store = PostgresStore::new(state.pool.as_ref().clone());
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true) // Should be true in prod
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(24)));

    Router::new()
        .route("/health", get(health_page))
        .route("/actions/cache/clear", post(clear_cache))
        .route("/actions/reindex", post(reindex))
        .route("/login", get(login_page).post(login_handler))
        .nest_service("/static", ServeDir::new("internal_static"))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            internal_admin_auth,
        ))
        .layer(session_layer)
        .layer(CookieManagerLayer::new())
        .layer(security_layer)
        .with_state(state)
}
