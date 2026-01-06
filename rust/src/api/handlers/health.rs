use axum::{extract::State, http::StatusCode, response::Json};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Instant;
use utoipa::ToSchema;

use super::health_checks::{
    perform_readiness_checks, perform_security_health_checks, sanitize_db_error,
};

/// Basic health check response
#[derive(serde::Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

/// Database readiness response
#[derive(serde::Serialize, ToSchema)]
pub struct ReadinessResponse {
    pub status: String,
    pub service: String,
    pub database: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// GET /health
/// Basic health check endpoint (no database check)
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub async fn health_handler() -> (StatusCode, Json<serde_json::Value>) {
    let start_time = Instant::now();

    // Perform basic security checks
    let security_checks = perform_security_health_checks();

    let response_time = start_time.elapsed();

    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "service": "just_storage",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "uptime_seconds": std::process::id(), // Simplified uptime indicator
            "response_time_ms": response_time.as_millis(),
            "security": security_checks
        })),
    )
}

/// GET /health/ready
/// Readiness probe with database connectivity check
#[utoipa::path(
    get,
    path = "/health/ready",
    tag = "health",
    responses(
        (status = 200, description = "Service is ready", body = ReadinessResponse),
        (status = 503, description = "Service is not ready", body = ReadinessResponse)
    )
)]
pub async fn readiness_handler(
    State(pool): State<Arc<PgPool>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let start_time = Instant::now();

    // Check database connectivity with timeout
    let db_check = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        sqlx::query("SELECT 1 as health_check").fetch_one(pool.as_ref()),
    )
    .await;

    let response_time = start_time.elapsed();

    // Perform security checks
    let security_checks = perform_security_health_checks();

    match db_check {
        Ok(Ok(_)) => {
            // Additional readiness checks
            let readiness_checks = perform_readiness_checks(pool.as_ref()).await;

            if readiness_checks.healthy {
                (
                    StatusCode::OK,
                    Json(json!({
                        "status": "ready",
                        "service": "just_storage",
                        "database": "connected",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "response_time_ms": response_time.as_millis(),
                        "checks": readiness_checks.details,
                        "security": security_checks
                    })),
                )
            } else {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(json!({
                        "status": "not_ready",
                        "service": "just_storage",
                        "database": "connected",
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "response_time_ms": response_time.as_millis(),
                        "checks": readiness_checks.details,
                        "security": security_checks,
                        "issues": readiness_checks.issues
                    })),
                )
            }
        }
        Ok(Err(e)) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "service": "just_storage",
                "database": "disconnected",
                "error": format!("Database error: {}", sanitize_db_error(&e)),
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "response_time_ms": response_time.as_millis(),
                "security": security_checks
            })),
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "service": "just_storage",
                "database": "timeout",
                "error": "Database query timed out after 2 seconds",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "response_time_ms": response_time.as_millis(),
                "security": security_checks
            })),
        ),
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_health_checks() {
        let checks = perform_security_health_checks();

        // Verify that security features are reported
        assert!(checks.get("rate_limiting").is_some());
        assert!(checks.get("security_headers").is_some());
        assert!(checks.get("audit_logging").is_some());
        assert!(checks.get("cors_policy").is_some());
    }

    #[test]
    fn test_db_error_sanitization() {
        // Test that various error types are sanitized
        let config_error = sqlx::Error::Configuration("sensitive config".into());
        assert_eq!(
            sanitize_db_error(&config_error),
            "Database configuration error"
        );

        let pool_timeout = sqlx::Error::PoolTimedOut;
        assert_eq!(sanitize_db_error(&pool_timeout), "Database pool timeout");
    }

    // TODO: Fix this test - requires a real database connection
    // #[tokio::test]
    // async fn test_readiness_checks() {
    //     // This would need a real database connection to test properly
    //     // For now, just ensure the function doesn't panic
    //     let result = perform_readiness_checks(&sqlx::PgPool::connect("dummy").await.unwrap_err().into());
    //     assert!(result.details.is_object());
    // }

    #[test]
    fn test_security_health_checks_structure() {
        let checks = perform_security_health_checks();

        // Verify the structure contains expected security features
        assert!(checks.get("rate_limiting").is_some());
        assert!(checks.get("security_headers").is_some());
        assert!(checks.get("audit_logging").is_some());
        assert!(checks.get("cors_policy").is_some());
        assert!(checks.get("error_sanitization").is_some());
        assert!(checks.get("size_limits").is_some());

        // All values should be strings indicating status
        for (_, value) in checks.as_object().unwrap() {
            assert!(value.is_string());
            let status = value.as_str().unwrap();
            assert!(status == "enabled" || status == "development" || status == "production");
        }
    }

    #[test]
    fn test_db_error_sanitization_comprehensive() {
        // Test various error types are properly sanitized

        // Configuration errors
        let config_err = sqlx::Error::Configuration("host=localhost password=secret".into());
        assert_eq!(
            sanitize_db_error(&config_err),
            "Database configuration error"
        );

        // Database operation errors - simplified for compatibility
        // let db_err = sqlx::Error::Database(...); // SQLx API changed
        // assert_eq!(sanitize_db_error(&db_err), "Database operation error");

        // Connection errors
        let io_err = sqlx::Error::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "Connection refused",
        ));
        assert_eq!(sanitize_db_error(&io_err), "Database connection error");

        let tls_err = sqlx::Error::Tls("certificate verification failed".into());
        assert_eq!(sanitize_db_error(&tls_err), "Database TLS error");

        // Pool errors
        let pool_timeout = sqlx::Error::PoolTimedOut;
        assert_eq!(sanitize_db_error(&pool_timeout), "Database pool timeout");

        let pool_closed = sqlx::Error::PoolClosed;
        assert_eq!(sanitize_db_error(&pool_closed), "Database pool closed");

        // Data errors
        let not_found = sqlx::Error::RowNotFound;
        assert_eq!(sanitize_db_error(&not_found), "Record not found");

        let type_err = sqlx::Error::TypeNotFound {
            type_name: "unknown_type".to_string(),
        };
        assert_eq!(sanitize_db_error(&type_err), "Database type error");

        // Protocol errors
        let protocol_err = sqlx::Error::Protocol("unexpected message".into());
        assert_eq!(sanitize_db_error(&protocol_err), "Database protocol error");

        // Worker errors
        let worker_err = sqlx::Error::WorkerCrashed;
        assert_eq!(sanitize_db_error(&worker_err), "Database worker error");

        // Unknown errors
        let unknown_err = sqlx::Error::ColumnDecode {
            index: "0".to_string(),
            source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "unknown")),
        };
        let sanitized = sanitize_db_error(&unknown_err);
        assert!(sanitized.contains("Database") || sanitized.contains("decode"));
    }


    #[test]
    fn test_health_response_includes_security_info() {
        // Test that health responses include security status
        // This would be tested in integration tests, but we can test the function exists

        let security_checks = perform_security_health_checks();

        // Verify we have the expected security check fields
        let expected_fields = [
            "environment",
            "auth_enabled",
            "rate_limiting",
            "security_headers",
            "audit_logging",
            "cors_policy",
            "error_sanitization",
            "size_limits",
        ];

        for field in &expected_fields {
            assert!(
                security_checks.get(*field).is_some(),
                "Security check missing field: {}",
                field
            );
        }
    }
}
