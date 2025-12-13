# JustStorage Implementation Guide

This guide provides concrete examples and patterns for implementing JustStorage, a content-addressable object storage service built with Rust using Clean Architecture principles.

---

## Project Structure

### Rust Implementation

```
rust/
├── Cargo.toml
├── Cargo.lock
├── migrations/              # SQLx database migrations
│   └── 003_add_metadata.sql
├── benches/                # Criterion benchmarks
│   ├── memory_bench.rs
│   ├── performance_validation.rs
│   ├── resource_bench.rs
│   └── storage_bench.rs
├── tests/                  # Integration tests
│   └── integration_test.rs
├── tools/                  # CLI utilities
│   └── validate_db.rs
└── src/
    ├── main.rs             # Entry point, server setup
    ├── lib.rs              # Library root
    ├── config.rs           # Configuration loading
    ├── domain/             # Domain layer (core business logic)
    │   ├── mod.rs
    │   ├── errors.rs
    │   ├── entities/       # Domain entities
    │   │   ├── mod.rs
    │   │   ├── blob.rs
    │   │   └── object.rs
    │   └── value_objects/  # Value objects
    │       ├── mod.rs
    │       ├── content_hash.rs
    │       ├── metadata.rs
    │       ├── namespace.rs
    │       ├── object_id.rs
    │       ├── object_status.rs
    │       ├── storage_class.rs
    │       └── tenant_id.rs
    ├── application/        # Application layer (use cases)
    │   ├── mod.rs
    │   ├── dto.rs          # Data transfer objects
    │   ├── ports/          # Ports (interfaces)
    │   │   ├── mod.rs
    │   │   ├── blob_repository.rs
    │   │   ├── blob_store.rs
    │   │   └── object_repository.rs
    │   ├── use_cases/      # Use case implementations
│   │   ├── mod.rs
    │   │   ├── upload_object.rs
    │   │   ├── download_object.rs
    │   │   ├── delete_object.rs
    │   │   └── list_objects.rs
    │   └── gc/             # Garbage collection
    │       ├── mod.rs
    │       └── worker.rs
    ├── infrastructure/     # Infrastructure layer
    │   ├── mod.rs
    │   ├── persistence/    # Database implementations
│   │   ├── mod.rs
    │   │   ├── postgres_blob_repository.rs
    │   │   └── postgres_object_repository.rs
    │   └── storage/        # Storage implementations
    │       ├── mod.rs
    │       ├── local_filesystem_store.rs
    │       ├── content_hasher.rs
    │       └── path_builder.rs
    └── api/                # API layer (HTTP handlers)
        ├── mod.rs
        ├── router.rs       # Axum router setup
        ├── errors.rs       # API error types
        ├── handlers/       # HTTP handlers
        │   ├── mod.rs
        │   ├── upload.rs
        │   ├── download.rs
        │   ├── delete.rs
        │   ├── list.rs
        │   ├── health.rs
        │   └── tests/
        │       ├── mod.rs
        │       └── health_tests.rs
        └── middleware/    # HTTP middleware
            ├── mod.rs
            ├── auth.rs     # JWT/API key authentication
            └── metrics.rs  # Request metrics/logging
```

**Architecture Layers:**

- **Domain**: Core business logic, entities, value objects (no dependencies)
- **Application**: Use cases, ports (interfaces), DTOs (depends on domain)
- **Infrastructure**: Database, filesystem implementations (depends on application ports)
- **API**: HTTP handlers, middleware (depends on application use cases)

---

## Configuration

### `config.rs`

