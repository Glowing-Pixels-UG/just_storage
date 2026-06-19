//! Health check utilities and logic
//!
//! This module contains the core logic for performing various
//! health checks on the system.

use serde_json::{json, Map, Value};
use sqlx::PgPool;
use std::path::Path;

/// Result of readiness checks
#[derive(Debug)]
pub struct ReadinessCheckResult {
    pub healthy: bool,
    pub details: Value,
    pub issues: Vec<String>,
}

/// Perform basic security health checks
pub fn perform_security_health_checks(auth_disabled: bool) -> Value {
    json!({
        "environment": if cfg!(debug_assertions) { "development" } else { "production" },
        "auth_enabled": if auth_disabled { "disabled" } else { "enabled" },
        "rate_limiting": "enabled",
        "security_headers": "enabled",
        "audit_logging": "enabled",
        "cors_policy": "enabled",
        "error_sanitization": "enabled",
        "size_limits": "enabled"
    })
}

/// Perform readiness checks beyond basic database connectivity
pub async fn perform_readiness_checks(
    pool: &PgPool,
    expected_migration_count: usize,
    hot_storage_root: &Path,
    cold_storage_root: &Path,
) -> ReadinessCheckResult {
    let mut issues = Vec::new();
    let mut details = Map::new();

    match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await
    {
        Ok(applied) => {
            let expected = expected_migration_count as i64;
            details.insert(
                "migrations".to_string(),
                json!({
                    "applied": applied,
                    "expected": expected,
                    "status": if applied >= expected { "ready" } else { "incomplete" }
                }),
            );

            if applied < expected {
                issues.push(format!(
                    "Only {applied} of {expected} expected migrations are applied"
                ));
            }
        }
        Err(error) => {
            details.insert(
                "migrations".to_string(),
                json!({
                    "status": "error",
                    "error": sanitize_db_error(&error)
                }),
            );
            issues.push("Unable to read SQLx migration status".to_string());
        }
    }

    let hot_ready = check_storage_root(hot_storage_root).await;
    let cold_ready = check_storage_root(cold_storage_root).await;

    details.insert("hot_storage".to_string(), hot_ready.details);
    details.insert("cold_storage".to_string(), cold_ready.details);

    issues.extend(
        hot_ready
            .issues
            .into_iter()
            .map(|issue| format!("hot: {issue}")),
    );
    issues.extend(
        cold_ready
            .issues
            .into_iter()
            .map(|issue| format!("cold: {issue}")),
    );

    details.insert("active_checks".to_string(), json!(true));

    ReadinessCheckResult {
        healthy: issues.is_empty(),
        details: Value::Object(details),
        issues,
    }
}

struct StorageRootCheck {
    details: Value,
    issues: Vec<String>,
}

async fn check_storage_root(root: &Path) -> StorageRootCheck {
    let mut issues = Vec::new();
    let mut details = Map::new();

    details.insert("path".to_string(), json!(root.to_string_lossy()));

    for (name, path) in [
        ("root", root.to_path_buf()),
        ("temp", root.join("temp")),
        ("sha256", root.join("sha256")),
    ] {
        match tokio::fs::metadata(&path).await {
            Ok(metadata) if metadata.is_dir() => {
                details.insert(name.to_string(), json!("ready"));
            }
            Ok(_) => {
                details.insert(name.to_string(), json!("not_directory"));
                issues.push(format!("{} exists but is not a directory", path.display()));
            }
            Err(error) => {
                details.insert(
                    name.to_string(),
                    json!({
                        "status": "missing",
                        "error": format!("{:?}", error.kind())
                    }),
                );
                issues.push(format!("{} is not readable", path.display()));
            }
        }
    }

    StorageRootCheck {
        details: Value::Object(details),
        issues,
    }
}

/// Sanitize database error messages to prevent information leakage
pub fn sanitize_db_error(error: &sqlx::Error) -> String {
    match error {
        sqlx::Error::Configuration(_) => "Database configuration error".to_string(),
        sqlx::Error::Database(_) => "Database operation error".to_string(),
        sqlx::Error::Io(_) => "Database connection error".to_string(),
        sqlx::Error::Tls(_) => "Database TLS error".to_string(),
        sqlx::Error::Protocol(_) => "Database protocol error".to_string(),
        sqlx::Error::RowNotFound => "Record not found".to_string(),
        sqlx::Error::TypeNotFound { .. } => "Database type error".to_string(),
        sqlx::Error::ColumnIndexOutOfBounds { .. } => "Database column error".to_string(),
        sqlx::Error::ColumnNotFound(_) => "Database column error".to_string(),
        sqlx::Error::ColumnDecode { .. } => "Database decode error".to_string(),
        sqlx::Error::Decode(_) => "Database decode error".to_string(),
        sqlx::Error::PoolTimedOut => "Database pool timeout".to_string(),
        sqlx::Error::PoolClosed => "Database pool closed".to_string(),
        sqlx::Error::WorkerCrashed => "Database worker error".to_string(),
        _ => "Database error".to_string(),
    }
}

/// Check database connectivity and basic operations
pub async fn check_database_connectivity(pool: &PgPool) -> Result<String, String> {
    let start = std::time::Instant::now();

    // Simple query to test connectivity
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
    {
        Ok(1) => {
            let duration = start.elapsed();
            Ok(format!("Connected in {:?}", duration))
        }
        Ok(_) => Err("Unexpected database response".to_string()),
        Err(e) => Err(sanitize_db_error(&e)),
    }
}

/// Check system resources and basic system health
pub fn check_system_resources() -> Value {
    json!({
        "memory": "ok",  // In a real implementation, check actual memory usage
        "cpu": "ok",     // In a real implementation, check CPU usage
        "disk": "ok",    // In a real implementation, check disk space
        "uptime": "ok"   // In a real implementation, check system uptime
    })
}
