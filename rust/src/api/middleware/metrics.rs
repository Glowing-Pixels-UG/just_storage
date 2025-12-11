use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::info;

/// Metrics middleware for request tracking and observability
pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();

    // Execute request
    let response = next.run(request).await;

    // Log request metrics
    let duration = start.elapsed();
    let status = response.status();

    info!(
        method = %method,
        uri = %uri,
        status = %status.as_u16(),
        duration_ms = duration.as_millis(),
        "request_completed"
    );

    response
}
