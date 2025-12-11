# Clean Architecture for ActiveStorage

## Current Problem

The existing design violates SRP by putting too much into `LocalFsObjectStore`:

- ❌ File I/O operations
- ❌ Database transactions
- ❌ Business logic (state transitions)
- ❌ Hash computation
- ❌ Path generation

This makes it hard to test, maintain, and extend.

---

## Proper Architecture (Clean Architecture / Hexagonal)

```
┌────────────────────────────────────────────────────────────┐
│                      API Layer (Adapters)                   │
│  • axum handlers                                            │
│  • HTTP request/response mapping                            │
│  • Auth extraction                                          │
│  • Error → HTTP status                                      │
└────────────────┬───────────────────────────────────────────┘
                 │
┌────────────────▼───────────────────────────────────────────┐
│                   Application Layer (Use Cases)             │
│  • UploadObjectUseCase                                      │
│  • DownloadObjectUseCase                                    │
│  • DeleteObjectUseCase                                      │
│  • ListObjectsUseCase                                       │
│  • GarbageCollectionUseCase                                 │
└────────────────┬───────────────────────────────────────────┘
                 │
┌────────────────▼───────────────────────────────────────────┐
│                      Domain Layer (Core)                    │
│  • Object (entity)                                          │
│  • Blob (entity)                                            │
│  • ObjectStatus (value object)                              │
│  • StorageClass (value object)                              │
│  • ContentHash (value object)                               │
│  • Domain events                                            │
└────────────────┬───────────────────────────────────────────┘
                 │
┌────────────────▼───────────────────────────────────────────┐
│               Infrastructure Layer (Adapters)               │
│  • PostgresObjectRepository (port impl)                     │
│  • LocalFilesystemBlobStore (port impl)                     │
│  • S3BlobStore (future port impl)                           │
│  • ContentHasher (utility)                                  │
│  • PathBuilder (utility)                                    │
└─────────────────────────────────────────────────────────────┘
```

---

## Module Structure

```
src/
├── main.rs                          # Entry point, DI container
│
├── domain/                          # Core business logic (no dependencies)
│   ├── mod.rs
│   ├── entities/
│   │   ├── mod.rs
│   │   ├── object.rs                # Object aggregate
│   │   └── blob.rs                  # Blob entity
│   ├── value_objects/
│   │   ├── mod.rs
│   │   ├── object_status.rs         # WRITING, COMMITTED, etc.
│   │   ├── storage_class.rs         # Hot, Cold
│   │   ├── content_hash.rs          # SHA-256 wrapper
│   │   ├── namespace.rs             # Validated namespace
│   │   └── tenant_id.rs             # Validated tenant
│   ├── events/
│   │   ├── mod.rs
│   │   ├── object_uploaded.rs
│   │   └── object_deleted.rs
│   └── errors.rs                    # Domain errors
│
├── application/                     # Use cases (orchestration)
│   ├── mod.rs
│   ├── ports/                       # Interfaces (traits)
│   │   ├── mod.rs
│   │   ├── object_repository.rs     # Trait: save, find, delete
│   │   ├── blob_store.rs            # Trait: write, read, remove
│   │   └── event_publisher.rs       # Trait: publish events
│   ├── use_cases/
│   │   ├── mod.rs
│   │   ├── upload_object.rs         # UploadObjectUseCase
│   │   ├── download_object.rs       # DownloadObjectUseCase
│   │   ├── delete_object.rs         # DeleteObjectUseCase
│   │   ├── list_objects.rs          # ListObjectsUseCase
│   │   └── garbage_collection.rs    # GarbageCollectionUseCase
│   └── dto.rs                       # Data transfer objects
│
├── infrastructure/                  # External implementations
│   ├── mod.rs
│   ├── persistence/
│   │   ├── mod.rs
│   │   ├── postgres_object_repository.rs  # ObjectRepository impl
│   │   └── postgres_blob_repository.rs    # Blob ref counting
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── local_filesystem_store.rs      # BlobStore impl
│   │   ├── content_hasher.rs              # SHA-256 utility
│   │   └── path_builder.rs                # Path generation
│   └── events/
│       ├── mod.rs
│       └── noop_event_publisher.rs        # Stub for now
│
└── api/                             # HTTP adapters
    ├── mod.rs
    ├── handlers/
    │   ├── mod.rs
    │   ├── upload.rs                # POST /v1/objects
    │   ├── download.rs              # GET /v1/objects/{id}
    │   ├── delete.rs                # DELETE /v1/objects/{id}
    │   └── list.rs                  # GET /v1/objects
    ├── middleware/
    │   ├── mod.rs
    │   ├── auth.rs                  # Auth extraction
    │   └── metrics.rs               # Prometheus metrics
    └── errors.rs                    # HTTP error responses
```

