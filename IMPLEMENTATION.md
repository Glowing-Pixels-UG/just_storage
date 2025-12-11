# ActiveStorage Implementation Guide

This guide provides concrete examples and patterns for implementing ActiveStorage in both Rust and Go.

---

## Project Structure

### Rust

```
rust/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, axum server setup
│   ├── config.rs            # Configuration loading
│   ├── api/
│   │   ├── mod.rs
│   │   ├── upload.rs        # POST /v1/objects
│   │   ├── download.rs      # GET /v1/objects/{id}
│   │   ├── delete.rs        # DELETE /v1/objects/{id}
│   │   └── list.rs          # GET /v1/objects
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── object_store.rs  # Trait definition ✅
│   │   ├── local_fs.rs      # LocalFsObjectStore impl ✅
│   │   └── integrity.rs     # Scrubber
│   ├── gc/
│   │   └── worker.rs        # Background GC
│   └── metrics.rs           # Prometheus metrics
```

### Go

```
go/
├── go.mod
├── cmd/
│   └── activestorage/
│       └── main.go          # Entry point
├── internal/
│   ├── config/
│   │   └── config.go        # Configuration
│   ├── api/
│   │   ├── handlers.go      # HTTP handlers
│   │   ├── upload.go
│   │   ├── download.go
│   │   ├── delete.go
│   │   └── list.go
│   ├── storage/
│   │   ├── object_store.go  # Interface ✅
│   │   ├── localfs.go       # LocalFsStore impl
│   │   └── integrity.go     # Scrubber
│   ├── gc/
│   │   └── worker.go        # Background GC
│   └── metrics/
│       └── metrics.go       # Prometheus metrics
```

---

## Configuration

### Rust: `config.rs`

```rust
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub hot_root: PathBuf,
    pub cold_root: PathBuf,
}

fn default_host() -> String { "0.0.0.0".to_string() }
fn default_port() -> u16 { 8080 }
fn default_max_connections() -> u32 { 20 }

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let cfg = config::Config::builder()
            .add_source(config::File::with_name("config"))
            .add_source(config::Environment::with_prefix("ACTIVESTORAGE"))
            .build()?;

        cfg.try_deserialize()
    }
}
```

### Go: `config/config.go`

```go
package config

import (
 "github.com/kelseyhightower/envconfig"
)

type Config struct {
 Server   ServerConfig
 Database DatabaseConfig
 Storage  StorageConfig
}

type ServerConfig struct {
 Host string `envconfig:"SERVER_HOST" default:"0.0.0.0"`
 Port int    `envconfig:"SERVER_PORT" default:"8080"`
}

type DatabaseConfig struct {
 URL            string `envconfig:"DATABASE_URL" required:"true"`
 MaxConnections int    `envconfig:"DATABASE_MAX_CONNECTIONS" default:"20"`
}

type StorageConfig struct {
 HotRoot  string `envconfig:"STORAGE_HOT_ROOT" default:"/data/hot"`
 ColdRoot string `envconfig:"STORAGE_COLD_ROOT" default:"/data/cold"`
}

func Load() (*Config, error) {
 var cfg Config
 if err := envconfig.Process("ACTIVESTORAGE", &cfg); err != nil {
  return nil, err
 }
 return &cfg, nil
}
```

---

## API Implementation Examples

### Rust: Upload Handler (`api/upload.rs`)

```rust
use axum::{
    extract::{Extension, Request},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde_json::json;
use crate::storage::{ObjectStore, PutRequest, StorageClass};

pub async fn upload_handler(
    Extension(store): Extension<Arc<dyn ObjectStore>>,
    headers: HeaderMap,
    body: Request,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    // Parse headers
    let namespace = headers
        .get("X-Namespace")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing X-Namespace header"})),
            )
        })?
        .to_string();

    let tenant_id = headers
        .get("X-Tenant")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing X-Tenant header"})),
            )
        })?
        .to_string();

    let key = headers
        .get("X-Key")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let storage_class = headers
        .get("X-Storage-Class")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| match s {
            "hot" => Some(StorageClass::Hot),
            "cold" => Some(StorageClass::Cold),
            _ => None,
        })
        .unwrap_or(StorageClass::Hot);

    let content_type = headers
        .get("Content-Type")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // Stream body
    let stream = body.into_body();
    let reader = StreamReader::new(stream);

    // Upload
    let request = PutRequest {
        namespace,
        tenant_id,
        key,
        storage_class,
        content_type,
    };

    match store.put(request, reader).await {
        Ok(meta) => Ok((StatusCode::CREATED, Json(meta))),
        Err(e) => Err((
            StatusCode::from_u16(e.status_code()).unwrap(),
            Json(json!({"error": e.to_string()})),
        )),
    }
}
```

### Go: Upload Handler (`api/upload.go`)

