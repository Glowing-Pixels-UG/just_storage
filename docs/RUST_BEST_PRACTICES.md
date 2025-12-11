# Rust Best Practices for ActiveStorage

A comprehensive guide to building a safe, concurrent, and performant object storage service in Rust.

---

## Table of Contents

1. [Architecture & Layer Separation](#architecture--layer-separation)
2. [Storage Safety Patterns](#storage-safety-patterns)
3. [Concurrency & Safety](#concurrency--safety)
4. [Database Patterns](#database-patterns)
5. [API Design with Axum](#api-design-with-axum)
6. [Background Jobs](#background-jobs)
7. [Things to Avoid](#things-to-avoid)
8. [Complete Code Examples](#complete-code-examples)

---

## Architecture & Layer Separation

### Layer Boundaries

Keep your code organized in clear layers that don't leak abstractions:

```
┌─────────────────────────────────┐
│     API Layer (Axum)            │ ← HTTP, auth, validation
├─────────────────────────────────┤
│     Service Layer               │ ← Business logic
├─────────────────────────────────┤
│     Persistence Layer           │ ← DB repos, FS backend
├─────────────────────────────────┤
│     Domain Layer                │ ← Core types
└─────────────────────────────────┘
```

**Domain Layer** (`src/domain/`)

- Pure types: `Object`, `Blob`, `StorageClass`, `ObjectStatus`
- No HTTP, no DB details
- Serialization only

**Persistence Layer** (`src/storage/`, `src/db/`)

- Filesystem backend: `LocalFsBackend`
- DB repositories via `sqlx`
- Knows file paths, SQL queries
- No HTTP concepts

**Service Layer** (`src/service/`)

- Orchestrates upload, download, delete, GC
- Uses persistence, exposes pure functions
- Transaction boundaries here

**API Layer** (`src/api/`)

- `axum` handlers
- Request/response mapping
- Auth extraction
- Error → HTTP status mapping

### Key Principle

**Keep axum handlers stupid. The brain is in the service layer.**

---

## Storage Safety Patterns

### 1. Content-Addressable Layout

```
/data/hot/
  tmp/                 ← Temp uploads
    upload-<uuid>
  sha256/              ← Content-addressed
    ab/                ← Fan-out by first 2 chars
      abcdef123...     ← Full hash as filename
```

**Why:**

- Predictable paths
- Automatic deduplication
- Integrity verification built-in
- Avoids huge directories

### 2. Atomic Writes: ALWAYS Use Temp + Rename

```rust
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use sha2::{Sha256, Digest};
use uuid::Uuid;

async fn write_blob_atomic(
    root: &Path,
    mut reader: impl AsyncRead + Unpin,
) -> Result<(String, u64), StorageError> {
    // 1. Write to temp file
    let tmp_path = root.join("tmp").join(Uuid::new_v4().to_string());
    fs::create_dir_all(tmp_path.parent().unwrap()).await?;

    let mut file = fs::File::create(&tmp_path).await?;
    let mut hasher = Sha256::new();
    let mut total_bytes = 0u64;

    // 2. Stream + hash in one pass
    let mut buffer = vec![0u8; 64 * 1024]; // 64KB chunks
    loop {
        let n = reader.read(&mut buffer).await?;
        if n == 0 { break; }

        hasher.update(&buffer[..n]);
        file.write_all(&buffer[..n]).await?;
        total_bytes += n as u64;
    }

    // 3. Flush + fsync (critical for durability)
    file.flush().await?;
    file.sync_all().await?;
    drop(file); // Close before rename

    // 4. Compute final hash
    let content_hash = format!("sha256:{:x}", hasher.finalize());

    // 5. Derive final path
    let final_path = hash_to_path(root, &content_hash);
    fs::create_dir_all(final_path.parent().unwrap()).await?;

    // 6. Atomic rename (POSIX guarantee)
    if !final_path.exists() {
        fs::rename(&tmp_path, &final_path).await?;

        // 7. fsync parent dir (persist rename)
        let parent = fs::File::open(final_path.parent().unwrap()).await?;
        parent.sync_all().await?;
    } else {
        // Blob already exists (deduplication)
        fs::remove_file(&tmp_path).await?;
    }

    Ok((content_hash, total_bytes))
}

fn hash_to_path(root: &Path, hash: &str) -> PathBuf {
    let hash_str = hash.strip_prefix("sha256:").unwrap_or(hash);
    let prefix = &hash_str[..2];
    root.join("sha256").join(prefix).join(hash_str)
}
```

**Critical Points:**

- ✅ Never expose partially written files
- ✅ `fsync()` before DB commit prevents "DB says exists, file missing" after crash
- ✅ Compute hash while streaming (don't re-read)
- ✅ `rename()` is atomic on POSIX

### 3. Never Block in Async Code

❌ **Wrong:**

```rust
// This blocks the async executor!
use std::fs::File;
let file = File::open("data.bin")?; // WRONG
```

✅ **Right:**

```rust
// Non-blocking async I/O
use tokio::fs::File;
let file = File::open("data.bin").await?; // CORRECT
```

**For CPU-heavy work:**

```rust
use tokio::task;

// Offload to blocking thread pool
let hash = task::spawn_blocking(move || {
    compute_expensive_hash(&large_data)
}).await?;
```

---

## Concurrency & Safety

### The Core Protocol: DB is Truth, FS is Cache

**Never trust the filesystem state. Always trust the database.**

```
Database (source of truth)
    ↓
objects.status = 'committed'
    ↓
Only then file is "visible"
```

### State Machine

```
┌──────────┐
│  (none)  │
└────┬─────┘
     │ INSERT status='writing'
     ↓
┌──────────┐
│ WRITING  │ ← Reserved, not visible, file being written
└────┬─────┘
     │ File written + fsync + UPDATE status='committed'
     ↓
┌──────────┐
│COMMITTED │ ← Visible to readers
└────┬─────┘
     │ UPDATE status='deleting'
     ↓
┌──────────┐
│ DELETING │ ← Marked, file still exists
└────┬─────┘
     │ GC removes file
     ↓
┌──────────┐
│ DELETED  │ ← Gone
└──────────┘
```

### Handling Concurrent Writes

**Strategy A: First-Wins (Recommended for v1)**

```rust
// Postgres enforces uniqueness
let result = sqlx::query(
    r#"
    INSERT INTO objects (id, namespace, tenant_id, key, status, storage_class)
    VALUES ($1, $2, $3, $4, 'WRITING', $5)
    "#
)
.bind(object_id)
.bind(&namespace)
.bind(&tenant_id)
.bind(&key) // UNIQUE constraint on (namespace, tenant_id, key)
.bind(storage_class.as_str())
.execute(&pool)
.await;

match result {
    Ok(_) => { /* proceed */ },
    Err(e) if is_unique_violation(&e) => {
        return Err(StorageError::Conflict("Key already exists".into()));
    },
    Err(e) => return Err(e.into()),
}
```

**Strategy B: Last-Writer-Wins**

```rust
sqlx::query(
    r#"
    INSERT INTO objects (id, namespace, tenant_id, key, status, storage_class)
    VALUES ($1, $2, $3, $4, 'WRITING', $5)
    ON CONFLICT (namespace, tenant_id, key)
    DO UPDATE SET
        status = 'WRITING',
        updated_at = now()
    RETURNING id
    "#
)
.bind(object_id)
.bind(&namespace)
.bind(&tenant_id)
.bind(&key)
.bind(storage_class.as_str())
.fetch_one(&pool)
.await?;
```

### Multiple Service Instances

**You don't need in-memory locks across instances.**

As long as:

- ✅ All instances use the same Postgres DB
- ✅ All instances write to the same filesystem (Longhorn PVC)
- ✅ All follow the two-phase protocol (reserve → write → commit)

**Then you're safe.** The DB handles concurrency.

**Optional optimization** (reduces DB conflicts within one instance):

```rust
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

struct KeyLocks {
    locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl KeyLocks {
    async fn lock(&self, key: &str) -> Arc<Mutex<()>> {
        let mut map = self.locks.lock().await;
        map.entry(key.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }
}

// Usage in upload
let lock = key_locks.lock(&format!("{}/{}/{}", namespace, tenant, key)).await;
let _guard = lock.lock().await;
// Now proceed with DB + file write
```

**But:** This is an optimization, not a requirement. The DB is still the authority.

### Handling Deletes vs. Reads

**Problem:** Reader streaming file, another client deletes it.

**Solution:** Deferred GC.

```rust
// DELETE API handler
async fn delete_object(id: Uuid, pool: &PgPool) -> Result<(), StorageError> {
    sqlx::query(
        r#"
        UPDATE objects SET status = 'DELETING'
        WHERE id = $1 AND status = 'COMMITTED'
        RETURNING content_hash
        "#
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(StorageError::NotFound)?;

    // Decrement ref count
    sqlx::query(
        r#"
        UPDATE blobs SET ref_count = ref_count - 1
        WHERE content_hash = (SELECT content_hash FROM objects WHERE id = $1)
        "#
    )
    .bind(id)
    .execute(pool)
    .await?;

    // File stays on disk!
    // Background GC removes it later when ref_count = 0

    Ok(())
}
```

**Why this works:**

- Reader has open file descriptor → continues working even after `unlink()`
- No coordination needed beyond filesystem semantics

---

## Database Patterns

### Schema (Simplified)

```sql
CREATE TYPE object_status AS ENUM ('WRITING', 'COMMITTED', 'DELETING', 'DELETED');
CREATE TYPE storage_class AS ENUM ('hot', 'cold');

CREATE TABLE objects (
    id              UUID PRIMARY KEY,
    namespace       TEXT NOT NULL,
    tenant_id       TEXT NOT NULL,
    key             TEXT,
    status          object_status NOT NULL,
    storage_class   storage_class NOT NULL,
    content_hash    TEXT,
    size_bytes      BIGINT,
    content_type    TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE(namespace, tenant_id, key) WHERE key IS NOT NULL AND status != 'DELETED'
);

CREATE INDEX idx_objects_committed ON objects(tenant_id, namespace)
    WHERE status = 'COMMITTED';

CREATE TABLE blobs (
    content_hash    TEXT PRIMARY KEY,
    storage_class   storage_class NOT NULL,
    ref_count       BIGINT NOT NULL DEFAULT 0 CHECK (ref_count >= 0),
    size_bytes      BIGINT NOT NULL,
    first_seen_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at    TIMESTAMPTZ
);

CREATE INDEX idx_blobs_gc ON blobs(ref_count) WHERE ref_count = 0;
```

### Using sqlx: Compile-Time Checked Queries

```rust
use sqlx::{PgPool, postgres::PgRow, Row};

// Type-safe queries
#[derive(sqlx::FromRow)]
struct ObjectRow {
    id: Uuid,
    namespace: String,
    tenant_id: String,
    key: Option<String>,
    status: String,
    storage_class: String,
    content_hash: Option<String>,
    size_bytes: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn get_object(pool: &PgPool, id: Uuid) -> Result<ObjectRow, sqlx::Error> {
    sqlx::query_as::<_, ObjectRow>(
        r#"
        SELECT id, namespace, tenant_id, key, status, storage_class,
               content_hash, size_bytes, created_at
        FROM objects
        WHERE id = $1 AND status = 'COMMITTED'
        "#
    )
    .bind(id)
    .fetch_one(pool)
    .await
}
```

### Transactions

```rust
use sqlx::{PgPool, Postgres, Transaction};

async fn commit_object(
    pool: &PgPool,
    id: Uuid,
    content_hash: String,
    size_bytes: i64,
    storage_class: &str,
) -> Result<(), StorageError> {
    let mut tx: Transaction<Postgres> = pool.begin().await?;

    // Update object status
    sqlx::query(
        r#"
        UPDATE objects
        SET status = 'COMMITTED',
            content_hash = $2,
            size_bytes = $3
        WHERE id = $1 AND status = 'WRITING'
        "#
    )
    .bind(id)
    .bind(&content_hash)
    .bind(size_bytes)
    .execute(&mut *tx)
    .await?;

    // Update blob ref count
    sqlx::query(
        r#"
        INSERT INTO blobs (content_hash, storage_class, ref_count, size_bytes)
        VALUES ($1, $2, 1, $3)
        ON CONFLICT (content_hash)
        DO UPDATE SET
            ref_count = blobs.ref_count + 1,
            last_used_at = now()
        "#
    )
    .bind(&content_hash)
    .bind(storage_class)
    .bind(size_bytes)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
```

---

## API Design with Axum

### Stack

```toml
[dependencies]
axum = "0.7"
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["serde", "v4"] }
sha2 = "0.10"
hex = "0.4"
thiserror = "1"
```

### Basic Server Setup

```rust
use axum::{
    Router,
    routing::{get, post, delete},
    extract::State,
};
use std::sync::Arc;
use sqlx::PgPool;

#[derive(Clone)]
struct AppState {
    db: PgPool,
    object_service: Arc<ObjectService>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
        )
        .init();

    // DB connection
    let db_url = std::env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&db_url).await?;

    // Service
    let object_service = Arc::new(ObjectService::new(pool.clone()));

    let state = AppState {
        db: pool,
        object_service,
    };

    // Routes
    let app = Router::new()
        .route("/v1/objects", post(upload_handler))
        .route("/v1/objects/:id", get(download_handler))
        .route("/v1/objects/:id", delete(delete_handler))
        .route("/health", get(health_handler))
        .with_state(state)
        .layer(
            tower_http::trace::TraceLayer::new_for_http()
        );

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    tracing::info!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
```

### Upload Handler (Streaming)

```rust
use axum::{
    body::Body,
    extract::{State, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use futures::TryStreamExt;

#[derive(serde::Deserialize)]
struct UploadHeaders {
    namespace: String,
    tenant_id: String,
    key: Option<String>,
    storage_class: String,
}

async fn upload_handler(
    State(app): State<AppState>,
    headers: HeaderMap,
    body: Body,
) -> Result<impl IntoResponse, ApiError> {
    // Extract headers
    let namespace = extract_header(&headers, "X-Namespace")?;
    let tenant_id = extract_header(&headers, "X-Tenant")?;
    let key = headers.get("X-Key")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let storage_class = headers.get("X-Storage-Class")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("hot");

    // Convert body to AsyncRead
    let stream = body.into_data_stream()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
    let reader = tokio_util::io::StreamReader::new(stream);

    // Call service
    let params = UploadParams {
        namespace,
        tenant_id,
        key,
        storage_class: parse_storage_class(storage_class)?,
    };

    let meta = app.object_service.upload(params, reader).await?;

    Ok((StatusCode::CREATED, Json(meta)))
}

fn extract_header(headers: &HeaderMap, name: &str) -> Result<String, ApiError> {
    headers.get(name)
        .and_then(|v| v.to_str().ok())
        .map(String::from)
        .ok_or_else(|| ApiError::MissingHeader(name.to_string()))
}
```

### Error Handling

```rust
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};

#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("object not found")]
    NotFound,

    #[error("object already exists")]
    Conflict,

    #[error("hash mismatch")]
    HashMismatch,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

pub enum ApiError {
    Storage(StorageError),
    MissingHeader(String),
    InvalidHeader(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Storage(StorageError::NotFound) => {
                (StatusCode::NOT_FOUND, "Object not found")
            }
            ApiError::Storage(StorageError::Conflict) => {
                (StatusCode::CONFLICT, "Object already exists")
            }
            ApiError::Storage(StorageError::HashMismatch) => {
                (StatusCode::BAD_REQUEST, "Content hash mismatch")
            }
            ApiError::MissingHeader(name) => {
                (StatusCode::BAD_REQUEST, "Missing required header")
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error"),
        };

        (status, Json(serde_json::json!({
            "error": message
        }))).into_response()
    }
}

impl From<StorageError> for ApiError {
    fn from(e: StorageError) -> Self {
        ApiError::Storage(e)
    }
}
```

---

## Background Jobs

### GC Worker

```rust
use tokio::time::{interval, Duration};
use sqlx::PgPool;

pub struct GarbageCollector {
    db: PgPool,
    hot_root: PathBuf,
    cold_root: PathBuf,
}

impl GarbageCollector {
    pub fn new(db: PgPool, hot_root: PathBuf, cold_root: PathBuf) -> Self {
        Self { db, hot_root, cold_root }
    }

    pub async fn run(self: Arc<Self>) {
        let mut tick = interval(Duration::from_secs(60));

        loop {
            tick.tick().await;

            if let Err(e) = self.collect_garbage().await {
                tracing::error!("GC error: {}", e);
            }
        }
    }

    async fn collect_garbage(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Find blobs ready for deletion
        let blobs: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT content_hash, storage_class
            FROM blobs
            WHERE ref_count = 0
            LIMIT 100
            "#
        )
        .fetch_all(&self.db)
        .await?;

        if blobs.is_empty() {
            return Ok(());
        }

        tracing::info!("GC: found {} blobs to delete", blobs.len());

        for (content_hash, storage_class) in blobs {
            let path = self.blob_path(&content_hash, &storage_class);

            // Delete file
            match tokio::fs::remove_file(&path).await {
                Ok(_) => {
                    tracing::info!("GC: deleted {}", path.display());
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // Already gone, fine
                }
                Err(e) => {
                    tracing::error!("GC: failed to delete {}: {}", path.display(), e);
                    continue;
                }
            }

            // Remove from DB
            sqlx::query("DELETE FROM blobs WHERE content_hash = $1")
                .bind(&content_hash)
                .execute(&self.db)
                .await?;
        }

        Ok(())
    }

    fn blob_path(&self, content_hash: &str, storage_class: &str) -> PathBuf {
        let root = if storage_class == "hot" {
            &self.hot_root
        } else {
            &self.cold_root
        };

        let hash = content_hash.strip_prefix("sha256:").unwrap_or(content_hash);
        let prefix = &hash[..2];

        root.join("sha256").join(prefix).join(hash)
    }
}

// Start in main
tokio::spawn(async move {
    gc_worker.run().await;
});
```

---

## Things to Avoid

### ❌ 1. Never `unwrap()` in Storage Code

```rust
// BAD
let file = tokio::fs::File::open(path).await.unwrap();

// GOOD
let file = tokio::fs::File::open(path).await
    .map_err(|e| StorageError::Io(e))?;
```

### ❌ 2. Don't Build File Paths from User Input

```rust
// BAD - Path traversal vulnerability!
let path = format!("/data/{}/{}", namespace, user_key);

// GOOD - Only use content-hash-derived paths
let path = hash_to_path(root, content_hash);
```

### ❌ 3. Don't Mix Blocking and Async I/O

```rust
// BAD
use std::fs::File; // Blocking!
let file = File::open("data")?;

// GOOD
use tokio::fs::File; // Async
let file = File::open("data").await?;
```

### ❌ 4. Don't Read Entire Body into Memory

```rust
// BAD - OOM on large uploads!
let bytes = axum::body::to_bytes(body, usize::MAX).await?;

// GOOD - Stream it
let stream = body.into_data_stream();
let reader = StreamReader::new(stream);
```

### ❌ 5. Don't Overuse Generics in Service Layer

```rust
// BAD - Unnecessary complexity
pub struct ObjectService<R, B, D>
where
    R: AsyncRead + Send,
    B: BlobBackend<R>,
    D: Database,
{ ... }

// GOOD - Concrete types
pub struct ObjectService {
    db: PgPool,
    fs_backend: LocalFsBackend,
}
```

### ❌ 6. Don't Ignore fsync

```rust
// BAD - May lose data on crash
file.write_all(&data).await?;
// Missing fsync!

// GOOD
file.write_all(&data).await?;
file.flush().await?;
file.sync_all().await?; // Critical for durability
```

---

## Complete Code Examples

See:

- `rust/src/storage/object_store.rs` - Trait definition
- `rust/src/storage/local_fs.rs` - LocalFsObjectStore implementation
- `IMPLEMENTATION.md` - Full examples with handlers

---

## Key Takeaways

1. **DB is truth, FS is cache** - Always check `status='COMMITTED'` in DB
2. **Atomic writes** - Temp file + fsync + rename, always
3. **No blocking in async** - Use `tokio::fs`, not `std::fs`
4. **Stream, don't buffer** - Pass readers through, never load into memory
5. **Content-addressable** - Hash-based paths, deduplication built-in
6. **Deferred GC** - Never delete files immediately on API delete
7. **DB handles concurrency** - Trust Postgres, not in-memory locks
8. **Error handling** - No `unwrap()`, structured errors only
9. **Layer separation** - Keep axum handlers dumb, logic in service layer
10. **fsync before commit** - Critical for crash safety

---

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_upload_download_lifecycle() {
        // Setup test DB + temp dirs
        // Upload object
        // Download and verify
        // Delete
        // Verify gone
    }

    #[tokio::test]
    async fn test_concurrent_uploads_same_key() {
        // Spawn multiple tasks uploading to same key
        // Verify only one succeeds (or last-wins behavior)
    }

    #[tokio::test]
    async fn test_crash_during_upload() {
        // Simulate crash after file write, before DB commit
        // Verify object not visible
        // Run GC cleanup
        // Verify orphan removed
    }
}
```

---

This guide provides the foundation for building a production-ready object storage service in Rust. Follow these patterns, avoid the listed pitfalls, and you'll have a safe, concurrent, and performant system.