---

## Key Principles

### 1. Dependency Inversion

**Domain and Application layers depend on NO infrastructure.**

```rust
// ❌ BAD - Application depends on concrete implementation
use crate::infrastructure::postgres_object_repository::PostgresRepository;

struct UploadObjectUseCase {
    repo: PostgresRepository,  // Concrete type!
}

// ✅ GOOD - Application depends on abstraction
use crate::application::ports::ObjectRepository;

struct UploadObjectUseCase {
    repo: Box<dyn ObjectRepository>,  // Trait!
}
```

### 2. Single Responsibility

Each module has ONE reason to change:

- **Domain entities** - Business rules change
- **Use cases** - Workflow changes
- **Repositories** - Database schema changes
- **Blob store** - Storage backend changes
- **API handlers** - HTTP API changes

### 3. Interface Segregation

Don't force implementations to depend on methods they don't use:

```rust
// ❌ BAD - One huge interface
trait Storage {
    fn write_blob(...);
    fn read_blob(...);
    fn delete_blob(...);
    fn backup_to_s3(...);      // Not all impls need this
    fn run_garbage_collection(...);  // Different concern
}

// ✅ GOOD - Separate interfaces
trait BlobStore {
    fn write_blob(...);
    fn read_blob(...);
    fn delete_blob(...);
}

trait BlobBackup {
    fn backup_to_s3(...);
}

trait GarbageCollector {
    fn collect(...);
}
```

### 4. Open/Closed

Open for extension, closed for modification:

```rust
// Adding S3 backend doesn't require changing existing code
impl BlobStore for S3BlobStore { ... }
impl BlobStore for LocalFilesystemStore { ... }
impl BlobStore for SeaweedFSStore { ... }

// Use case stays the same
struct UploadObjectUseCase {
    blob_store: Box<dyn BlobStore>,  // Any implementation!
}
```

---

## Example: Upload Flow (Proper Separation)

### 1. API Layer (Entry Point)

```rust
// api/handlers/upload.rs
pub async fn upload_handler(
    State(use_case): State<Arc<UploadObjectUseCase>>,
    headers: HeaderMap,
    body: Body,
) -> Result<Json<ObjectDto>, ApiError> {
    let request = UploadRequest::from_headers(headers)?;
    let stream = body.into_data_stream();

    let object = use_case.execute(request, stream).await?;

    Ok(Json(ObjectDto::from(object)))
}
```

**Responsibility:** HTTP → domain mapping

---

### 2. Use Case Layer (Orchestration)

```rust
// application/use_cases/upload_object.rs
pub struct UploadObjectUseCase {
    object_repo: Arc<dyn ObjectRepository>,
    blob_store: Arc<dyn BlobStore>,
    event_publisher: Arc<dyn EventPublisher>,
}

impl UploadObjectUseCase {
    pub async fn execute(
        &self,
        request: UploadRequest,
        reader: impl AsyncRead,
    ) -> Result<Object, ApplicationError> {
        // 1. Create domain entity in WRITING state
        let mut object = Object::new(
            request.namespace,
            request.tenant_id,
            request.key,
            request.storage_class,
        );

        // 2. Reserve in DB (status=WRITING)
        self.object_repo.save(&object).await?;

        // 3. Write blob to storage
        let (content_hash, size) = self.blob_store
            .write(reader, object.storage_class())
            .await?;

        // 4. Commit: update object state
        object.commit(content_hash, size)?;  // Domain method!
        self.object_repo.save(&object).await?;

        // 5. Publish event
        self.event_publisher
            .publish(ObjectUploadedEvent::from(&object))
            .await?;

        Ok(object)
    }
}
```