```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub hot_storage_root: PathBuf,
    pub cold_storage_root: PathBuf,
    pub listen_addr: String,
    pub gc_interval_secs: u64,
    pub gc_batch_size: i64,
    // Database connection pool settings
    pub db_max_connections: u32,
    pub db_min_connections: u32,
    pub db_acquire_timeout_secs: u64,
    pub db_idle_timeout_secs: u64,
    pub db_max_lifetime_secs: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:password@localhost/activestorage".to_string()),
            hot_storage_root: std::env::var("HOT_STORAGE_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/data/hot")),
            cold_storage_root: std::env::var("COLD_STORAGE_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/data/cold")),
            listen_addr: std::env::var("LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            gc_interval_secs: std::env::var("GC_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            gc_batch_size: std::env::var("GC_BATCH_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            db_max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),
            db_min_connections: std::env::var("DB_MIN_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            db_acquire_timeout_secs: std::env::var("DB_ACQUIRE_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            db_idle_timeout_secs: std::env::var("DB_IDLE_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(600),
            db_max_lifetime_secs: std::env::var("DB_MAX_LIFETIME_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1800),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.database_url.starts_with("postgres://")
            && !self.database_url.starts_with("postgresql://")
        {
            return Err("DATABASE_URL must start with postgres:// or postgresql://".to_string());
        }
        if self.listen_addr.is_empty() {
            return Err("LISTEN_ADDR cannot be empty".to_string());
        }
        if self.gc_interval_secs < 10 {
            return Err("GC_INTERVAL_SECS must be at least 10 seconds".to_string());
        }
        if self.gc_batch_size < 1 || self.gc_batch_size > 1000 {
            return Err("GC_BATCH_SIZE must be between 1 and 1000".to_string());
        }
        Ok(())
    }
}
```

**Environment Variables:**

- `DATABASE_URL` - PostgreSQL connection string
- `HOT_STORAGE_ROOT` - Path to hot storage directory (default: `/data/hot`)
- `COLD_STORAGE_ROOT` - Path to cold storage directory (default: `/data/cold`)
- `LISTEN_ADDR` - Server listen address (default: `0.0.0.0:8080`)
- `GC_INTERVAL_SECS` - Garbage collection interval (default: `60`)
- `GC_BATCH_SIZE` - GC batch size (default: `100`)
- `DB_MAX_CONNECTIONS` - Database pool max connections (default: `20`)
- `DB_MIN_CONNECTIONS` - Database pool min connections (default: `5`)
- `DB_ACQUIRE_TIMEOUT_SECS` - Connection acquire timeout (default: `30`)
- `DB_IDLE_TIMEOUT_SECS` - Connection idle timeout (default: `600`)
- `DB_MAX_LIFETIME_SECS` - Connection max lifetime (default: `1800`)

---

## API Implementation Examples

### Upload Handler (`api/handlers/upload.rs`)

```rust
use axum::{
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use futures_util::StreamExt;
use serde::Deserialize;
use std::sync::Arc;

use crate::api::errors::ApiError;
use crate::application::dto::{ObjectDto, UploadRequest};
use crate::application::use_cases::UploadObjectUseCase;
use crate::domain::value_objects::StorageClass;

#[derive(Deserialize)]
pub struct UploadQuery {
    namespace: String,
    tenant_id: String,
    key: Option<String>,
    storage_class: Option<String>,
}

/// POST /v1/objects
/// Upload object with streaming body
pub async fn upload_handler(
    State(use_case): State<Arc<UploadObjectUseCase>>,
    Query(query): Query<UploadQuery>,
    body: Body,
) -> Result<(StatusCode, Json<ObjectDto>), ApiError> {
    // Parse storage class
    let storage_class = match query.storage_class {
        Some(s) => Some(s.parse::<StorageClass>().map_err(ApiError::bad_request)?),
        None => None,
    };

    // Create request DTO
    let request = UploadRequest {
        namespace: query.namespace,
        tenant_id: query.tenant_id,
        key: query.key,
        storage_class,
    };

    // Convert body to async reader
    let stream = body.into_data_stream();
    let reader = Box::pin(tokio_util::io::StreamReader::new(
        stream.map(|result| result.map_err(std::io::Error::other)),
    ));

    // Execute use case
    let object = use_case.execute(request, reader).await?;

    Ok((StatusCode::CREATED, Json(object)))
}
```

### Upload Use Case (`application/use_cases/upload_object.rs`)

