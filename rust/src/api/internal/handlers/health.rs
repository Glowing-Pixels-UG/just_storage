use axum::{
    extract::State,
    response::IntoResponse,
};
use crate::api::router::AppState;
use crate::api::internal::templates::HealthTemplate;
use crate::domain::value_objects::StorageClass;
use std::time::Instant;

pub async fn health_page(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let start = Instant::now();
    let db_ok = sqlx::query("SELECT 1").fetch_one(&*state.pool).await.is_ok();
    let db_latency = format!("{:?}", start.elapsed());

    // Get total objects count
    let total_objects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM objects")
        .fetch_one(&*state.pool)
        .await
        .unwrap_or(0);

    // Check migrations
    let migration_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(&*state.pool)
        .await
        .unwrap_or(0);
    
    // We expect 5 migrations based on the migrations folder
    let migrations_ok = migration_count == 5;

    // Get storage usage
    let hot_usage = state.blob_store.get_total_size(StorageClass::Hot).await.unwrap_or(0);
    let cold_usage = state.blob_store.get_total_size(StorageClass::Cold).await.unwrap_or(0);

    HealthTemplate {
        service_name: "just-storage".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        db_status: if db_ok && migrations_ok { 
            "Connected".to_string() 
        } else if db_ok {
            format!("Connected ({} migrations pending)", 5 - migration_count)
        } else { 
            "Disconnected".to_string() 
        },
        db_latency,
        hot_storage_usage: format_size(hot_usage),
        cold_storage_usage: format_size(cold_usage),
        total_objects,
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
