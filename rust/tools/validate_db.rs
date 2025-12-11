use clap::Parser;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::env;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    database_url: Option<String>,

    #[arg(long)]
    fix: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let db_url = cli
        .database_url
        .or_else(|| env::var("DATABASE_URL").ok())
        .expect("DATABASE_URL must be set or passed with --database-url");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // Find objects with invalid status
    let invalid_status_rows = sqlx::query(
        r#"SELECT id, status FROM objects WHERE status NOT IN ('WRITING','COMMITTED','DELETING','DELETED')"#
    )
    .fetch_all(&pool)
    .await?;

    println!("Invalid status rows: {}", invalid_status_rows.len());
    for r in invalid_status_rows.iter() {
        let id: uuid::Uuid = r.get("id");
        let status: String = r.get("status");
        println!("id: {}, status: {}", id, status);
        if cli.fix {
            // map to a safe default (WRITING)
            sqlx::query("UPDATE objects SET status = 'WRITING' WHERE id = $1")
                .bind(id)
                .execute(&pool)
                .await?;
            println!("Fixed: {} -> WRITING", id);
        }
    }

    // Find objects with invalid storage_class
    let invalid_storage_rows = sqlx::query(
        r#"SELECT id, storage_class FROM objects WHERE storage_class NOT IN ('hot','cold')"#,
    )
    .fetch_all(&pool)
    .await?;

    println!("Invalid storage_class rows: {}", invalid_storage_rows.len());
    for r in invalid_storage_rows.iter() {
        let id: uuid::Uuid = r.get("id");
        let storage_class: String = r.get("storage_class");
        println!("id: {}, storage_class: {}", id, storage_class);
        if cli.fix {
            sqlx::query("UPDATE objects SET storage_class = 'hot' WHERE id = $1")
                .bind(id)
                .execute(&pool)
                .await?;
            println!("Fixed: {} -> hot", id);
        }
    }

    // Find blobs with invalid content hash (not 64 hex characters)
    let invalid_blob_hashes = sqlx::query(
        r#"SELECT id, content_hash FROM objects WHERE content_hash IS NOT NULL AND length(content_hash) != 64"#
    )
    .fetch_all(&pool)
    .await?;

    println!("Invalid content_hash rows: {}", invalid_blob_hashes.len());
    for r in invalid_blob_hashes.iter() {
        let id: uuid::Uuid = r.get("id");
        let content_hash: Option<String> = r.get("content_hash");
        println!("id: {}, content_hash: {:?}", id, content_hash);
    }

    Ok(())
}
