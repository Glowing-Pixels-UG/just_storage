# Architecture Overview

Comprehensive architecture documentation for JustStorage.

## System Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                         Clients                             │
│  (Model Hub, Knowledge Base, File Service, etc.)            │
└────────────────┬────────────────────────────────────────────┘
                 │ HTTP/REST
                 ▼
┌─────────────────────────────────────────────────────────────┐
│                    API Layer (Axum)                         │
│  ┌──────────┐  ┌─────────┐  ┌──────────┐  ┌─────────────┐ │
│  │ Auth     │  │ Metrics │  │ Error    │  │ Handlers    │ │
│  │ Middleware│  │Middleware│  │ Mapping  │  │ (CRUD+List) │ │
│  └──────────┘  └─────────┘  └──────────┘  └─────────────┘ │
└────────────────┬────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────────┐
│               Application Layer (Use Cases)                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ Upload   │  │ Download │  │ Delete   │  │ List     │  │
│  │ UseCase  │  │ UseCase  │  │ UseCase  │  │ UseCase  │  │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │
│  ┌─────────────────────────────────────────────────────┐  │
│  │         Garbage Collector (Background Worker)       │  │
│  └─────────────────────────────────────────────────────┘  │
└────────────────┬──────────────┬─────────────────────────────┘
                 │              │
                 ▼              ▼
┌─────────────────────────┐  ┌──────────────────────────────┐
│   Domain Layer          │  │   Infrastructure Layer       │
│  ┌──────────────────┐   │  │  ┌────────────────────────┐ │
│  │ Entities         │   │  │  │ Persistence            │ │
│  │ - Object         │   │  │  │ - PostgresObjectRepo   │ │
│  │ - Blob           │   │  │  │ - PostgresBlobRepo     │ │
│  └──────────────────┘   │  │  └────────────────────────┘ │
│  ┌──────────────────┐   │  │  ┌────────────────────────┐ │
│  │ Value Objects    │   │  │  │ Storage                │ │
│  │ - ObjectId       │   │  │  │ - LocalFilesystemStore │ │
│  │ - ContentHash    │   │  │  │ - ContentHasher        │ │
│  │ - Namespace      │   │  │  │ - PathBuilder          │ │
│  │ - TenantId       │   │  │  └────────────────────────┘ │
│  │ - StorageClass   │   │  │                              │
│  └──────────────────┘   │  └──────────┬───────────────────┘
└─────────────────────────┘             │
                                        ▼
                          ┌──────────────────────────────────┐
                          │  External Systems                │
                          │  ┌─────────────┐  ┌────────────┐ │
                          │  │ PostgreSQL  │  │ Filesystem │ │
                          │  │ (Metadata)  │  │ (Blobs)    │ │
                          │  └─────────────┘  └────────────┘ │
                          └──────────────────────────────────┘
