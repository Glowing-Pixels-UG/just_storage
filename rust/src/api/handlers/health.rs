use axum::{extract::State, http::StatusCode, response::Json};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;

/// GET /health
/// Basic health check endpoint (no database check)
pub async fn health_handler() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "service": "activestorage",
            "version": env!("CARGO_PKG_VERSION")
        })),
    )
}

/// GET /health/ready
/// Readiness probe with database connectivity check
pub async fn readiness_handler(
    State(pool): State<Arc<PgPool>>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Check database connectivity
    match sqlx::query("SELECT 1").fetch_one(pool.as_ref()).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "status": "ready",
                "service": "activestorage",
                "database": "connected"
            })),
        ),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "service": "activestorage",
                "database": "disconnected",
                "error": e.to_string()
            })),
        ),
    }
}
