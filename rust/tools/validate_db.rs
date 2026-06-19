use clap::Parser;
use sqlx::postgres::PgConnectOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::env;
use std::str::FromStr;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    database_url: Option<String>,

    /// Apply SQLx migrations before validating.
    #[arg(long)]
    migrate: bool,

    /// Repair invalid rows where the tool has a safe deterministic fix.
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

    let connect_options = PgConnectOptions::from_str(&db_url)
        .map_err(|error| anyhow::anyhow!("Invalid database URL: {error}"))?
        .statement_cache_capacity(0);

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    println!("Database reachable: yes");

    let migrator = sqlx::migrate!("./migrations");
    let expected_migrations = migrator.migrations.len() as i64;

    if cli.migrate {
        println!("Running SQLx migrations before validation");
        migrator.run(&pool).await?;
    } else {
        println!("Migration mode: read-only (--migrate not set)");
    }

    print_migration_status(&pool, expected_migrations).await?;
    print_table_counts(&pool).await?;

    if !table_exists(&pool, "public.objects").await? {
        println!("objects table missing; database classification: reachable_without_schema");
        return Ok(());
    }

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

    // Find objects with invalid content hash (not 64 hex characters)
    let invalid_blob_hashes = sqlx::query(
        r#"SELECT id, content_hash
           FROM objects
           WHERE content_hash IS NOT NULL
             AND (length(content_hash) != 64 OR content_hash !~ '^[0-9a-fA-F]{64}$')"#,
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

async fn print_migration_status(pool: &sqlx::PgPool, expected: i64) -> anyhow::Result<()> {
    if !table_exists(pool, "public._sqlx_migrations").await? {
        println!("SQLx migrations table: missing (expected {expected})");
        return Ok(());
    }

    let applied = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM _sqlx_migrations")
        .fetch_one(pool)
        .await?;
    let status = if applied >= expected {
        "complete"
    } else {
        "incomplete"
    };

    println!("SQLx migrations: applied={applied} expected={expected} status={status}");
    Ok(())
}

async fn print_table_counts(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    for (label, relation) in [
        ("objects", "public.objects"),
        ("blobs", "public.blobs"),
        ("api_keys", "public.api_keys"),
        ("audit_logs", "public.audit_logs"),
        ("_sqlx_migrations", "public._sqlx_migrations"),
    ] {
        if table_exists(pool, relation).await? {
            let count = count_relation(pool, relation).await?;
            println!("table_count {label}: {count}");
        } else {
            println!("table_count {label}: missing");
        }
    }

    Ok(())
}

async fn table_exists(pool: &sqlx::PgPool, relation: &str) -> anyhow::Result<bool> {
    let exists = sqlx::query_scalar::<_, Option<String>>("SELECT to_regclass($1)::text")
        .bind(relation)
        .fetch_one(pool)
        .await?;
    Ok(exists.is_some())
}

async fn count_relation(pool: &sqlx::PgPool, relation: &str) -> anyhow::Result<i64> {
    let query = match relation {
        "public.objects" => "SELECT COUNT(*) FROM public.objects",
        "public.blobs" => "SELECT COUNT(*) FROM public.blobs",
        "public.api_keys" => "SELECT COUNT(*) FROM public.api_keys",
        "public.audit_logs" => "SELECT COUNT(*) FROM public.audit_logs",
        "public._sqlx_migrations" => "SELECT COUNT(*) FROM public._sqlx_migrations",
        _ => return Err(anyhow::anyhow!("unsupported relation: {relation}")),
    };

    Ok(sqlx::query_scalar::<_, i64>(query).fetch_one(pool).await?)
}
