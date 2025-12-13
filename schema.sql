-- ActiveStorage Database Schema
-- PostgreSQL 14+

-- Extension for UUID generation
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================================
-- OBJECTS TABLE
-- ============================================================================
-- Stores metadata for each logical object (model, file, blob, etc.)
-- This is the source of truth for what exists and its current state.

CREATE TABLE objects (
    -- Identity
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- Logical organization
    namespace       TEXT NOT NULL,              -- 'models', 'kb', 'uploads', 'logs', etc.
    tenant_id       TEXT NOT NULL,              -- Tenant/org identifier
    key             TEXT,                       -- Optional human-readable key (e.g., 'gpt-4-turbo')

    -- State machine
    status          TEXT NOT NULL CHECK (status IN ('WRITING', 'COMMITTED', 'DELETING', 'DELETED')),

    -- Storage location
    storage_class   TEXT NOT NULL CHECK (storage_class IN ('hot', 'cold')),

    -- Content metadata (filled after upload completes)
    content_hash    TEXT,                       -- 'sha256:abcdef123...' (filled when status=COMMITTED)
    size_bytes      BIGINT,                     -- Size in bytes
    content_type    TEXT,                       -- MIME type (e.g., 'application/octet-stream')

    -- Timestamps
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_access_at  TIMESTAMPTZ,                -- Updated on reads (optional, for access tracking)

    -- Optional: metadata bag for app-specific fields
    metadata        JSONB,                      -- e.g., {"model_version": "v1.2", "format": "safetensors"}

    -- Constraints
    CONSTRAINT unique_key_per_tenant_ns UNIQUE (namespace, tenant_id, key)
        WHERE key IS NOT NULL AND status != 'DELETED'
);

-- Indexes for common queries
CREATE INDEX idx_objects_status ON objects(status);
CREATE INDEX idx_objects_tenant_ns ON objects(tenant_id, namespace) WHERE status = 'COMMITTED';
CREATE INDEX idx_objects_created ON objects(created_at DESC) WHERE status = 'COMMITTED';
CREATE INDEX idx_objects_content_hash ON objects(content_hash) WHERE content_hash IS NOT NULL;

-- Extension for full-text search capabilities
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Indexes for advanced search and filtering
CREATE INDEX idx_objects_content_type ON objects(content_type) WHERE status = 'COMMITTED';
CREATE INDEX idx_objects_storage_class ON objects(storage_class) WHERE status = 'COMMITTED';
CREATE INDEX idx_objects_size_bytes ON objects(size_bytes) WHERE status = 'COMMITTED' AND size_bytes IS NOT NULL;
CREATE INDEX idx_objects_created_at_range ON objects(created_at) WHERE status = 'COMMITTED';
CREATE INDEX idx_objects_updated_at_range ON objects(updated_at) WHERE status = 'COMMITTED';

-- GIN indexes for JSONB metadata queries
CREATE INDEX idx_objects_metadata_gin ON objects USING GIN (metadata jsonb_path_ops) WHERE status = 'COMMITTED';
CREATE INDEX idx_objects_metadata_trgm ON objects USING GIN (metadata::text gin_trgm_ops) WHERE status = 'COMMITTED';

-- GIN index for fuzzy text search on keys
CREATE INDEX idx_objects_key_trgm ON objects USING GIN (key gin_trgm_ops) WHERE status = 'COMMITTED' AND key IS NOT NULL;

-- Composite indexes for common query patterns
CREATE INDEX idx_objects_tenant_ns_content_type ON objects(tenant_id, namespace, content_type) WHERE status = 'COMMITTED';
CREATE INDEX idx_objects_tenant_ns_storage_class ON objects(tenant_id, namespace, storage_class) WHERE status = 'COMMITTED';
CREATE INDEX idx_objects_tenant_ns_created_at ON objects(tenant_id, namespace, created_at DESC) WHERE status = 'COMMITTED';

-- Specialized indexes for metadata fields
CREATE INDEX idx_objects_metadata_kind ON objects ((metadata->>'kind')) WHERE status = 'COMMITTED';
CREATE INDEX idx_objects_metadata_model_name ON objects ((metadata->'model'->>'model_name')) WHERE status = 'COMMITTED' AND metadata->'model' IS NOT NULL;
CREATE INDEX idx_objects_metadata_kb_title ON objects ((metadata->'kb_doc'->>'title')) WHERE status = 'COMMITTED' AND metadata->'kb_doc' IS NOT NULL;
CREATE INDEX idx_objects_metadata_kb_source ON objects ((metadata->'kb_doc'->>'source')) WHERE status = 'COMMITTED' AND metadata->'kb_doc' IS NOT NULL;