```

---

## Clean Architecture Layers

### 1. Domain Layer (Core)

**Purpose**: Business logic and rules, independent of external systems.

**Components**:

- **Entities**:
  - `Object` - Logical object with state machine
  - `Blob` - Physical file with reference counting

- **Value Objects**:
  - `ObjectId` (UUID)
  - `ContentHash` (SHA-256)
  - `Namespace`, `TenantId`, `StorageClass`
  - `ObjectStatus` (WRITING → COMMITTED → DELETING → DELETED)
  - `ObjectMetadata` (extensible JSON)

- **Domain Errors**:
  - `DomainError` - Business rule violations

**Key Principle**: Zero dependencies on other layers.

### 2. Application Layer (Use Cases)

**Purpose**: Orchestrate domain entities to fulfill business use cases.

**Components**:

- **Ports (Traits)**:
  - `ObjectRepository` - CRUD for objects
  - `BlobRepository` - Reference counting
  - `BlobStore` - Physical storage operations

- **Use Cases**:
  - `UploadObjectUseCase` - Two-phase commit upload
  - `DownloadObjectUseCase` - Stream blob to client
  - `DeleteObjectUseCase` - Mark deleted, decrement refs
  - `ListObjectsUseCase` - Paginated listing

- **Background Workers**:
  - `GarbageCollector` - Delete orphaned blobs

**Key Principle**: Depends on domain, defines interfaces for infrastructure.

### 3. Infrastructure Layer (Adapters)

**Purpose**: Implement ports with concrete technologies.

**Components**:

- **Persistence**:
  - `PostgresObjectRepository` - SQLx-based object storage
  - `PostgresBlobRepository` - SQLx-based blob tracking

- **Storage**:
  - `LocalFilesystemStore` - Content-addressable file storage
  - `ContentHasher` - Streaming SHA-256 computation
  - `PathBuilder` - Generate storage paths from hashes

**Key Principle**: Depends on application (ports) and domain.

### 4. API Layer (Delivery)

**Purpose**: HTTP interface to use cases.

**Components**:

- **Handlers**:
  - `upload_handler` - POST /v1/objects
  - `download_handler` - GET /v1/objects/{id}
  - `delete_handler` - DELETE /v1/objects/{id}
  - `list_handler` - GET /v1/objects

- **Middleware**:
  - `auth_middleware` - JWT/API key validation
  - `metrics_middleware` - Request logging

- **Error Mapping**:
  - Use case errors → HTTP status codes

**Key Principle**: Thin layer, delegates to use cases.

---

## Data Flow

### Upload Flow

```text
1. Client → POST /v1/objects
   ├─ Query params: namespace, tenant_id, key, storage_class
   └─ Body: binary stream

2. API Layer
   ├─ auth_middleware validates JWT/API key
   ├─ upload_handler creates UploadRequest
   └─ calls UploadObjectUseCase.execute()

3. Use Case (Two-Phase Commit)
   ├─ Phase 1: Create object in WRITING state
   │   └─ object_repo.create() → DB insert
   ├─ Phase 2: Write blob, commit
   │   ├─ blob_store.write() → compute hash, fsync file
   │   ├─ blob_repo.get_or_create() → increment ref count
   │   └─ object_repo.commit() → update to COMMITTED
   └─ Return ObjectDto

4. API Layer
   └─ Return 201 Created with JSON body
```

**Crash Safety**: If crash occurs before step 3.2.3, object stays in WRITING state and is invisible to clients. GC will clean up eventually.

### Download Flow

```text
1. Client → GET /v1/objects/{id}?tenant_id=X

2. API Layer
   ├─ auth_middleware validates
   ├─ download_handler parses ID
   └─ calls DownloadObjectUseCase.execute()

3. Use Case
   ├─ object_repo.find_by_id() → get object from DB
   ├─ verify tenant_id matches
   ├─ verify status == COMMITTED
   └─ blob_store.read() → open file stream

4. API Layer
   └─ Return 200 OK with streaming body
```

### Delete Flow

```text
1. Client → DELETE /v1/objects/{id}?tenant_id=X

2. API Layer
   ├─ auth_middleware validates
   ├─ delete_handler parses ID
   └─ calls DeleteObjectUseCase.execute()

3. Use Case
   ├─ object_repo.find_by_id() → get object
   ├─ verify tenant_id matches
   ├─ object_repo.mark_deleting() → set status=DELETING
   └─ blob_repo.decrement_ref() → ref_count--

4. API Layer
   └─ Return 204 No Content

5. Background GC (async)
   ├─ Find blobs where ref_count = 0
   ├─ blob_store.delete() → remove file
   └─ blob_repo.delete() → remove DB entry
```

---

## State Machine

Every object transitions through explicit states:

```text
┌──────────┐
│  (none)  │
└────┬─────┘
     │ POST /v1/objects
     ▼
┌──────────┐
│ WRITING  │  ← Temp file written, DB txn #1
└────┬─────┘    Invisible to reads
     │ File written, fsync, rename
     │ DB txn #2 updates status
     ▼
┌───────────┐
│ COMMITTED │  ← Visible to reads
└────┬──────┘    Content immutable
     │ DELETE /v1/objects
     │ Mark deleted, ref_count--
     ▼
