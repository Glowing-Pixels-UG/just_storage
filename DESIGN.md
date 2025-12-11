# ActiveStorage Design Document

## Overview

ActiveStorage is an internal object storage service designed to replace generic S3/GCS usage with a streamlined, domain-specific interface for Model Hub and related services.

**Key principles:**

- Simple, opinionated API tailored to our use cases
- Strong consistency guarantees
- Clear separation between what Longhorn handles vs. what we handle
- No S3/GCS anti-patterns

---

## Architecture Stack

### Storage Foundation (What We Don't Implement)

```
[ActiveStorage Service]
         ↓
[Longhorn Volumes] ← CSI, replication, snapshots, backups
         ↓
[ZFS Pools] ← disk redundancy (8+2), checksums, CoW
         ↓
[Physical Disks] ← NVMe (hot) / HDD (cold)
```

**Longhorn + ZFS handle:**

- ✅ Node failures & replica management
- ✅ Disk failures & bit rot detection
- ✅ Volume-level snapshots
- ✅ Volume backups to Hetzner Storage Box
- ✅ Block-level integrity
- ✅ Pod attachment/scheduling

**ActiveStorage handles:**

- ✅ Object semantics (tenants, namespaces, keys)
- ✅ Concurrency & write safety
- ✅ Metadata indexing & listing
- ✅ Object-level integrity (content hashing)
- ✅ Lifecycle management & GC
- ✅ API, auth, metrics

---

## Object State Machine

Every object transitions through explicit states. **Database is source of truth.**

```
┌─────────────┐
│   (none)    │
└──────┬──────┘
       │ API: POST /objects
       ↓
┌─────────────┐
│   WRITING   │ ← Temp file being written, txn #1 committed
└──────┬──────┘
       │ File written, fsync, rename, txn #2 commits
       ↓
┌─────────────┐
│  COMMITTED  │ ← Visible to readers
└──────┬──────┘
       │ API: DELETE /objects/{id}
       ↓
┌─────────────┐
│  DELETING   │ ← Metadata marked, ref_count decremented
└──────┬──────┘
       │ Background GC
       ↓
┌─────────────┐
│   DELETED   │ ← Metadata archived, file removed
└─────────────┘
```

### State Invariants

| State | DB Entry | File on Disk | Visible to Reads | GC Action |
|-------|----------|--------------|------------------|-----------|
| WRITING | exists, status=writing | temp file or partial | ❌ No | Timeout cleanup (orphan) |
| COMMITTED | exists, status=committed | full file, correct hash | ✅ Yes | None |
| DELETING | exists, status=deleting | still exists | ❌ No | Pending removal |
| DELETED | archived/purged | removed | ❌ No | Complete |

---

## Responsibility Boundaries

### What Longhorn Already Solves (Don't Re-Implement)

#### 1. **Replication & Node Failures**

- Longhorn maintains N replicas across nodes
- Automatic replica rebuilds on node loss
- **Your service:** Assume the filesystem is reliable, available, and replicated

#### 2. **Volume Management**

- CSI mounting, PVC lifecycle
- Volume attachment to pods
- **Your service:** Just use the mounted paths `/data/hot`, `/data/cold`

#### 3. **Volume Snapshots & Backups**

- Point-in-time snapshots (CoW)
- Backup to Hetzner Storage Box
- **Your service:** Ensure crash-consistent state; Longhorn handles the rest

#### 4. **Disk-Level Integrity**

- ZFS checksums, scrubs, vdev rebuilds
- **Your service:** Focus on object-level integrity (content hashes)

### What Your Service Must Handle

#### 1. **Object Semantics & Concurrency**

**Problem:** Longhorn sees only bytes; doesn't understand:

- Which file = which object
- Half-written vs. complete files
- Concurrent writers to same logical key
- Logical delete vs. physical file removal

**Solution:**

- Two-phase write protocol (temp → rename, DB state machine)
- DB-enforced concurrency (UNIQUE constraints, transactions)
- Status-based reads (only serve `status=committed`)

#### 2. **Naming, Indexing, Listing**

**Problem:** Longhorn has no concept of:

- Tenants, namespaces, tags
- Listing "all models for tenant X"
- Searching by metadata

**Solution:**

- Metadata in Postgres (indexed, queryable)
- Never list filesystem directly
- Content-addressable storage: files named by SHA256

#### 3. **Lifecycle & Garbage Collection**

**Problem:** Longhorn doesn't know:

- When to delete old versions
- Per-tenant quotas
- Orphaned files after crashes

**Solution:**