```rust
use std::sync::Arc;
use crate::application::dto::{ObjectDto, UploadRequest};
use crate::application::ports::{
    BlobReader, BlobRepository, BlobStore, ObjectRepository,
};
use crate::domain::entities::Object;
use crate::domain::value_objects::{Namespace, TenantId};

pub struct UploadObjectUseCase {
    object_repo: Arc<dyn ObjectRepository>,
    blob_repo: Arc<dyn BlobRepository>,
    blob_store: Arc<dyn BlobStore>,
}

impl UploadObjectUseCase {
    pub async fn execute(
        &self,
        request: UploadRequest,
        reader: BlobReader,
    ) -> Result<ObjectDto, UploadError> {
        // 1. Parse and validate request
        let namespace = Namespace::new(request.namespace)
            .map_err(|e| UploadError::InvalidRequest(e.to_string()))?;
        let tenant_id = TenantId::from_string(&request.tenant_id)
            .map_err(|e| UploadError::InvalidRequest(e.to_string()))?;
        let storage_class = request.storage_class.unwrap_or_default();

        // 2. Create domain entity in WRITING state
        let mut object = Object::new(namespace, tenant_id, request.key, storage_class);

        // 3. Reserve in DB (status=WRITING)
        self.object_repo.save(&object).await?;

        // 4. Write blob to storage (computes hash during write)
        let (content_hash, size_bytes) = self.blob_store.write(reader, storage_class).await?;

        // 5. Get or create blob entry with ref counting
        self.blob_repo
            .get_or_create(&content_hash, storage_class, size_bytes)
            .await?;

        // 6. Commit: update object state to COMMITTED
        object.commit(content_hash, size_bytes)?;
        self.object_repo.save(&object).await?;

        // 7. Return DTO
        Ok(ObjectDto::from(&object))
    }
}
```

### Router Setup (`api/router.rs`)

```rust
use axum::{
    middleware as axum_middleware,
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;

use crate::api::handlers::{
    delete_handler, download_handler, health_handler, list_handler,
    readiness_handler, upload_handler,
};
use crate::api::middleware::{auth, metrics};
use crate::application::use_cases::{
    DeleteObjectUseCase, DownloadObjectUseCase, ListObjectsUseCase, UploadObjectUseCase,
};

pub struct AppState {
    pub pool: Arc<sqlx::PgPool>,
    pub upload_use_case: Arc<UploadObjectUseCase>,
    pub download_use_case: Arc<DownloadObjectUseCase>,
    pub delete_use_case: Arc<DeleteObjectUseCase>,
    pub list_use_case: Arc<ListObjectsUseCase>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check (no auth required)
        .route("/health", get(health_handler))
        .route(
            "/health/ready",
            get(readiness_handler).with_state(Arc::clone(&state.pool)),
        )
        // Protected API routes
        .route("/v1/objects", post(upload_handler).with_state(Arc::clone(&state.upload_use_case)))
        .route("/v1/objects", get(list_handler).with_state(Arc::clone(&state.list_use_case)))
        .route(
            "/v1/objects/:id",
            get(download_handler).with_state(Arc::clone(&state.download_use_case)),
        )
        .route(
            "/v1/objects/:id",
            delete(delete_handler).with_state(Arc::clone(&state.delete_use_case)),
        )
        // Apply middleware layers (auth + metrics)
        .layer(axum_middleware::from_fn(auth::auth_middleware))
        .layer(axum_middleware::from_fn(metrics::metrics_middleware))
}
```

---

## Garbage Collection Worker

### `application/gc/worker.rs`

```rust
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

use crate::application::ports::{BlobRepository, BlobStore};

pub struct GarbageCollector {
    blob_repo: Arc<dyn BlobRepository>,
    blob_store: Arc<dyn BlobStore>,
    interval: Duration,
    batch_size: i64,
}

impl GarbageCollector {
    pub fn new(
        blob_repo: Arc<dyn BlobRepository>,
        blob_store: Arc<dyn BlobStore>,
        interval: Duration,
        batch_size: i64,
    ) -> Self {
        Self {
            blob_repo,
            blob_store,
            interval,
            batch_size,
        }
    }

    /// Run garbage collection loop
    pub async fn run(self: Arc<Self>) {
        info!("Starting garbage collector with interval: {:?}", self.interval);
        let mut interval = time::interval(self.interval);

        loop {
            interval.tick().await;

            match self.collect_once().await {
                Ok(count) => {
                    if count > 0 {
                        info!("Garbage collection completed: {} blobs deleted", count);
                    }
                }
                Err(e) => {
                    error!("Garbage collection failed: {}", e);
                }
            }
        }
    }

    /// Run one GC cycle
    async fn collect_once(&self) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        // Find orphaned blobs (ref_count = 0)
        let orphaned_blobs = self.blob_repo.find_orphaned(self.batch_size).await?;

        let count = orphaned_blobs.len();
        if count == 0 {
            return Ok(0);
        }

        info!("Found {} orphaned blobs to delete", count);

        for blob in orphaned_blobs {
            let content_hash = blob.content_hash();
            let storage_class = blob.storage_class();

            // Delete physical file
            match self.blob_store.delete(content_hash, storage_class).await {
                Ok(_) => {
                    info!("Deleted blob file: {}", content_hash);
                }
                Err(e) => {
                    warn!("Failed to delete blob file {}: {}", content_hash, e);
                    // Continue anyway - DB entry will be deleted
                }
            }

            // Delete DB entry
            if let Err(e) = self.blob_repo.delete(content_hash).await {
                error!("Failed to delete blob DB entry {}: {}", content_hash, e);
            }
        }

        Ok(count)
    }
}
```

