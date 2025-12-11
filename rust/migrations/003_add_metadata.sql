-- Add metadata columns to objects table
-- This adds rich metadata support for models, KB docs, and general extensibility

ALTER TABLE objects
ADD COLUMN IF NOT EXISTS content_type TEXT,
ADD COLUMN IF NOT EXISTS metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
ADD COLUMN IF NOT EXISTS last_access_at TIMESTAMPTZ;

-- Create indexes for efficient metadata queries
CREATE INDEX IF NOT EXISTS idx_objects_metadata_kind ON objects ((metadata->>'kind'));
CREATE INDEX IF NOT EXISTS idx_objects_content_type ON objects (content_type);
CREATE INDEX IF NOT EXISTS idx_objects_last_access_at ON objects (last_access_at);

-- GIN index for flexible JSONB queries on tags
CREATE INDEX IF NOT EXISTS idx_objects_metadata_gin ON objects USING gin (metadata jsonb_path_ops);

-- Example queries enabled by these indexes:
--
-- Find all models:
-- SELECT * FROM objects WHERE metadata->>'kind' = 'model';
--
-- Find llama models:
-- SELECT * FROM objects WHERE metadata->'model'->>'family' = 'llama';
--
-- Find objects with specific tags:
-- SELECT * FROM objects WHERE metadata @> '{"tags": {"env": "prod"}}';
--
-- Find stale objects for tiering:
-- SELECT * FROM objects
-- WHERE storage_class = 'hot'
--   AND last_access_at < NOW() - INTERVAL '30 days';