-- Trigger to update updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_objects_updated_at BEFORE UPDATE ON objects
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================================================
-- BLOBS TABLE
-- ============================================================================
-- Tracks physical files on disk, with reference counting for deduplication.
-- Multiple objects can reference the same blob (same content_hash).

CREATE TABLE blobs (
    -- Identity (content-addressed)
    content_hash    TEXT PRIMARY KEY,           -- 'sha256:abcdef123...'

    -- Storage metadata
    storage_class   TEXT NOT NULL CHECK (storage_class IN ('hot', 'cold')),
    ref_count       BIGINT NOT NULL DEFAULT 0,  -- Number of objects referencing this blob

    -- Garbage collection
    gc_pending      BOOLEAN NOT NULL DEFAULT false, -- Marked for deletion when ref_count=0

    -- Timestamps
    first_seen_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at    TIMESTAMPTZ,

    -- Constraints
    CHECK (ref_count >= 0)
);

-- Indexes for GC worker
CREATE INDEX idx_blobs_gc ON blobs(gc_pending, ref_count) WHERE gc_pending = true;
CREATE INDEX idx_blobs_last_used ON blobs(last_used_at) WHERE ref_count > 0;

-- ============================================================================
-- HELPER VIEWS
-- ============================================================================

-- Active objects (what users see)
CREATE VIEW active_objects AS
SELECT
    id,
    namespace,
    tenant_id,
    key,
    storage_class,
    content_hash,
    size_bytes,
    content_type,
    created_at,
    updated_at,
    last_access_at,
    metadata
FROM objects
WHERE status = 'COMMITTED';

-- Storage usage by tenant/namespace
CREATE VIEW storage_usage AS
SELECT
    tenant_id,
    namespace,
    storage_class,
    COUNT(*) as object_count,
    SUM(size_bytes) as total_bytes,
    SUM(size_bytes) / (1024.0 * 1024.0 * 1024.0) as total_gb
FROM objects
WHERE status = 'COMMITTED'
GROUP BY tenant_id, namespace, storage_class;

-- Orphaned blobs (ref_count=0, not pending GC)
CREATE VIEW orphaned_blobs AS
SELECT
    content_hash,
    storage_class,
    first_seen_at,
    last_used_at
FROM blobs
WHERE ref_count = 0 AND gc_pending = false;

-- Stuck uploads (in WRITING state for > 1 hour)
CREATE VIEW stuck_uploads AS
SELECT
    id,
    namespace,
    tenant_id,
    key,
    created_at,
    EXTRACT(EPOCH FROM (now() - created_at)) as age_seconds
FROM objects
WHERE status = 'WRITING'
  AND created_at < now() - interval '1 hour'
ORDER BY created_at;

-- ============================================================================
-- FUNCTIONS FOR SAFE OPERATIONS
-- ============================================================================

-- Increment blob reference count (called on object commit)
CREATE OR REPLACE FUNCTION increment_blob_ref(p_content_hash TEXT, p_storage_class TEXT)
RETURNS void AS $$
BEGIN
    INSERT INTO blobs (content_hash, storage_class, ref_count)
    VALUES (p_content_hash, p_storage_class, 1)
    ON CONFLICT (content_hash)
    DO UPDATE SET
        ref_count = blobs.ref_count + 1,
        last_used_at = now();
END;
$$ LANGUAGE plpgsql;

-- Decrement blob reference count (called on object delete)
CREATE OR REPLACE FUNCTION decrement_blob_ref(p_content_hash TEXT)
RETURNS void AS $$
BEGIN
    UPDATE blobs
    SET
        ref_count = ref_count - 1,
        gc_pending = (ref_count - 1 = 0)
    WHERE content_hash = p_content_hash;

    IF NOT FOUND THEN
        RAISE WARNING 'Blob not found: %', p_content_hash;
    END IF;
END;
$$ LANGUAGE plpgsql;