- Explicit versioning in metadata
- Reference counting for blobs
- Background GC worker: removes files only when `ref_count=0`

#### 4. **Object-Level Integrity**

**Problem:** ZFS checksums blocks, but doesn't validate:

- "Is this the model I uploaded?"
- "Did a bug corrupt this file?"

**Solution:**

- SHA-256 hash computed on upload
- Stored in DB + filename
- Optional scrubber: recompute hashes, detect mismatches

#### 5. **Access Control & Observability**

**Problem:** Longhorn has no concept of:

- Which tenant can access which object
- Usage stats (GB/tenant, ops/second)

**Solution:**

- Auth/authz at API layer
- Per-tenant metrics from DB
- Prometheus metrics: latency, throughput, errors

---

## Crash Safety & Concurrency Guarantees

### Write Protocol (Atomic, Crash-Safe)

```
1. START DB TXN
   INSERT INTO objects (id, namespace, tenant, key, status)
   VALUES (uuid, 'models', 'acme', 'gpt-4', 'WRITING')
   COMMIT

   → Crash here: DB shows WRITING, no file → GC cleanup

2. WRITE FILE
   stream body → /data/hot/tmp/upload-<uuid>
   compute SHA-256 while writing
   fsync(file)
   rename(/data/hot/tmp/upload-<uuid>, /data/hot/sha256/ab/abcdef...)
   fsync(dir)

   → Crash here: file exists, DB says WRITING → GC cleanup

3. START DB TXN
   UPDATE objects SET
     status = 'COMMITTED',
     content_hash = 'sha256:abcdef...',
     size_bytes = 12345
   WHERE id = uuid

   INSERT INTO blobs (content_hash, ref_count, storage_class)
   VALUES ('sha256:abcdef...', 1, 'hot')
   ON CONFLICT (content_hash) DO UPDATE SET ref_count = ref_count + 1

   COMMIT

   → Crash here: file exists, DB says COMMITTED → safe ✅
```

**Result:** Either the object is fully committed, or it's not visible.

### Read Protocol (Consistent)

```
1. SELECT id, content_hash, size_bytes, content_type
   FROM objects
   WHERE namespace = 'models'
     AND tenant = 'acme'
     AND key = 'gpt-4'
     AND status = 'COMMITTED'

   → Not found: 404
   → Found: proceed

2. Compute file path from content_hash:
   path = /data/hot/sha256/{first2chars}/{full_hash}

3. Open file, stream to response
   Set Content-Length, Content-Type

   → File missing: mark corrupted, 500 (should never happen)
```

**Result:** Strong read-after-write consistency; only committed objects visible.

### Delete Protocol (Deferred GC)

```
1. START DB TXN
   UPDATE objects SET status = 'DELETING'
   WHERE id = uuid AND status = 'COMMITTED'

   UPDATE blobs SET ref_count = ref_count - 1
   WHERE content_hash = (SELECT content_hash FROM objects WHERE id = uuid)

   -- If this blob is now unreferenced, mark for GC
   UPDATE blobs SET gc_pending = true
   WHERE content_hash = ... AND ref_count = 0

   COMMIT

   → File still on disk; readers with open FDs unaffected

2. BACKGROUND GC WORKER
   SELECT content_hash FROM blobs
   WHERE gc_pending = true AND ref_count = 0
   LIMIT 100

   For each:
     unlink(file)
     DELETE FROM blobs WHERE content_hash = ...
     UPDATE objects SET status = 'DELETED' WHERE content_hash = ...
```

**Result:** Deletes never race with reads; deduplication works correctly.

### Concurrent Writes to Same Key

**Two strategies:**

#### A. Last-Writer-Wins (Overwrite)

```sql
INSERT INTO objects (namespace, tenant, key, status, ...)
VALUES ('models', 'acme', 'gpt-4', 'WRITING', ...)
ON CONFLICT (namespace, tenant, key)
DO UPDATE SET
  status = 'WRITING',
  updated_at = now();

-- Old blob's ref_count gets decremented
-- New write proceeds
```

#### B. First-Wins (No Overwrite)

```sql
INSERT INTO objects (namespace, tenant, key, status, ...)
VALUES ('models', 'acme', 'gpt-4', 'WRITING', ...)
-- UNIQUE constraint violation → 409 Conflict
```

**Recommendation:** Start with First-Wins; add versioning later if needed.

---

## Avoiding S3/GCS Mistakes

### ❌ Mistakes We Won't Repeat