**Usage in `main.rs`:**

```rust
// Start garbage collector in background
let gc = Arc::new(GarbageCollector::new(
    Arc::clone(&blob_repo),
    Arc::clone(&blob_store),
    Duration::from_secs(config.gc_interval_secs),
    config.gc_batch_size,
));
tokio::spawn(Arc::clone(&gc).run());
info!("Garbage collector started");
```

---

## Testing Patterns

### Integration Test: Upload → Download → Delete

```rust
use std::sync::Arc;
use just_storage::{
    application::{
        dto::UploadRequest,
        ports::{BlobRepository, BlobStore, ObjectRepository},
        use_cases::{DeleteObjectUseCase, DownloadObjectUseCase, UploadObjectUseCase},
    },
    domain::value_objects::StorageClass,
    infrastructure::{
        persistence::{PostgresBlobRepository, PostgresObjectRepository},
        storage::LocalFilesystemStore,
    },
};

#[tokio::test]
#[ignore] // Requires database and filesystem
async fn test_full_lifecycle() {
    // Setup
    let pool = sqlx::PgPool::connect("postgres://postgres:password@localhost/activestorage_test")
        .await
        .expect("Failed to connect to test database");

    let object_repo: Arc<dyn ObjectRepository> =
        Arc::new(PostgresObjectRepository::new(pool.clone()));
    let blob_repo: Arc<dyn BlobRepository> = Arc::new(PostgresBlobRepository::new(pool.clone()));

    let store = LocalFilesystemStore::new(
        std::path::PathBuf::from("/tmp/test_hot"),
        std::path::PathBuf::from("/tmp/test_cold"),
    );
    store.init().await.expect("Failed to init storage");
    let blob_store: Arc<dyn BlobStore> = Arc::new(store);

    // Create use cases
    let upload_use_case = Arc::new(UploadObjectUseCase::new(
        Arc::clone(&object_repo),
        Arc::clone(&blob_repo),
        Arc::clone(&blob_store),
    ));

    let download_use_case = Arc::new(DownloadObjectUseCase::new(
        Arc::clone(&object_repo),
        Arc::clone(&blob_store),
    ));

    let delete_use_case = Arc::new(DeleteObjectUseCase::new(
        Arc::clone(&object_repo),
        Arc::clone(&blob_repo),
        Arc::clone(&blob_store),
    ));

    // Upload
    let data = b"test data";
    let request = UploadRequest {
        namespace: "test".to_string(),
        tenant_id: "tenant1".to_string(),
        key: Some("my-object".to_string()),
        storage_class: Some(StorageClass::Hot),
    };

    let reader = Box::pin(std::io::Cursor::new(data));
    let object = upload_use_case.execute(request, reader).await.unwrap();
    assert_eq!(object.status, "COMMITTED");
    assert_eq!(object.size_bytes, Some(9));

    // Download
    let (mut reader, meta) = download_use_case.execute(&object.id).await.unwrap();
    let mut buf = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut buf).await.unwrap();
    assert_eq!(&buf, data);

    // Delete
    delete_use_case.execute(&object.id).await.unwrap();

    // Verify deleted
    let result = download_use_case.execute(&object.id).await;
    assert!(result.is_err());
}
```

### Unit Test with Mocks