-- Mark object as committed (transaction-safe)
CREATE OR REPLACE FUNCTION commit_object(
    p_id UUID,
    p_content_hash TEXT,
    p_size_bytes BIGINT,
    p_content_type TEXT,
    p_storage_class TEXT
)
RETURNS void AS $$
BEGIN
    -- Update object
    UPDATE objects
    SET
        status = 'COMMITTED',
        content_hash = p_content_hash,
        size_bytes = p_size_bytes,
        content_type = p_content_type
    WHERE id = p_id AND status = 'WRITING';

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Object not found or not in WRITING state: %', p_id;
    END IF;

    -- Increment blob ref count
    PERFORM increment_blob_ref(p_content_hash, p_storage_class);
END;
$$ LANGUAGE plpgsql;

-- Delete object (mark for GC)
CREATE OR REPLACE FUNCTION delete_object(p_id UUID)
RETURNS void AS $$
DECLARE
    v_content_hash TEXT;
BEGIN
    -- Get content hash and mark as deleting
    UPDATE objects
    SET status = 'DELETING'
    WHERE id = p_id AND status = 'COMMITTED'
    RETURNING content_hash INTO v_content_hash;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Object not found or not in COMMITTED state: %', p_id;
    END IF;

    -- Decrement ref count
    IF v_content_hash IS NOT NULL THEN
        PERFORM decrement_blob_ref(v_content_hash);
    END IF;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- MAINTENANCE QUERIES (for GC worker)
-- ============================================================================

-- Get blobs ready for deletion
CREATE OR REPLACE FUNCTION get_blobs_for_gc(p_limit INT DEFAULT 100)
RETURNS TABLE(content_hash TEXT, storage_class TEXT) AS $$
BEGIN
    RETURN QUERY
    SELECT b.content_hash, b.storage_class
    FROM blobs b
    WHERE b.gc_pending = true
      AND b.ref_count = 0
    LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

-- Mark blob as fully deleted
CREATE OR REPLACE FUNCTION mark_blob_deleted(p_content_hash TEXT)
RETURNS void AS $$
BEGIN
    DELETE FROM blobs WHERE content_hash = p_content_hash;

    UPDATE objects
    SET status = 'DELETED'
    WHERE content_hash = p_content_hash AND status = 'DELETING';
END;
$$ LANGUAGE plpgsql;

-- Clean up stuck uploads
CREATE OR REPLACE FUNCTION cleanup_stuck_uploads(p_age_hours INT DEFAULT 1)
RETURNS INT AS $$
DECLARE
    v_deleted INT;
BEGIN
    WITH deleted AS (
        DELETE FROM objects
        WHERE status = 'WRITING'
          AND created_at < now() - (p_age_hours || ' hours')::interval
        RETURNING id
    )
    SELECT COUNT(*) INTO v_deleted FROM deleted;

    RETURN v_deleted;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- EXAMPLE QUERIES
-- ============================================================================

-- Upload workflow
-- ----------------
-- 1. Start upload
-- INSERT INTO objects (namespace, tenant_id, key, status, storage_class)
-- VALUES ('models', 'acme', 'gpt-4', 'WRITING', 'hot')
-- RETURNING id;

-- 2. Write file to /data/hot/tmp/upload-<uuid>
-- 3. Compute hash, rename to /data/hot/sha256/ab/abcdef...
-- 4. Commit

-- SELECT commit_object(
--     '550e8400-e29b-41d4-a716-446655440000',
--     'sha256:abcdef123...',
--     1073741824,
--     'application/octet-stream',
--     'hot'
-- );

-- Download workflow
-- -----------------
-- SELECT id, content_hash, size_bytes, content_type, storage_class
-- FROM objects
-- WHERE namespace = 'models'
--   AND tenant_id = 'acme'
--   AND key = 'gpt-4'
--   AND status = 'COMMITTED';

-- Delete workflow
-- ---------------
-- SELECT delete_object('550e8400-e29b-41d4-a716-446655440000');

-- List objects
-- ------------
-- SELECT id, key, size_bytes, created_at
-- FROM objects
-- WHERE namespace = 'models'
--   AND tenant_id = 'acme'
--   AND status = 'COMMITTED'
-- ORDER BY created_at DESC
-- LIMIT 50;

-- Storage usage
-- -------------
-- SELECT * FROM storage_usage
-- WHERE tenant_id = 'acme';

-- GC operations
-- -------------
-- SELECT * FROM get_blobs_for_gc(100);
-- SELECT mark_blob_deleted('sha256:abcdef123...');

-- Maintenance
-- -----------
-- SELECT cleanup_stuck_uploads(1); -- Clean up uploads older than 1 hour