**Responsibility:** Orchestrate the workflow, enforce business rules

---

### 3. Domain Layer (Business Logic)

```rust
// domain/entities/object.rs
pub struct Object {
    id: ObjectId,
    namespace: Namespace,
    tenant_id: TenantId,
    key: Option<String>,
    status: ObjectStatus,
    storage_class: StorageClass,
    content_hash: Option<ContentHash>,
    size_bytes: Option<u64>,
    created_at: DateTime<Utc>,
}

impl Object {
    pub fn new(
        namespace: Namespace,
        tenant_id: TenantId,
        key: Option<String>,
        storage_class: StorageClass,
    ) -> Self {
        Self {
            id: ObjectId::new(),
            namespace,
            tenant_id,
            key,
            status: ObjectStatus::Writing,  // Initial state
            storage_class,
            content_hash: None,
            size_bytes: None,
            created_at: Utc::now(),
        }
    }

    pub fn commit(
        &mut self,
        content_hash: ContentHash,
        size_bytes: u64,
    ) -> Result<(), DomainError> {
        // Business rule: can only commit if writing
        if self.status != ObjectStatus::Writing {
            return Err(DomainError::InvalidStateTransition {
                from: self.status,
                to: ObjectStatus::Committed,
            });
        }

        self.status = ObjectStatus::Committed;
        self.content_hash = Some(content_hash);
        self.size_bytes = Some(size_bytes);

        Ok(())
    }

    pub fn mark_for_deletion(&mut self) -> Result<(), DomainError> {
        if self.status != ObjectStatus::Committed {
            return Err(DomainError::CannotDeleteNonCommitted);
        }

        self.status = ObjectStatus::Deleting;
        Ok(())
    }
}
```

**Responsibility:** Encapsulate business rules, validate state transitions

---

### 4. Infrastructure Layer (Implementation Details)

```rust
// infrastructure/storage/local_filesystem_store.rs
pub struct LocalFilesystemStore {
    hot_root: PathBuf,
    cold_root: PathBuf,
    hasher: Arc<ContentHasher>,
    path_builder: Arc<PathBuilder>,
}

#[async_trait]
impl BlobStore for LocalFilesystemStore {
    async fn write(
        &self,
        mut reader: impl AsyncRead + Send + Unpin,
        storage_class: StorageClass,
    ) -> Result<(ContentHash, u64), StorageError> {
        // 1. Get temp path
        let temp_id = Uuid::new_v4();
        let temp_path = self.path_builder.temp_path(storage_class, temp_id);

        // 2. Write to temp + compute hash
        let (hash, size) = self.hasher
            .write_and_hash(&temp_path, reader)
            .await?;

        // 3. Move to final path (atomic)
        let final_path = self.path_builder.final_path(storage_class, &hash);
        fs::rename(&temp_path, &final_path).await?;

        Ok((hash, size))
    }

    async fn read(
        &self,
        content_hash: &ContentHash,
        storage_class: StorageClass,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, StorageError> {
        let path = self.path_builder.final_path(storage_class, content_hash);
        let file = File::open(path).await?;
        Ok(Box::new(BufReader::new(file)))
    }

    async fn delete(
        &self,
        content_hash: &ContentHash,
        storage_class: StorageClass,
    ) -> Result<(), StorageError> {
        let path = self.path_builder.final_path(storage_class, content_hash);
        fs::remove_file(path).await?;
        Ok(())
    }
}
```

**Responsibility:** Implement file I/O operations

```rust
// infrastructure/persistence/postgres_object_repository.rs
pub struct PostgresObjectRepository {
    pool: PgPool,
}

#[async_trait]
impl ObjectRepository for PostgresObjectRepository {
    async fn save(&self, object: &Object) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO objects (id, namespace, tenant_id, key, status, storage_class, content_hash, size_bytes)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                status = EXCLUDED.status,
                content_hash = EXCLUDED.content_hash,
                size_bytes = EXCLUDED.size_bytes
            "#
        )
        .bind(object.id())
        .bind(object.namespace())
        .bind(object.tenant_id())
        .bind(object.key())
        .bind(object.status())
        .bind(object.storage_class())
        .bind(object.content_hash())
        .bind(object.size_bytes())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &ObjectId) -> Result<Option<Object>, RepositoryError> {
        // Map DB row to domain entity
        let row = sqlx::query_as::<_, ObjectRow>(
            "SELECT * FROM objects WHERE id = $1 AND status = 'COMMITTED'"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Object::from))
    }
}
```

