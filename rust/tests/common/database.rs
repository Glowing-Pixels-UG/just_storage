#![allow(dead_code)]

//! Database helpers for tests (testcontainers + migrations)

use sqlx::PgPool;
use testcontainers_modules::{postgres::Postgres, testcontainers::runners::AsyncRunner};

/// Start a PostgreSQL container and return a connected PgPool and the container handle
pub async fn setup_test_database() -> (PgPool, testcontainers::ContainerAsync<Postgres>) {
    let init_sql = include_str!("../../../schema.sql");
    let container = Postgres::default()
        .with_init_sql(init_sql.as_bytes().to_vec())
        .start()
        .await
        .expect("Failed to start PostgreSQL container");

    let host = container
        .get_host()
        .await
        .expect("Failed to get container host");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get container port");

    let database_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Clean up any existing test data
    cleanup_test_data(&pool).await;

    (pool, container)
}

/// Remove test data between test runs
pub async fn cleanup_test_data(pool: &PgPool) {
    sqlx::query("DELETE FROM audit_logs")
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM api_keys").execute(pool).await.ok();
    sqlx::query("DELETE FROM objects").execute(pool).await.ok();
    sqlx::query("DELETE FROM blobs").execute(pool).await.ok();
}

/// Create temporary storage directories
pub fn setup_test_storage() -> (tempfile::TempDir, tempfile::TempDir) {
    let hot_dir = tempfile::TempDir::new().expect("Failed to create temp hot storage dir");
    let cold_dir = tempfile::TempDir::new().expect("Failed to create temp cold storage dir");
    (hot_dir, cold_dir)
}