```go
package api

import (
 "encoding/json"
 "net/http"

 "github.com/google/uuid"
 "your-project/internal/storage"
)

type UploadHandler struct {
 store storage.ObjectStore
}

func NewUploadHandler(store storage.ObjectStore) *UploadHandler {
 return &UploadHandler{store: store}
}

func (h *UploadHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
 if r.Method != http.MethodPost {
  http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
  return
 }

 // Parse headers
 namespace := r.Header.Get("X-Namespace")
 if namespace == "" {
  http.Error(w, "Missing X-Namespace header", http.StatusBadRequest)
  return
 }

 tenantID := r.Header.Get("X-Tenant")
 if tenantID == "" {
  http.Error(w, "Missing X-Tenant header", http.StatusBadRequest)
  return
 }

 var key *string
 if k := r.Header.Get("X-Key"); k != "" {
  key = &k
 }

 storageClass := storage.StorageClassHot
 if sc := r.Header.Get("X-Storage-Class"); sc == "cold" {
  storageClass = storage.StorageClassCold
 }

 var contentType *string
 if ct := r.Header.Get("Content-Type"); ct != "" {
  contentType = &ct
 }

 // Upload
 req := storage.PutRequest{
  Namespace:    namespace,
  TenantID:     tenantID,
  Key:          key,
  StorageClass: storageClass,
  ContentType:  contentType,
 }

 meta, err := h.store.Put(r.Context(), req, r.Body)
 if err != nil {
  if storageErr, ok := err.(*storage.StorageError); ok {
   http.Error(w, storageErr.Error(), storageErr.StatusCode())
  } else {
   http.Error(w, err.Error(), http.StatusInternalServerError)
  }
  return
 }

 w.Header().Set("Content-Type", "application/json")
 w.WriteHeader(http.StatusCreated)
 json.NewEncoder(w).Encode(meta)
}
```

---

## Garbage Collection Worker

### Rust: `gc/worker.rs`

```rust
use sqlx::PgPool;
use std::path::Path;
use tokio::fs;
use tokio::time::{interval, Duration};
use tracing::{error, info};

pub struct GarbageCollector {
    db: PgPool,
    hot_root: String,
    cold_root: String,
}

impl GarbageCollector {
    pub fn new(db: PgPool, hot_root: String, cold_root: String) -> Self {
        Self { db, hot_root, cold_root }
    }

    pub async fn run(&self) {
        let mut tick = interval(Duration::from_secs(60)); // Run every minute

        loop {
            tick.tick().await;
            if let Err(e) = self.collect_garbage().await {
                error!("GC error: {}", e);
            }
        }
    }

    async fn collect_garbage(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Get blobs ready for deletion
        let blobs: Vec<(String, String)> = sqlx::query_as(
            "SELECT content_hash, storage_class FROM blobs
             WHERE gc_pending = true AND ref_count = 0
             LIMIT 100"
        )
        .fetch_all(&self.db)
        .await?;

        if blobs.is_empty() {
            return Ok(());
        }

        info!("GC: found {} blobs to delete", blobs.len());

        for (content_hash, storage_class) in blobs {
            let path = self.blob_path(&content_hash, &storage_class);

            // Delete file
            match fs::remove_file(&path).await {
                Ok(_) => info!("GC: deleted file {}", path),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // Already gone, fine
                }
                Err(e) => {
                    error!("GC: failed to delete {}: {}", path, e);
                    continue;
                }
            }

            // Mark blob as deleted in DB
            sqlx::query("SELECT mark_blob_deleted($1)")
                .bind(&content_hash)
                .execute(&self.db)
                .await?;
        }

        Ok(())
    }

    fn blob_path(&self, content_hash: &str, storage_class: &str) -> String {
        let root = if storage_class == "hot" {
            &self.hot_root
        } else {
            &self.cold_root
        };

        let hash = content_hash.strip_prefix("sha256:").unwrap_or(content_hash);
        let prefix = &hash[..2];

        format!("{}/sha256/{}/{}", root, prefix, hash)
    }
}
```

### Go: `gc/worker.go`

