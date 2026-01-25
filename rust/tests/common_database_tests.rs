mod common;
use common::database::{setup_test_database, cleanup_test_data};

#[tokio::test]
async fn test_setup_and_cleanup_database() {
    let (pool, _container) = setup_test_database().await;

    // Simple query to ensure DB is reachable
    let row: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("query failed");
    assert_eq!(row.0, 1);

    // Cleanup should run without panic
    cleanup_test_data(&pool).await;
}