┌───────────┐
│ DELETING  │  ← No longer visible
└────┬──────┘    Pending GC
     │ Background GC
     │ Physical file deleted
     ▼
┌──────────┐
│ DELETED  │  ← File removed (tombstone)
└──────────┘
```

**Properties**:

- **Atomicity**: State transitions are transactional
- **Visibility**: Only COMMITTED objects are visible
- **Crash Safety**: Incomplete uploads remain in WRITING
- **Async Deletion**: DELETING → DELETED happens in background

---

## Content-Addressable Storage

### Addressing Scheme

Files are stored by content hash:

```text
Storage Path = {storage_class}/{hash[0:2]}/{hash[2:4]}/{hash}

Example:
  Content Hash: sha256:a3c5f1e2b4d67890...
  Storage Path: hot/a3/c5/a3c5f1e2b4d67890...
```

### Deduplication

```text
Object A ──┐
           ├──→ Blob (hash=X, ref_count=2)
Object B ──┘

Object A deleted: ref_count = 1 (file remains)
Object B deleted: ref_count = 0 (GC deletes file)
```

### Reference Counting

```sql
-- Upload increments
UPDATE blobs SET ref_count = ref_count + 1
  WHERE content_hash = $1;

-- Delete decrements
UPDATE blobs SET ref_count = ref_count - 1
  WHERE content_hash = $1;

-- GC finds orphans
SELECT content_hash FROM blobs WHERE ref_count = 0;
```

---

## Database Schema

### Objects Table

Stores logical object metadata.

```sql
CREATE TABLE objects (
  id              UUID PRIMARY KEY,
  namespace       TEXT NOT NULL,
  tenant_id       TEXT NOT NULL,
  key             TEXT,  -- Optional, for by-key access
  status          TEXT NOT NULL,  -- WRITING|COMMITTED|DELETING|DELETED
  storage_class   TEXT NOT NULL,  -- hot|cold
  content_hash    TEXT,  -- Filled when COMMITTED
  size_bytes      BIGINT,
  content_type    TEXT,
  metadata        JSONB,
  created_at      TIMESTAMPTZ NOT NULL,
  updated_at      TIMESTAMPTZ NOT NULL,
  UNIQUE (namespace, tenant_id, key) WHERE key IS NOT NULL
);
```

**Indexes**:

- `idx_objects_status` - Filter by status
- `idx_objects_tenant_ns` - List by tenant/namespace
- `idx_objects_content_hash` - Join with blobs

### Blobs Table

Tracks physical files with reference counting.

```sql
CREATE TABLE blobs (
  content_hash    TEXT PRIMARY KEY,
  storage_class   TEXT NOT NULL,
  ref_count       BIGINT NOT NULL DEFAULT 0,
  first_seen_at   TIMESTAMPTZ NOT NULL,
  last_used_at    TIMESTAMPTZ,
  CHECK (ref_count >= 0)
);
```

**Indexes**:

- `idx_blobs_gc` - Find orphans for GC

---

## Concurrency Control

### Upload Conflicts

```sql
-- Unique key constraint prevents duplicates
CONSTRAINT unique_key_per_tenant_ns
  UNIQUE (namespace, tenant_id, key)
  WHERE key IS NOT NULL AND status != 'DELETED'
```

**Behavior**: Second upload with same key returns 409 Conflict.

### Concurrent Deletes

```sql
-- Reference counting is atomic
UPDATE blobs SET ref_count = ref_count - 1
  WHERE content_hash = $1;
```

**Safety**: GC only deletes when `ref_count = 0`.

### GC Race Conditions

```text
Scenario: Object deleted while GC is running

1. Object A deleted → ref_count = 0
2. GC finds blob (ref_count = 0)
3. New object B uploaded (same hash) → ref_count = 1
4. GC tries to delete file → SHOULD FAIL

Solution: Check ref_count again before delete:

SELECT ref_count FROM blobs WHERE content_hash = $1;
IF ref_count = 0 THEN
  DELETE file
