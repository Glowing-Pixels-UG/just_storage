use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Middleware to handle HTMX redirects
///
/// HTMX intercepts 302 redirects and doesn't allow the browser to handle them.
/// This middleware detects HTMX requests and converts 302 redirects to HX-Redirect headers.
pub async fn htmx_redirect_middleware(
    req: Request<Body>,
    next: Next,
) -> Response {
    let is_htmx = req.headers().contains_key("hx-request");
    
    let response = next.run(req).await;
    
    if is_htmx && response.status().is_redirection() {
        if let Some(location) = response.headers().get("location") {
            let mut htmx_response = StatusCode::OK.into_response();
            htmx_response.headers_mut().insert("HX-Redirect", location.clone());
            return htmx_response;
        }
    }
    
    response
}
