# Longhorn vs. ActiveStorage: Responsibility Boundaries

This document explicitly defines what Longhorn + ZFS solve for us versus what ActiveStorage must implement.

## Repository Information

**Project:** [just_storage](https://github.com/Glowing-Pixels-UG/just_storage)  
**Description:** Content-addressable object storage with strong consistency, automatic deduplication, and crash-safe operations  
**License:** MIT License  
**Primary Language:** Rust  
**Topics:** `clean-architecture`, `content-addressable`, `object-storage`, `rust`, `storage`  
**Default Branch:** `main`  
**Visibility:** Public

---

## The Stack

```
┌─────────────────────────────────────┐
│     ActiveStorage Service API       │ ← We implement this
│  (Objects, Tenants, Consistency)    │
├─────────────────────────────────────┤
│       POSIX Filesystem              │ ← Interface boundary
├─────────────────────────────────────┤
│      Longhorn Volume Manager        │ ← CSI provides this
│   (Replication, Snapshots, HA)      │
├─────────────────────────────────────┤
│          ZFS Pools                  │ ← Disk redundancy
│    (Checksums, CoW, Scrubbing)      │
├─────────────────────────────────────┤
│       Physical Disks                │
│   NVMe (hot) / HDD (cold)           │
└─────────────────────────────────────┘
```

---

## What Longhorn + ZFS Handle (We Don't Re-Implement)

### 1. Node-Level High Availability

**Longhorn provides:**

- Multiple replicas of each volume across different nodes
- Automatic failover if a node dies
- Replica rebuilding on new nodes
- Pod can continue accessing volume despite node failures

**What this means for us:**

- ✅ We don't implement cross-node replication
- ✅ We don't handle node failure detection
- ✅ We assume the filesystem is always available (within SLA)
- ✅ Single StatefulSet replica is fine for v1

**Example scenario:**

```
Node A crashes → Longhorn automatically serves volume from Node B
Our service pod restarts on Node C → volume re-attaches
No data loss, no service code needed for this
```

---

### 2. Disk-Level Redundancy & Integrity

**ZFS provides:**

- RAID-Z2 (8 disks + 2 parity) → tolerates 2 disk failures
- End-to-end checksums → detects bit rot, silent corruption
- Copy-on-write → never overwrites data in place
- `zfs scrub` → validates all checksums periodically

**What this means for us:**

- ✅ We don't detect bad blocks or disk failures
- ✅ We don't implement RAID or disk-level redundancy
- ✅ ZFS checksums handle block-level corruption
- ✅ We focus on object-level integrity (content hashing)

**Example scenario:**

```
Disk develops bad sectors → ZFS detects via checksum, rebuilds from parity
Our service: unaware, continues reading/writing normally
```

---

### 3. Volume Snapshots & Backups

**Longhorn provides:**

- Point-in-time snapshots (CoW, minimal overhead)
- Incremental backups to remote targets (Hetzner Storage Box)
- Restore from snapshot/backup
- Scheduled backup policies

**What this means for us:**

- ✅ We don't implement volume-level backup logic
- ✅ Our service just needs crash-consistent state (like a DB)
- ✅ Disaster recovery = restore Longhorn volume
- ✅ Backup frequency/retention configured in Longhorn, not our app

**Example scenario:**

```
Weekly: Longhorn snapshots → backs up to Hetzner
Disaster: Restore volume → our service starts → all objects intact
Our service: no backup code, just atomic writes
```

---

### 4. Volume Attachment & Scheduling

**Kubernetes CSI + Longhorn provide:**

- PVC provisioning
- Attaching volumes to pods
- Ensuring only authorized pods can mount a volume
- Volume topology awareness (which nodes have replicas)

**What this means for us:**

- ✅ We don't manage "which node has the disk"
- ✅ Kubernetes mounts `/data/hot` and `/data/cold` automatically
- ✅ We just use those paths

**Example scenario:**

```yaml
volumeMounts:
- name: hot-storage
  mountPath: /data/hot

# Longhorn CSI handles everything below this
```

---

### 5. Performance & IOPS Management

**Longhorn + ZFS provide:**

- ARC caching (ZFS in-memory cache)
- Async replication between Longhorn replicas
- ZFS prefetching for sequential reads

**What this means for us:**

- ✅ We don't implement caching layers
- ✅ We just write/read files; caching happens underneath
- ⚠️ We should tune ZFS recordsize for workload (1M for large blobs)

---

## What ActiveStorage Must Handle (Not Longhorn's Job)

### 1. Object Semantics & Metadata

**Problem:** Longhorn sees only:

```
/data/hot/sha256/ab/abcdef123...
```

It doesn't know:

- This is "GPT-4 model for tenant Acme"
- This belongs to namespace "models"
- This file is the canonical copy vs. a temp upload

**Our responsibility:**

```rust
struct ObjectMeta {
    id: Uuid,
    namespace: String,    // "models", "kb", "uploads"
    tenant: String,       // "acme"
    key: Option<String>,  // "gpt-4-turbo"
    status: ObjectStatus, // WRITING, COMMITTED, DELETING
    content_hash: String, // "sha256:abcdef..."
    size_bytes: i64,
}
```

**Implementation:**

- Metadata stored in Postgres
- Files are content-addressed by hash
- Mapping from logical name → physical path managed by us

---

### 2. Write Atomicity & Crash Safety

**Problem:** Longhorn ensures bytes hit disk, but doesn't know:

- Is this file complete or half-written?
- Is the metadata committed?
- Should readers see this yet?

**Our responsibility: Two-phase commit**

```rust
// Phase 1: Reserve
txn.execute(
    "INSERT INTO objects (id, status) VALUES ($1, 'WRITING')",
    &[&id]
).await?;
txn.commit().await?;

// Phase 2: Write file
let temp = format!("/data/hot/tmp/upload-{}", uuid);
write_and_hash(&temp, reader).await?;
fsync_file(&temp).await?;
rename(&temp, &final_path).await?;
fsync_dir(parent_of(&final_path)).await?;

// Phase 3: Commit metadata
txn.execute(
    "UPDATE objects SET status = 'COMMITTED', content_hash = $1 WHERE id = $2",
    &[&hash, &id]
).await?;
txn.commit().await?;
```

**Crash scenarios:**

| Crash Point | File State | DB State | Recovery |
|-------------|------------|----------|----------|
| Before phase 1 | None | None | No-op |
| After phase 1, before phase 2 | None or temp | `WRITING` | GC cleanup (orphan) |
| After phase 2, before phase 3 | Final path exists | `WRITING` | GC cleanup or retry commit |
| After phase 3 | Final path | `COMMITTED` | ✅ Success |

**Key insight:** DB status is source of truth; filesystem is just storage.

---

### 3. Concurrency Control

**Problem:** Two pods (or requests) try to:

- Write same logical key (`models/acme/gpt-4`)
- Read while another deletes
- GC while another writes

**Longhorn doesn't solve this** — it's a block device, no cross-process coordination.

**Our responsibility:**

#### Concurrent Writes

```sql
-- Strategy A: First-wins
INSERT INTO objects (namespace, tenant, key, ...)
VALUES ('models', 'acme', 'gpt-4', ...)
-- UNIQUE constraint fails → 409 Conflict

-- Strategy B: Last-wins
INSERT ... ON CONFLICT (namespace, tenant, key)
DO UPDATE SET status = 'WRITING', updated_at = now();
```

#### Concurrent Read + Delete

```rust
// Delete: mark, don't remove immediately
txn.execute("UPDATE objects SET status = 'DELETING' WHERE id = $1").await?;
txn.execute("UPDATE blobs SET ref_count = ref_count - 1 WHERE hash = $1").await?;

// File stays on disk → readers with open FDs unaffected
// Background GC removes file only when ref_count = 0
```

#### GC Safety

```sql
-- GC only removes blobs with no references
DELETE FROM blobs
WHERE ref_count = 0 AND gc_pending = true;
```

**Key insight:** DB transactions + deferred GC prevent races.

---

### 4. Listing & Search

**Problem:** Longhorn/ZFS provide:

```bash
ls /data/hot/sha256/ab/
# abcdef123...
# abcdef456...
```

But you want:

```
GET /v1/objects?namespace=models&tenant=acme&created_after=2025-01-01
```

**Our responsibility:**

- Never list filesystem directly (slow, no filtering)
- Use DB index:

```sql
SELECT id, key, size_bytes, created_at
FROM objects
WHERE namespace = 'models'
  AND tenant = 'acme'
  AND status = 'COMMITTED'
  AND created_at > '2025-01-01'
ORDER BY created_at DESC
LIMIT 50;
```

**Performance:**

- ✅ Fast: indexed query
- ✅ Bounded: pagination with cursor
- ✅ Filterable: tenant, namespace, tags, dates

---

### 5. Object-Level Integrity

**Problem:** ZFS checksums blocks, but:

- Doesn't verify "is this the GPT-4 model I uploaded?"
- Can't detect application-level bugs (e.g., wrote wrong data)

**Our responsibility:**

```rust
// On upload
let hash = Sha256::digest(&bytes);
let content_hash = format!("sha256:{}", hex::encode(hash));

// Store in DB
object.content_hash = content_hash.clone();

// Use hash in filename
let path = format!("/data/hot/sha256/{}/{}", &hash[0..2], hash);

// Optional: background scrubber
for object in db.query("SELECT id, content_hash FROM objects WHERE status = 'COMMITTED'") {
    let file = open_file_for_object(&object)?;
    let computed = hash_file(&file)?;
    if computed != object.content_hash {
        alert!("Corruption detected: {}", object.id);
        mark_corrupted(&object.id)?;
    }
}
```

**Key insight:** Content hashes give end-to-end integrity guarantee, independent of ZFS.

---

### 6. Lifecycle Management & Garbage Collection

**Problem:** Over time:

- Old versions accumulate
- Orphaned temp files from crashes
- Deleted objects' files still on disk

**Longhorn doesn't clean up** — it doesn't understand object lifecycle.

**Our responsibility:**

#### Orphan Cleanup

```sql
-- Find objects stuck in WRITING > 1 hour
SELECT id, created_at FROM objects
WHERE status = 'WRITING'
  AND created_at < now() - interval '1 hour';

-- Clean up
DELETE FROM objects WHERE id = $1;
-- Remove temp file if exists
```

#### Deleted Objects GC

```sql
-- Background worker
SELECT content_hash FROM blobs
WHERE ref_count = 0 AND gc_pending = true
LIMIT 100;

-- For each
unlink(file_path);
DELETE FROM blobs WHERE content_hash = $1;
UPDATE objects SET status = 'DELETED' WHERE content_hash = $1;
```

#### Old Versions (Future)

```sql
-- Keep last 3 versions per key
DELETE FROM objects
WHERE (namespace, tenant, key, version) IN (
    SELECT namespace, tenant, key, version
    FROM objects
    WHERE status = 'COMMITTED'
    ORDER BY version DESC
    OFFSET 3
);
```

---

### 7. Access Control & Tenancy

**Problem:** Longhorn volume is mounted, anyone in the pod can read any file.

**Our responsibility:**

```rust
// API layer enforces tenancy
async fn download_object(
    Path(id): Path<Uuid>,
    Extension(auth): Extension<AuthContext>,
) -> Result<impl IntoResponse> {
    let object = db.get_object(id).await?;

    // Verify tenant
    if object.tenant != auth.tenant {
        return Err(Error::Forbidden);
    }

    // Serve file
    let file = open_file(&object.content_hash)?;
    Ok(file_stream(file))
}
```

---

### 8. Observability & Metrics

**Problem:** Longhorn provides volume-level metrics (IOPS, latency, capacity).

It doesn't know:

- Which tenant is using the most storage?
- How many objects per namespace?
- API request rates, error rates?

**Our responsibility:**

```rust
// Prometheus metrics
lazy_static! {
    static ref REQUESTS: IntCounterVec = register_int_counter_vec!(
        "activestorage_requests_total",
        "Total requests",
        &["method", "status", "namespace"]
    ).unwrap();

    static ref STORAGE_BYTES: GaugeVec = register_gauge_vec!(
        "activestorage_storage_bytes",
        "Storage used",
        &["storage_class", "namespace", "tenant"]
    ).unwrap();
}

// Update on each request
REQUESTS.with_label_values(&["GET", "200", "models"]).inc();

// Periodic job: update storage metrics from DB
let stats = db.query("
    SELECT namespace, tenant, storage_class, SUM(size_bytes)
    FROM objects
    WHERE status = 'COMMITTED'
    GROUP BY namespace, tenant, storage_class
").await?;

for row in stats {
    STORAGE_BYTES
        .with_label_values(&[&row.storage_class, &row.namespace, &row.tenant])
        .set(row.total_bytes);
}
```

---

## Decision Tree: "Should Longhorn or ActiveStorage Handle This?"

| Concern | Who Handles | Why |
|---------|-------------|-----|
| Node crashes | **Longhorn** | Volume replicas, failover |
| Disk fails | **ZFS** | RAID-Z, checksums |
| Volume backup | **Longhorn** | Snapshots → Hetzner |
| File is half-written | **ActiveStorage** | DB state machine |
| Two clients write same key | **ActiveStorage** | DB transactions |
| List "all models for tenant X" | **ActiveStorage** | DB index |
| Detect corrupted file | Both | ZFS: blocks, **ActiveStorage**: end-to-end |
| Delete old versions | **ActiveStorage** | Lifecycle policy |
| "Who used 1TB this month?" | **ActiveStorage** | Per-tenant metrics |

---

## Example Scenarios

### Scenario 1: Node Failure During Upload

```
1. Client: POST /v1/objects (starts upload to Node A)
2. Node A: writes temp file, DB says status=WRITING
3. ❌ Node A crashes
4. Kubernetes: restarts pod on Node B
5. Longhorn: attaches volume to Node B (replicas on B, C were synced)
6. ActiveStorage (Node B):
   - Finds object with status=WRITING, created 10 min ago
   - Orphan cleanup: DELETE FROM objects WHERE id = $1
   - Removes temp file if exists
7. Client: Receives 500, retries upload
8. ✅ Second attempt succeeds
```

**Longhorn handled:** Volume failover
**ActiveStorage handled:** Orphan cleanup, retry logic

---

### Scenario 2: Concurrent Deletes + Reads

```
1. Client A: GET /v1/objects/{id} (starts streaming file)
2. Client B: DELETE /v1/objects/{id}
3. ActiveStorage:
   - DELETE request: UPDATE objects SET status='DELETING', ref_count -= 1
   - File still on disk
   - Client A's file descriptor still open → read continues ✅
4. Background GC (5 min later):
   - Sees ref_count=0, gc_pending=true
   - unlink(file)
   - Client A already finished
5. ✅ No race, clean delete
```

**Longhorn handled:** Nothing (filesystem atomics sufficient)
**ActiveStorage handled:** Deferred GC, state management

---

### Scenario 3: Disk Corruption

```
1. Disk sector goes bad on Node A
2. ZFS: detects checksum mismatch during read
3. ZFS: rebuilds block from parity, logs error
4. Longhorn: continues serving (volume healthy)
5. Optional: ActiveStorage scrubber runs
   - Recomputes SHA-256 of objects
   - All match → ✅ no corruption at object level
6. Admin: sees ZFS error logs, replaces disk
```

**ZFS handled:** Block-level detection & repair
**Longhorn handled:** Volume availability during repair
**ActiveStorage handled:** Optional end-to-end verification

---

## Summary Table

| Layer | Responsibility | Failure Domain | Our Code Involvement |
|-------|----------------|----------------|----------------------|
| **Physical Disks** | Store bytes | Disk hardware | ❌ None |
| **ZFS** | RAID, checksums, CoW | Disk/block corruption | ❌ None (tune config) |
| **Longhorn** | Volume HA, replication, backups | Node/cluster | ❌ None (CSI mounts) |
| **POSIX Filesystem** | read/write/rename semantics | Process crash | ✅ Use correctly |
| **ActiveStorage** | Objects, consistency, lifecycle | Application logic | ✅ All of this |

---

## Key Takeaway

**Longhorn + ZFS solve infrastructure concerns:**

- "Is the data replicated?"
- "Can I survive node/disk failures?"
- "How do I back up?"

**ActiveStorage solves application concerns:**

- "What is this blob?"
- "Who can access it?"
- "Is it a complete upload or garbage?"
- "How do I find it?"

**Don't overlap:** Trust Longhorn for what it's good at, implement the rest cleanly at the application layer.
