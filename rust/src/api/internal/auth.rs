use crate::api::router::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::Next,
    response::Response,
};

/// Middleware for internal admin authentication
pub async fn internal_admin_auth(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let expected_token = match &state.config.admin_token {
        Some(token) => token,
        None => return Err(StatusCode::FORBIDDEN),
    };

    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let bearer_token = format!("Bearer {}", expected_token);

    if auth_header == Some(&bearer_token) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