```rust
use mockall::predicate::*;
use crate::application::use_cases::UploadObjectUseCase;
use crate::application::ports::{BlobRepository, BlobStore, ObjectRepository};

#[tokio::test]
async fn test_upload_use_case() {
    // Create mocks using mockall
    let mut object_repo = MockObjectRepository::new();
    let mut blob_repo = MockBlobRepository::new();
    let mut blob_store = MockBlobStore::new();

    // Setup expectations
    object_repo
        .expect_save()
        .times(2)
        .returning(|_| Ok(()));

    blob_store
        .expect_write()
        .once()
        .returning(|_, _| Ok((ContentHash::from_hex("abc123").unwrap(), 100)));

    blob_repo
        .expect_get_or_create()
        .once()
        .returning(|_, _, _| Ok(Blob::new(...)));

    // Execute
    let use_case = UploadObjectUseCase::new(
        Arc::new(object_repo),
        Arc::new(blob_repo),
        Arc::new(blob_store),
    );
    // ... test execution
}
```

---

## Deployment

### Docker

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app

# Copy manifests
COPY rust/Cargo.toml rust/Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && \
  echo "fn main() {}" > src/main.rs && \
  cargo build --release && \
  rm -rf src

# Copy source code
COPY rust/src ./src

# Build application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && \
  apt-get install -y ca-certificates && \
  rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/just_storage /app/just_storage

# Create data directories
RUN mkdir -p /data/hot /data/cold

EXPOSE 8080

CMD ["/app/just_storage"]
```

### Docker Compose

See `docker-compose.yml` for full configuration with PostgreSQL.

### Kubernetes

See `DESIGN.md` for full StatefulSet spec.

Key points:

- Use `StatefulSet` for stable identity
- Mount Longhorn PVCs for `/data/hot` and `/data/cold`
- Set resource limits (CPU/memory)
- Configure readiness/liveness probes on `/health` and `/health/ready`

---

## Monitoring

### Current Implementation

The current implementation uses structured logging via `tracing`:

```rust
// api/middleware/metrics.rs
pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    info!(
        method = %method,
        uri = %uri,
        status = %status.as_u16(),
        duration_ms = duration.as_millis(),
        "request_completed"
    );

    response
}
```

### Planned Prometheus Metrics

The following metrics are planned for future implementation:

```
activestorage_requests_total{method="PUT",status="200",namespace="models"}
activestorage_request_duration_seconds{method="GET",namespace="models"}
activestorage_objects_total{namespace="models",tenant="acme",status="COMMITTED"}
activestorage_storage_bytes{storage_class="hot",namespace="models"}
activestorage_gc_runs_total
activestorage_gc_deleted_blobs_total
```

### Health Checks

- `GET /health` — Liveness (is process alive?)
- `GET /health/ready` — Readiness (DB + filesystem mounted?)

---

## Implementation Status

### ✅ Completed

1. ✅ Core design documented
2. ✅ Database schema defined with migrations
3. ✅ Clean Architecture implemented (domain/application/infrastructure/api)
4. ✅ All API handlers implemented (upload, download, delete, list)
5. ✅ Use cases pattern implemented
6. ✅ GC worker implemented and tested
7. ✅ Authentication middleware (JWT + API keys)
8. ✅ Health check endpoints
9. ✅ Comprehensive error handling
10. ✅ Unit tests with mocks
11. ✅ Integration tests
12. ✅ Database validation CLI tool
13. ✅ Docker and docker-compose configurations
14. ✅ Structured logging with tracing

### ⏳ In Progress / Planned

1. ⏳ Prometheus metrics implementation
2. ⏳ Production deployment to dev cluster
3. ⏳ Performance benchmarks and optimization
4. ⏳ Monitoring dashboards
5. ⏳ Load testing
6. ⏳ Migration of first workload

---

## Additional Resources

- **Architecture**: See `docs/CLEAN_ARCHITECTURE.md` for detailed architecture documentation
- **API Reference**: See `docs/API.md` for complete API documentation
- **Design**: See `DESIGN.md` for system design and rationale
- **Operations**: See `docs/OPERATIONS.md` for operational procedures
- **Performance**: See `docs/PERFORMANCE.md` for performance characteristics
- **Quick Start**: See `docs/QUICKSTART.md` for getting started guide

---

## Key Design Principles

1. **Clean Architecture**: Clear separation of concerns with dependency inversion
2. **Single Responsibility**: Each module has one clear purpose
3. **Testability**: Easy to mock ports for unit testing
4. **Extensibility**: Add new storage backends without touching business logic
5. **Production Ready**: No unsafe unwrap/expect in production code paths
6. **Type Safety**: Strong typing with value objects and domain entities
7. **Error Handling**: Comprehensive error types with proper propagation
