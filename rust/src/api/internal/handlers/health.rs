use crate::api::internal::templates::HealthTemplate;
use crate::api::middleware::csrf::CsrfToken;
use crate::api::router::AppState;
use crate::domain::value_objects::StorageClass;
use axum::{
    extract::{Extension, State},
    response::IntoResponse,
};
use std::time::{Duration, Instant};

pub async fn health_page(
    State(state): State<AppState>,
    Extension(csrf_token): Extension<CsrfToken>,
) -> impl IntoResponse {
    let start = Instant::now();
    let db_ok = sqlx::query("SELECT 1")
        .fetch_one(&*state.pool)
        .await
        .is_ok();
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

    let migrations_ok = migration_count == state.expected_migration_count as i64;

    // Get storage usage
    let hot_usage = state
        .blob_store
        .get_total_size(StorageClass::Hot)
        .await
        .unwrap_or(0);
    let cold_usage = state
        .blob_store
        .get_total_size(StorageClass::Cold)
        .await
        .unwrap_or(0);

    // GC Info
    let (gc_status, gc_last_run, gc_next_run, gc_total_deleted) = if let Some(gc) = &state.gc {
        let stats = gc.stats();
        let last_run = gc.last_run();
        let interval = gc.config().interval;

        let last_run_str = last_run
            .map(|t| format_duration(t.elapsed()))
            .map(|s| format!("{} ago", s))
            .unwrap_or_else(|| "Never".to_string());

        let next_run_str = last_run
            .map(|t| {
                let elapsed = t.elapsed();
                if elapsed >= interval {
                    "Imminent".to_string()
                } else {
                    format_duration(interval - elapsed)
                }
            })
            .unwrap_or_else(|| "Scheduled".to_string());

        (
            "Active".to_string(),
            last_run_str,
            next_run_str,
            stats.total_items_deleted,
        )
    } else {
        (
            "Disabled".to_string(),
            "N/A".to_string(),
            "N/A".to_string(),
            0,
        )
    };

    HealthTemplate {
        csrf_token: csrf_token.0,
        service_name: "just-storage".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime: format_duration(state.start_time.elapsed()),
        db_status: if db_ok && migrations_ok {
            "Connected".to_string()
        } else if db_ok {
            format!(
                "Connected ({} migrations pending)",
                state.expected_migration_count as i64 - migration_count
            )
        } else {
            "Disconnected".to_string()
        },
        db_latency,
        db_pool_active: (state.pool.size() as usize - state.pool.num_idle()) as u32,
        db_pool_idle: state.pool.num_idle() as u32,
        db_pool_max: state.config.db_max_connections,
        hot_storage_usage: format_size(hot_usage),
        cold_storage_usage: format_size(cold_usage),
        hot_storage_path: state.config.hot_storage_root.to_string_lossy().to_string(),
        cold_storage_path: state.config.cold_storage_root.to_string_lossy().to_string(),
        total_objects,
        gc_status,
        gc_last_run,
        gc_next_run,
        gc_total_deleted,
    }
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else if secs < 86400 {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{}d {}h", secs / 86400, (secs % 86400) / 3600)
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
