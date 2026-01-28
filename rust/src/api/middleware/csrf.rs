use axum::{
    extract::Request,
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rand::{distributions::Alphanumeric, Rng};
use tower_sessions::Session;
use serde::{Deserialize, Serialize};

pub const CSRF_SESSION_KEY: &str = "csrf_token";
pub const CSRF_HEADER_NAME: &str = "X-CSRF-Token";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CsrfToken(pub String);

/// CSRF protection middleware for Dashboard/BFF
pub async fn csrf_middleware(
    session: Session,
    request: Request,
    next: Next,
) -> Response {
    // 1. Get or generate CSRF token in session
    let csrf_token = match session.get::<CsrfToken>(CSRF_SESSION_KEY).await {
        Ok(Some(token)) => token,
        _ => {
            let token = CsrfToken(
                rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(32)
                    .map(char::from)
                    .collect(),
            );
            if let Err(e) = session.insert(CSRF_SESSION_KEY, token.clone()).await {
                tracing::error!("Failed to insert CSRF token into session: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response();
            }
            token
        }
    };

    // 2. Validate token for state-changing methods
    let method = request.method();
    if method == Method::POST || method == Method::PUT || method == Method::DELETE || method == Method::PATCH {
        let path = request.uri().path();
        
        // Skip validation for OIDC callback (it has its own state) and login
        if path != "/dashboard/auth/callback" && path != "/dashboard/login" {
            let provided_token = request.headers()
                .get(CSRF_HEADER_NAME)
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            if provided_token.as_deref() != Some(&csrf_token.0) {
                tracing::warn!("CSRF validation failed for path: {} (provided: {:?})", path, provided_token);
                return (StatusCode::FORBIDDEN, "CSRF token mismatch").into_response();
            }
        }
    }

    // 3. Add token to extensions so handlers/templates can use it
    let mut request = request;
    request.extensions_mut().insert(csrf_token);

    next.run(request).await
}
