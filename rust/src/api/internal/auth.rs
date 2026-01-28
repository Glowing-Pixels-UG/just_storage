use crate::api::router::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use tower_cookies::Cookies;

/// Middleware for internal admin authentication
pub async fn internal_admin_auth(
    State(state): State<AppState>,
    cookies: Cookies,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();

    // Exempt login page and static assets from auth
    // Note: When nested under /dashboard, the path here might be relative or absolute 
    // depending on where the middleware is applied. 
    if path == "/login" || path == "/dashboard/login" || 
       path.starts_with("/static") || path.starts_with("/dashboard/static") {
        return Ok(next.run(req).await);
    }

    let expected_token = match &state.config.admin_token {
        Some(token) => token,
        None => return Err(StatusCode::FORBIDDEN),
    };

    // 1. Check Authorization header
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let bearer_token = format!("Bearer {}", expected_token);
    if auth_header == Some(&bearer_token) {
        return Ok(next.run(req).await);
    }

    // 2. Check admin_session cookie
    let cookie_token = cookies.get("admin_session").map(|c| c.value().to_string());
    if cookie_token == Some(expected_token.clone()) {
        return Ok(next.run(req).await);
    }

    // 3. Fallback: Redirect to login if it's a browser request (GET to HTML page)
    // For simplicity, we redirect all GET requests that aren't HTMX/API
    let accept_header = req
        .headers()
        .get("accept")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if req.method() == axum::http::Method::GET && accept_header.contains("text/html") {
        Ok(Redirect::to("/dashboard/login").into_response())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