| S3/GCS Problem | Our Solution |
|----------------|--------------|
| **Fake filesystem semantics** | No "directories"; keys are opaque. Tree UI = DB view. |
| **Slow listing (scan millions)** | DB-backed listing with indexes; never scan filesystem. |
| **Small-object overhead** | Hard limits per namespace; optional packfiles later. |
| **Eventual consistency** | Strong consistency: read-after-write guaranteed. |
| **Mysterious throttling** | Explicit 429 + Retry-After; metrics show why. |
| **Versioning tombstone hell** | Explicit versions in DB; no hidden delete markers. |
| **Complex pricing** | Simple: $/GB stored, $/GB egress (if charged). |
| **Shallow abstraction** | Deep module: domain API (`put_model`), not generic buckets. |

### ✅ What We Do Instead

1. **No fake POSIX:** Keys are opaque strings; no `/` magic
2. **Listing = DB query:** Fast, bounded, indexed
3. **Strong consistency:** DB commits = instant visibility
4. **Transparent throttling:** Clear errors, visible metrics
5. **Explicit versioning:** Object + revision in metadata
6. **Simple cost model:** Usage dashboards per tenant/namespace
7. **Domain-specific API:** 10–15 well-defined operations max

---

## API Design

### Core Endpoints

#### `POST /v1/objects` — Upload

```bash
curl -X POST http://localhost:8080/v1/objects \
  -H "X-Namespace: models" \
  -H "X-Tenant: acme" \
  -H "X-Storage-Class: hot" \
  -H "X-Key: gpt-4-turbo" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @model.bin
```

**Response:**

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "namespace": "models",
  "tenant": "acme",
  "key": "gpt-4-turbo",
  "content_hash": "sha256:abcdef1234...",
  "size_bytes": 1073741824,
  "storage_class": "hot",
  "created_at": "2025-12-11T10:30:00Z"
}
```

#### `GET /v1/objects/{id}` — Download

```bash
curl http://localhost:8080/v1/objects/550e8400-e29b-41d4-a716-446655440000 \
  -H "X-Tenant: acme" \
  -o model.bin
```

**Headers:**

- `Content-Type`: original type
- `Content-Length`: size
- `X-Content-Hash`: SHA-256 for client verification

#### `GET /v1/objects/by-key/{namespace}/{tenant}/{key}` — Download by Key

```bash
curl http://localhost:8080/v1/objects/by-key/models/acme/gpt-4-turbo -o model.bin
```

#### `HEAD /v1/objects/{id}` — Metadata Only

Returns headers without body.

#### `DELETE /v1/objects/{id}` — Delete

Marks object as DELETING; actual file removal is async.

#### `GET /v1/objects` — List

```bash
curl "http://localhost:8080/v1/objects?namespace=models&tenant=acme&limit=50&cursor=abc123"
```

**Response:**

```json
{
  "objects": [
    {
      "id": "...",
      "namespace": "models",
      "tenant": "acme",
      "key": "gpt-4-turbo",
      "size_bytes": 1073741824,
      "created_at": "2025-12-11T10:30:00Z"
    }
  ],
  "cursor": "xyz789"
}
```

### Future Endpoints (v2)

- `POST /v1/objects/{id}/promote` — Move cold → hot
- `POST /v1/objects/{id}/archive` — Move hot → cold
- `GET /v1/namespaces/{namespace}/stats` — Usage per namespace
- `GET /v1/tenants/{tenant}/quota` — Quota & usage

---

## Implementation: Rust

### Stack

- **HTTP:** `axum`
- **DB:** `sqlx` (Postgres)
- **Async:** `tokio`
- **Hashing:** `sha2`
- **IDs:** `uuid`
- **Metrics:** `prometheus`
- **Logging:** `tracing`

### Key Modules

```
src/
├── main.rs              # Entry point, axum server
├── api/
│   ├── mod.rs
│   ├── upload.rs        # POST /v1/objects
│   ├── download.rs      # GET /v1/objects/{id}
│   ├── delete.rs        # DELETE /v1/objects/{id}
│   └── list.rs          # GET /v1/objects
├── storage/
│   ├── mod.rs
│   ├── object_store.rs  # ObjectStore trait
│   ├── local_fs.rs      # LocalFsObjectStore impl
│   └── integrity.rs     # Scrubber, hash verification
├── db/
│   ├── mod.rs
│   ├── schema.sql       # DDL
│   ├── objects.rs       # Object CRUD
│   └── blobs.rs         # Blob ref counting
├── gc/
│   └── worker.rs        # Background GC loop
└── config.rs            # Configuration
```

### `ObjectStore` Trait

```rust
#[async_trait]
pub trait ObjectStore: Send + Sync {
    async fn put(
        &self,
        namespace: &str,
        tenant: &str,
        key: Option<&str>,
        storage_class: StorageClass,
        reader: impl AsyncRead + Send + Unpin,
    ) -> Result<ObjectMeta>;