ELSE
  Skip (someone added reference)
END IF
```

---

## Error Handling

### Error Types by Layer

**Domain Errors**:

- Business rule violations
- Invalid state transitions
- Validation failures

**Application Errors**:

- `UploadError`, `DownloadError`, `DeleteError`, `ListError`
- Wrap repository/storage errors

**Repository Errors**:

- Database failures
- Not found
- Serialization errors

**Storage Errors**:

- I/O failures
- Hash mismatches
- Insufficient space

### Error Propagation

```text
Storage Error
    ↓ (wrapped)
Repository Error
    ↓ (mapped)
Use Case Error
    ↓ (mapped)
HTTP Status Code (API layer)
```

### Example Mapping

```rust
match use_case_error {
    UploadError::Conflict => 409 Conflict,
    UploadError::InvalidRequest => 400 Bad Request,
    UploadError::StorageFull => 507 Insufficient Storage,
    UploadError::Internal => 500 Internal Server Error,
}
```

---

## Scalability

### Current Limitations

- Single-instance deployment (no horizontal scaling)
- Local filesystem storage (bound to single node)
- PostgreSQL as bottleneck for metadata

### Future Scaling Options

**Horizontal Scaling**:

- Read replicas for download traffic
- Shard by tenant_id or namespace
- Distributed filesystem (Ceph, MinIO)

**Vertical Scaling**:

- Increase PostgreSQL resources
- Add read replicas
- Faster storage (NVMe)

**Caching**:

- CDN for frequently accessed objects
- Redis for metadata caching

---

## Security

### Authentication

- **JWT Tokens**: For user-facing applications
- **API Keys**: For service-to-service communication

### Authorization

- **Tenant Isolation**: All operations scoped to tenant_id
- **Namespace Separation**: Objects grouped by namespace

### Data Integrity

- **Content Hashing**: SHA-256 verification
- **Atomic Operations**: Two-phase commit
- **Fsync**: Durable writes

### Network Security

- TLS termination at load balancer
- Internal-only service access
- Database connection encryption

---

## Observability

### Structured Logging

All logs are JSON with:

- `timestamp`
- `level` (INFO, WARN, ERROR)
- `target` (module path)
- `fields` (operation details)

### Key Metrics

- Request count by method/status
- Request duration (p50, p95, p99)
- Storage usage by class
- GC runs and deleted blobs
- Error rates

### Health Checks

- `/health` - Liveness
- Database connectivity
- Filesystem accessibility

---

## Deployment Architecture

### Single-Instance (Current)

```text
┌─────────────────────────────────┐
│      Kubernetes Pod             │
│  ┌───────────────────────────┐  │
│  │   JustStorage Container   │  │
│  │   - HTTP Server (8080)    │  │
│  │   - GC Worker             │  │
│  └───────────────────────────┘  │
│  ┌───────────────────────────┐  │
│  │   Hot Storage (PVC)       │  │
│  │   - NVMe-backed           │  │
│  └───────────────────────────┘  │
│  ┌───────────────────────────┐  │
│  │   Cold Storage (PVC)      │  │
│  │   - HDD-backed            │  │
│  └───────────────────────────┘  │
└─────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────┐
│   PostgreSQL (External)         │
└─────────────────────────────────┘
```

### High-Availability (Future)

```text
┌──────────────┐  ┌──────────────┐
│ Instance 1   │  │ Instance 2   │
│ (Active)     │  │ (Standby)    │
└──────┬───────┘  └──────┬───────┘
       │                 │
       └────────┬────────┘
                ▼
      ┌──────────────────┐
      │ Shared Storage   │
      │ (Ceph/MinIO)     │
      └──────────────────┘
```

---

## See Also

- [CLEAN_ARCHITECTURE.md](CLEAN_ARCHITECTURE.md) - Detailed layer breakdown
- [DESIGN.md](../DESIGN.md) - Design decisions
- [DATABASE.md](DATABASE.md) - Database schema details
- [DEPLOYMENT.md](DEPLOYMENT.md) - Deployment guide
