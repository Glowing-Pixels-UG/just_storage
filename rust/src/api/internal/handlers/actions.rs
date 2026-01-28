use crate::api::middleware::audit::{AuditEventType, AuditLogEntry};
use crate::api::router::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use time::OffsetDateTime;
use serde_json::json;

pub async fn clear_cache(State(state): State<AppState>) -> impl IntoResponse {
    // 1. Perform action (In this case, we don't have a global cache to clear yet, but we'll simulate it)
    tracing::info!("Internal action: Clear cache triggered");

    // 2. Log to AuditRepository
    let log_entry = AuditLogEntry {
        timestamp: OffsetDateTime::now_utc(),
        event_type: AuditEventType::ConfigurationChange,
        user_id: Some("internal-admin".to_string()),
        tenant_id: None,
        api_key_id: None,
        ip_address: None,
        user_agent: None,
        method: "POST".to_string(),
        path: "/internal/actions/cache/clear".to_string(),
        query: None,
        status_code: Some(StatusCode::OK.as_u16()),
        response_time_ms: Some(0),
        error_message: None,
        additional_data: Some(json!({ "action": "clear_cache" })),
    };

    if let Err(e) = state.audit_repo.store(log_entry).await {
        tracing::error!("Failed to store audit log for clear_cache: {}", e);
    }

    StatusCode::OK
}

pub async fn reindex(State(state): State<AppState>) -> impl IntoResponse {
    // 1. Perform action (Trigger GC as a surrogate for reindex for now)
    tracing::info!("Internal action: Reindex/GC triggered");

    let result_msg = if let Some(gc) = &state.gc {
        match gc.collect_once().await {
            Ok(result) => format!(
                "GC run successful: {} objects deleted",
                result.total_deleted
            ),
            Err(e) => format!("GC run failed: {}", e),
        }
    } else {
        "Garbage collector not initialized".to_string()
    };

    // 2. Log to AuditRepository
    let log_entry = AuditLogEntry {
        timestamp: OffsetDateTime::now_utc(),
        event_type: AuditEventType::ConfigurationChange,
        user_id: Some("internal-admin".to_string()),
        tenant_id: None,
        api_key_id: None,
        ip_address: None,
        user_agent: None,
        method: "POST".to_string(),
        path: "/internal/actions/reindex".to_string(),
        query: None,
        status_code: Some(StatusCode::OK.as_u16()),
        response_time_ms: Some(0),
        error_message: if result_msg.contains("failed") {
            Some(result_msg.clone())
        } else {
            None
        },
        additional_data: Some(json!({ "action": "reindex", "result": result_msg })),
    };

    if let Err(e) = state.audit_repo.store(log_entry).await {
        tracing::error!("Failed to store audit log for reindex: {}", e);
    }

    (StatusCode::OK, result_msg)
}