**Responsibility:** Implement database operations

---

## Benefits of This Architecture

### ✅ Testability

```rust
#[cfg(test)]
mod tests {
    // Mock repositories for testing use cases
    struct MockObjectRepository { /* ... */ }

    #[tokio::test]
    async fn test_upload_success() {
        let mock_repo = Arc::new(MockObjectRepository::new());
        let mock_store = Arc::new(MockBlobStore::new());
        let mock_publisher = Arc::new(NoopEventPublisher);

        let use_case = UploadObjectUseCase::new(
            mock_repo,
            mock_store,
            mock_publisher,
        );

        let result = use_case.execute(request, reader).await;

        assert!(result.is_ok());
        assert_eq!(mock_repo.save_call_count(), 2);  // Reserve + commit
    }
}
```

### ✅ Maintainability

Each layer can be modified independently:

- Change DB schema? Update `PostgresObjectRepository` only
- Add S3 support? Create `S3BlobStore` impl
- Change API format? Update handlers only

### ✅ Flexibility

Easy to swap implementations:

```rust
// Development
let blob_store = Arc::new(LocalFilesystemStore::new(...));

// Production with S3
let blob_store = Arc::new(S3BlobStore::new(...));

// Testing
let blob_store = Arc::new(MockBlobStore::new());

// Use case doesn't care!
let use_case = UploadObjectUseCase::new(repo, blob_store, publisher);
```

### ✅ Domain-Driven Design

Business rules live in the domain:

```rust
// Business rule enforced by domain entity
object.commit(hash, size)?;  // Can only commit if WRITING

// Not scattered across repositories/handlers
```

---

## Comparison: Before vs After

### Before (Monolithic)

```rust
struct LocalFsObjectStore {
    db: PgPool,              // Mixed concerns!
    hot_root: PathBuf,
    cold_root: PathBuf,
}

impl LocalFsObjectStore {
    async fn put(&self, ...) {
        // DB transaction
        // File I/O
        // Hash computation
        // Business logic
        // All mixed together!
    }
}
```

**Problems:**

- Hard to test (requires real DB + FS)
- Hard to extend (tightly coupled)
- Violates SRP (too many responsibilities)

### After (Clean Architecture)

```rust
// Domain (pure logic)
struct Object { ... }

// Application (orchestration)
struct UploadObjectUseCase {
    repo: Arc<dyn ObjectRepository>,      // Port
    blob_store: Arc<dyn BlobStore>,       // Port
}

// Infrastructure (implementations)
struct PostgresObjectRepository { ... }
struct LocalFilesystemStore { ... }
```

**Benefits:**

- Easy to test (mock ports)
- Easy to extend (add implementations)
- Follows SRP (each class has one job)

---

## Migration Path

### Phase 1: Define Ports (Traits)

1. Create `application/ports/` directory
2. Define `ObjectRepository`, `BlobStore` traits
3. Keep existing code working

### Phase 2: Extract Domain

1. Move business logic to `domain/entities/`
2. Create value objects for type safety
3. Refactor gradually

### Phase 3: Implement Use Cases

1. Create `application/use_cases/`
2. Move orchestration logic from handlers
3. Inject ports

### Phase 4: Refactor Infrastructure

1. Implement ports in `infrastructure/`
2. Split `LocalFsObjectStore` into separate classes
3. Replace direct usage with DI

### Phase 5: Update API Layer

1. Update handlers to use use cases
2. Simplify to pure HTTP mapping

---

## Conclusion

The key insight: **Don't put everything in "storage".**

Instead:

- **Domain** = business rules (what)
- **Application** = workflows (how)
- **Infrastructure** = implementations (where)
- **API** = interfaces (who)

Each layer has ONE job, and dependencies point inward (toward domain).

This is how you build maintainable, testable, extensible systems.
