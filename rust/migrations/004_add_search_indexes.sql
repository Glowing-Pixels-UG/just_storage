-- Add indexes for efficient search and filtering operations
-- Run this after the metadata column has been added

-- Indexes for filtering by common fields
CREATE INDEX IF NOT EXISTS idx_objects_content_type ON objects(content_type) WHERE status = 'COMMITTED';
CREATE INDEX IF NOT EXISTS idx_objects_storage_class ON objects(storage_class) WHERE status = 'COMMITTED';
CREATE INDEX IF NOT EXISTS idx_objects_size_bytes ON objects(size_bytes) WHERE status = 'COMMITTED' AND size_bytes IS NOT NULL;

-- Indexes for date range filtering
CREATE INDEX IF NOT EXISTS idx_objects_created_at_range ON objects(created_at) WHERE status = 'COMMITTED';
CREATE INDEX IF NOT EXISTS idx_objects_updated_at_range ON objects(updated_at) WHERE status = 'COMMITTED';

-- GIN index for JSONB metadata queries (jsonb_path_ops for faster containment queries)
CREATE INDEX IF NOT EXISTS idx_objects_metadata_gin ON objects USING GIN (metadata jsonb_path_ops) WHERE status = 'COMMITTED';

-- Try to create pg_trgm extension for fuzzy text search
-- Note: This may fail if user doesn't have CREATE EXTENSION privileges
-- In that case, the extension must be created manually by a superuser
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_trgm') THEN
        BEGIN
            CREATE EXTENSION IF NOT EXISTS pg_trgm;
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE 'Could not create pg_trgm extension: %. Trigram indexes will be skipped.', SQLERRM;
        END;
    END IF;
END $$;

-- GIN index for trigram-based fuzzy text search on metadata
-- Only create if pg_trgm extension exists
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_trgm') THEN
        IF NOT EXISTS (
            SELECT 1 FROM pg_indexes 
            WHERE indexname = 'idx_objects_metadata_trgm'
        ) THEN
            CREATE INDEX idx_objects_metadata_trgm ON objects 
            USING GIN ((metadata::text) gin_trgm_ops) 
            WHERE status = 'COMMITTED';
        END IF;
    END IF;
END $$;

-- GIN index for trigram-based fuzzy text search on keys
-- Only create if pg_trgm extension exists
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_trgm') THEN
        IF NOT EXISTS (
            SELECT 1 FROM pg_indexes 
            WHERE indexname = 'idx_objects_key_trgm'
        ) THEN
            CREATE INDEX idx_objects_key_trgm ON objects 
            USING GIN (key gin_trgm_ops) 
            WHERE status = 'COMMITTED' AND key IS NOT NULL;
        END IF;
    END IF;
END $$;

-- Composite indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_objects_tenant_ns_content_type ON objects(tenant_id, namespace, content_type) WHERE status = 'COMMITTED';
CREATE INDEX IF NOT EXISTS idx_objects_tenant_ns_storage_class ON objects(tenant_id, namespace, storage_class) WHERE status = 'COMMITTED';
CREATE INDEX IF NOT EXISTS idx_objects_tenant_ns_created_at ON objects(tenant_id, namespace, created_at DESC) WHERE status = 'COMMITTED';

-- Partial indexes for specific metadata fields (customize based on your use case)
-- Example: Index for model metadata
CREATE INDEX IF NOT EXISTS idx_objects_metadata_model_name ON objects ((metadata->'model'->>'model_name')) WHERE status = 'COMMITTED' AND metadata->'model' IS NOT NULL;

-- Example: Index for KB document metadata
CREATE INDEX IF NOT EXISTS idx_objects_metadata_kb_title ON objects ((metadata->'kb_doc'->>'title')) WHERE status = 'COMMITTED' AND metadata->'kb_doc' IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_objects_metadata_kb_source ON objects ((metadata->'kb_doc'->>'source')) WHERE status = 'COMMITTED' AND metadata->'kb_doc' IS NOT NULL;

-- Index for object kind (Model, KbDoc, Upload, etc.)
CREATE INDEX IF NOT EXISTS idx_objects_metadata_kind ON objects ((metadata->>'kind')) WHERE status = 'COMMITTED';