```go
package gc

import (
 "context"
 "fmt"
 "os"
 "time"

 "github.com/jackc/pgx/v5/pgxpool"
 "github.com/rs/zerolog/log"
)

type Worker struct {
 db       *pgxpool.Pool
 hotRoot  string
 coldRoot string
}

func NewWorker(db *pgxpool.Pool, hotRoot, coldRoot string) *Worker {
 return &Worker{
  db:       db,
  hotRoot:  hotRoot,
  coldRoot: coldRoot,
 }
}

func (w *Worker) Run(ctx context.Context) {
 ticker := time.NewTicker(60 * time.Second)
 defer ticker.Stop()

 for {
  select {
  case <-ticker.C:
   if err := w.collectGarbage(ctx); err != nil {
    log.Error().Err(err).Msg("GC error")
   }
  case <-ctx.Done():
   return
  }
 }
}

func (w *Worker) collectGarbage(ctx context.Context) error {
 // Get blobs ready for deletion
 rows, err := w.db.Query(ctx, `
  SELECT content_hash, storage_class
  FROM blobs
  WHERE gc_pending = true AND ref_count = 0
  LIMIT 100
 `)
 if err != nil {
  return err
 }
 defer rows.Close()

 var blobs []struct {
  ContentHash  string
  StorageClass string
 }

 for rows.Next() {
  var b struct {
   ContentHash  string
   StorageClass string
  }
  if err := rows.Scan(&b.ContentHash, &b.StorageClass); err != nil {
   return err
  }
  blobs = append(blobs, b)
 }

 if len(blobs) == 0 {
  return nil
 }

 log.Info().Int("count", len(blobs)).Msg("GC: found blobs to delete")

 for _, b := range blobs {
  path := w.blobPath(b.ContentHash, b.StorageClass)

  // Delete file
  if err := os.Remove(path); err != nil {
   if !os.IsNotExist(err) {
    log.Error().Err(err).Str("path", path).Msg("GC: failed to delete file")
    continue
   }
  } else {
   log.Info().Str("path", path).Msg("GC: deleted file")
  }

  // Mark blob as deleted
  _, err := w.db.Exec(ctx, "SELECT mark_blob_deleted($1)", b.ContentHash)
  if err != nil {
   log.Error().Err(err).Str("hash", b.ContentHash).Msg("GC: failed to mark deleted")
  }
 }

 return nil
}

func (w *Worker) blobPath(contentHash, storageClass string) string {
 root := w.hotRoot
 if storageClass == "cold" {
  root = w.coldRoot
 }

 hash := contentHash
 if len(contentHash) > 7 && contentHash[:7] == "sha256:" {
  hash = contentHash[7:]
 }

 prefix := hash[:2]
 return fmt.Sprintf("%s/sha256/%s/%s", root, prefix, hash)
}
```

---

## Testing Patterns

### Integration Test: Upload → Download → Delete

```rust
#[tokio::test]
async fn test_full_lifecycle() {
    let db = PgPool::connect(&test_database_url()).await.unwrap();
    let store = LocalFsObjectStore::new(db, "/tmp/test_hot", "/tmp/test_cold");

    // Upload
    let data = b"test data";
    let request = PutRequest {
        namespace: "test".to_string(),
        tenant_id: "tenant1".to_string(),
        key: Some("my-object".to_string()),
        storage_class: StorageClass::Hot,
        content_type: Some("text/plain".to_string()),
    };

    let meta = store.put(request, &data[..]).await.unwrap();
    assert_eq!(meta.status, ObjectStatus::Committed);
    assert_eq!(meta.size_bytes, Some(9));

    // Download
    let (mut reader, meta2) = store.get(meta.id).await.unwrap();
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await.unwrap();
    assert_eq!(&buf, data);

    // Delete
    store.delete(meta.id).await.unwrap();

    // Verify deleted
    let result = store.get(meta.id).await;
    assert!(matches!(result, Err(StorageError::NotFound(_))));
}
```

---

## Deployment

### Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/activestorage /usr/local/bin/
ENTRYPOINT ["/usr/local/bin/activestorage"]
```

### Kubernetes

See `DESIGN.md` for full StatefulSet spec.

Key points:

- Use `StatefulSet` for stable identity
- Mount Longhorn PVCs for `/data/hot` and `/data/cold`
- Set resource limits (CPU/memory)
- Configure readiness/liveness probes

---

## Monitoring

### Key Metrics

```
activestorage_requests_total{method="PUT",status="200",namespace="models"}
activestorage_request_duration_seconds{method="GET",namespace="models"}
activestorage_objects_total{namespace="models",tenant="acme",status="COMMITTED"}
activestorage_storage_bytes{storage_class="hot",namespace="models"}
activestorage_gc_runs_total
activestorage_gc_deleted_blobs_total
```

### Alerts

```yaml
groups:
- name: activestorage
  rules:
  - alert: HighErrorRate
    expr: rate(activestorage_requests_total{status=~"5.."}[5m]) > 0.05
    for: 5m
    annotations:
      summary: "High error rate in ActiveStorage"

  - alert: StorageNearFull
    expr: activestorage_storage_bytes / (1024^4) > 0.9 * <QUOTA>
    for: 10m
    annotations:
      summary: "Storage usage near quota"
```

---

## Next Steps

1. ✅ Core design documented
2. ✅ Database schema defined
3. ✅ ObjectStore trait/interface implemented
4. ⏳ Implement remaining API handlers
5. ⏳ Add GC worker
6. ⏳ Add metrics/observability
7. ⏳ Write integration tests
8. ⏳ Deploy to dev cluster
9. ⏳ Migrate first workload

---

## Additional Resources

- S3/GCS critique summary: `docs/S3_CRITIQUES.md` (to be created)
- ZFS tuning guide: See Longhorn + ZFS best practices
- Concurrency patterns: See state machine in `DESIGN.md`