    async fn get(
        &self,
        id: Uuid,
    ) -> Result<(impl AsyncRead + Send + Unpin, ObjectMeta)>;

    async fn get_by_key(
        &self,
        namespace: &str,
        tenant: &str,
        key: &str,
    ) -> Result<(impl AsyncRead + Send + Unpin, ObjectMeta)>;

    async fn delete(&self, id: Uuid) -> Result<()>;

    async fn head(&self, id: Uuid) -> Result<ObjectMeta>;

    async fn list(
        &self,
        namespace: Option<&str>,
        tenant: &str,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<ListResult>;
}
```

---

## Implementation: Go

### Stack

- **HTTP:** `net/http` + `chi`
- **DB:** `pgx`
- **Hashing:** `crypto/sha256`
- **IDs:** `github.com/google/uuid`
- **Metrics:** `prometheus/client_golang`
- **Logging:** `uber-go/zap`

### Key Packages

```
cmd/
└── activestorage/
    └── main.go          # Entry point

internal/
├── api/
│   ├── handlers.go      # HTTP handlers
│   ├── upload.go
│   ├── download.go
│   ├── delete.go
│   └── list.go
├── storage/
│   ├── store.go         # ObjectStore interface
│   ├── localfs.go       # LocalFsStore impl
│   └── integrity.go
├── db/
│   ├── objects.go
│   └── blobs.go
├── gc/
│   └── worker.go
└── config/
    └── config.go
```

### `ObjectStore` Interface

```go
type ObjectStore interface {
    Put(ctx context.Context, req PutRequest, reader io.Reader) (*ObjectMeta, error)
    Get(ctx context.Context, id uuid.UUID) (io.ReadCloser, *ObjectMeta, error)
    GetByKey(ctx context.Context, namespace, tenant, key string) (io.ReadCloser, *ObjectMeta, error)
    Delete(ctx context.Context, id uuid.UUID) error
    Head(ctx context.Context, id uuid.UUID) (*ObjectMeta, error)
    List(ctx context.Context, req ListRequest) (*ListResult, error)
}

type PutRequest struct {
    Namespace    string
    Tenant       string
    Key          *string // optional
    StorageClass string  // "hot" | "cold"
}
```

---

## Filesystem Layout

```
/data/hot/                # Longhorn PVC → NVMe-backed
  tmp/                    # Temp uploads
    upload-<uuid>
  sha256/                 # Content-addressed blobs
    ab/
      abcdef123456...     # Actual file, named by full SHA-256

/data/cold/               # Longhorn PVC → HDD-backed
  tmp/
  sha256/
    9f/
      9fabcd...
```

**Why content-addressed?**

- Deduplication: Same blob stored once
- Integrity: Filename = hash → instant verification
- GC: Simple ref counting

---

## Database Schema

```sql
CREATE TABLE objects (
    id              UUID PRIMARY KEY,
    namespace       TEXT NOT NULL,
    tenant_id       TEXT NOT NULL,
    key             TEXT,
    status          TEXT NOT NULL CHECK (status IN ('WRITING', 'COMMITTED', 'DELETING', 'DELETED')),
    storage_class   TEXT NOT NULL CHECK (storage_class IN ('hot', 'cold')),
    content_hash    TEXT,
    size_bytes      BIGINT,
    content_type    TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_access_at  TIMESTAMPTZ,

    UNIQUE(namespace, tenant_id, key) WHERE key IS NOT NULL
);

CREATE INDEX idx_objects_status ON objects(status);
CREATE INDEX idx_objects_tenant_ns ON objects(tenant_id, namespace);

CREATE TABLE blobs (
    content_hash    TEXT PRIMARY KEY,
    storage_class   TEXT NOT NULL,
    ref_count       BIGINT NOT NULL DEFAULT 0,
    gc_pending      BOOLEAN NOT NULL DEFAULT false,
    first_seen_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at    TIMESTAMPTZ
);

CREATE INDEX idx_blobs_gc ON blobs(gc_pending, ref_count) WHERE gc_pending = true;
```

---

## Observability

### Metrics (Prometheus)

```
activestorage_requests_total{method, status, namespace}
activestorage_request_duration_seconds{method, namespace}
activestorage_objects_total{namespace, tenant, status}
activestorage_storage_bytes{storage_class, namespace}
activestorage_gc_runs_total
activestorage_gc_deleted_blobs_total
activestorage_integrity_errors_total
```

### Logs (Structured)

```json
{
  "level": "info",
  "msg": "object uploaded",
  "object_id": "550e8400-...",
  "tenant": "acme",
  "namespace": "models",
  "size_bytes": 1073741824,
  "duration_ms": 1234
}
```

### Health Checks

- `GET /health` — Liveness (is process alive?)
- `GET /ready` — Readiness (DB + filesystem mounted?)

---

## Deployment (Kubernetes)

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: activestorage
spec:
  serviceName: activestorage
  replicas: 1  # Can scale later
  selector:
    matchLabels:
      app: activestorage
  template:
    metadata:
      labels:
        app: activestorage
    spec:
      containers:
      - name: activestorage
        image: activestorage:latest
        ports:
        - containerPort: 8080
          name: http
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: activestorage-secrets
              key: database-url
        volumeMounts:
        - name: hot-storage
          mountPath: /data/hot
        - name: cold-storage
          mountPath: /data/cold
  volumeClaimTemplates:
  - metadata:
      name: hot-storage
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: longhorn-nvme
      resources:
        requests:
          storage: 1Ti
  - metadata:
      name: cold-storage
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: longhorn-standard
      resources:
        requests:
          storage: 10Ti
```

---

## Testing Strategy

### Unit Tests

- State transitions (WRITING → COMMITTED → DELETING → DELETED)
- Ref counting logic
- Path generation from hash

### Integration Tests

- Full upload/download cycle
- Concurrent writes to same key
- Delete while read in progress
- Crash simulation (stop before DB commit, after file write, etc.)

### Chaos Tests

- Kill pod mid-upload
- Corrupt a file, verify scrubber detects it
- Delete DB row, leave file → orphan GC
- High concurrency: 100 uploads to same key

---

## Migration Path

### Phase 1: Internal Replacement

1. Deploy ActiveStorage with Longhorn PVCs
2. Migrate Model Hub to use ActiveStorage API
3. Run both S3 + ActiveStorage in parallel (write-both, read-from-new)
4. Validate, then cut over fully

### Phase 2: Expand Use Cases

- Knowledge base documents
- User uploads
- Internal logs/artifacts

### Phase 3: Advanced Features

- Multi-version support
- Auto-tiering (hot → cold based on access patterns)
- Optional remote replication to real S3 for DR

---

## FAQ

**Q: Why not just use MinIO/SeaweedFS?**
A: We don't need full S3 semantics. A focused, domain-specific API is simpler, faster, and avoids S3's accumulated complexity.

**Q: Is one StatefulSet replica enough?**
A: For v1, yes. Longhorn replicates the volume. Later, scale to 2–3 replicas with shared PVCs or coordinated writes.

**Q: What if Longhorn fails?**
A: Longhorn + ZFS + Hetzner backups give you 3 layers. Catastrophic failure = restore from Hetzner. RPO = backup frequency.

**Q: How do I tune ZFS for this?**
A:

- NVMe pool: `recordsize=1M`, `compression=off` (models are pre-compressed)
- HDD pool: `recordsize=1M`, `compression=lz4`
- Monitor fragmentation over time

**Q: Can I run object storage (Seaweed/MinIO) on top of this?**
A: No. Those should run directly on ZFS (hostPath), not via Longhorn PVCs. Avoid stacking distributed systems.

---

## Summary

**What makes this safe:**

1. ✅ DB is source of truth (explicit states)
2. ✅ Two-phase writes (temp → rename, atomic)
3. ✅ Content hashing for integrity
4. ✅ Deferred GC (no read/delete races)
5. ✅ Longhorn handles replication/failures underneath

**What makes this fast:**

1. ✅ Local NVMe for hot storage
2. ✅ Streaming (no RAM buffering)
3. ✅ DB-backed listing (no filesystem scans)
4. ✅ Content deduplication

**What makes this maintainable:**

1. ✅ Simple, domain-specific API (not fake S3)
2. ✅ Clear responsibility boundaries (Longhorn vs. us)
3. ✅ Metrics + logs for observability
4. ✅ Explicit lifecycle (no hidden tombstones)

**Next Steps:**

1. Review this design
2. Implement ObjectStore trait/interface
3. Write DB migrations
4. Implement upload/download handlers
5. Add GC worker
6. Deploy to dev cluster
7. Migrate first workload (Model Hub)
