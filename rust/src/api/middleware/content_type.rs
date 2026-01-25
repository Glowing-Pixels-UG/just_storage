use axum::http::Method;
use axum::{
    body::Body, extract::Request, http::StatusCode, middleware::Next, response::IntoResponse,
    response::Response,
};

/// Validate content-type and well-formed JSON for object endpoints
pub async fn validate_json_for_objects(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // Only validate POST requests to /v1/objects
    if method == Method::POST && path.starts_with("/v1/objects") {
        // Only enforce content-type validation when a Content-Type header is present
        let ct_opt = request
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok());

        if let Some(ct) = ct_opt {
            // If Content-Type is present but not JSON, reject
            if !ct.starts_with("application/json") {
                return (StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported Media Type")
                    .into_response();
            }

            // Read body bytes and ensure it's valid JSON
            let (parts, body) = request.into_parts();
            let bytes = match axum::body::to_bytes(body, 2 * 1024 * 1024).await {
                Ok(b) => b,
                Err(_) => return (StatusCode::BAD_REQUEST, "Bad Request").into_response(),
            };

            if serde_json::from_slice::<serde_json::Value>(&bytes).is_err() {
                return (StatusCode::BAD_REQUEST, "Malformed JSON").into_response();
            }

            // Reconstruct request with body replaced so downstream handlers can use it
            let req = Request::from_parts(parts, Body::from(bytes));
            return next.run(req).await;
        }

        // No Content-Type header: do not validate here - let auth/handlers decide
        return next.run(request).await;
    }

    next.run(request).await
}
