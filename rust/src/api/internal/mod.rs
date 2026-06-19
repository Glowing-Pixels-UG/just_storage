use axum::{
    http::header::{HeaderValue, CACHE_CONTROL, CONTENT_SECURITY_POLICY, X_FRAME_OPTIONS},
    middleware as axum_middleware,
    routing::{get, post},
    Router,
};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::{services::ServeDir, set_header::SetResponseHeaderLayer};

use crate::api::internal::auth::internal_admin_auth;
use crate::api::internal::handlers::actions::{clear_cache, reindex};
use crate::api::internal::handlers::auth::{oidc_callback, oidc_login, oidc_logout};
use crate::api::internal::handlers::health::health_page;
use crate::api::internal::handlers::login::{login_handler, login_page};
use crate::api::middleware::csrf::csrf_middleware;
use crate::api::middleware::htmx::htmx_redirect_middleware;
use crate::api::router::AppState;
use crate::infrastructure::persistence::EncryptedPostgresStore;
use tower_sessions::{Expiry, SessionManagerLayer};

pub mod auth;
pub mod handlers;
pub mod templates;

/// Create the internal ops router
pub async fn create_internal_router(state: AppState) -> Router {
    let security_layer = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            "no-store, max-age=0".parse::<HeaderValue>().unwrap(),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            CONTENT_SECURITY_POLICY,
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';"
                .parse::<HeaderValue>()
                .unwrap(),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            X_FRAME_OPTIONS,
            "DENY".parse::<HeaderValue>().unwrap(),
        ));

    // Set up session storage
    let encryption_key = match &state.config.session_encryption_key {
        Some(k) => {
            let mut key = [0u8; 32];
            let decoded = hex::decode(k).unwrap_or_else(|_| k.as_bytes().to_vec());
            let len = decoded.len().min(32);
            key[..len].copy_from_slice(&decoded[..len]);
            key
        }
        None => {
            // Derive from session_secret if available, otherwise use a default (not for prod!)
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(
                state
                    .config
                    .session_secret
                    .as_deref()
                    .unwrap_or("default-secret-key"),
            );
            hasher.finalize().into()
        }
    };

    let session_store = EncryptedPostgresStore::new(state.pool.as_ref().clone(), encryption_key);

    // Spawn background task to clean up expired sessions
    let session_store_clone = session_store.clone();
    tokio::spawn(async move {
        loop {
            match session_store_clone.delete_expired().await {
                Ok(_) => tracing::debug!("Expired sessions cleaned up"),
                Err(e) => tracing::error!("Failed to clean up expired sessions: {}", e),
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        }
    });

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true) // Should be true in prod
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(24)));

    let auth_rate_limit_config = crate::api::middleware::rate_limiting::RateLimitConfig {
        unauthenticated_requests_per_minute: 10, // Aggressive for login/callback
        ..Default::default()
    };
    let auth_rate_limit_layer =
        crate::api::middleware::rate_limiting::RateLimitLayer::new(auth_rate_limit_config);

    Router::new()
        .route("/health", get(health_page))
        .route("/actions/cache/clear", post(clear_cache))
        .route("/actions/reindex", post(reindex))
        .route("/login", get(login_page).post(login_handler))
        .route("/auth/login", get(oidc_login))
        .route("/auth/callback", get(oidc_callback))
        .route("/auth/logout", get(oidc_logout))
        .nest_service("/static", ServeDir::new("internal_static"))
        .layer(auth_rate_limit_layer)
        .layer(axum_middleware::from_fn(csrf_middleware))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            internal_admin_auth,
        ))
        .layer(axum_middleware::from_fn(htmx_redirect_middleware))
        .layer(session_layer)
        .layer(CookieManagerLayer::new())
        .layer(security_layer)
        .with_state(state)
}
