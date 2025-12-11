-- Find rows with invalid object status
SELECT id, status FROM objects WHERE status NOT IN ('WRITING','COMMITTED','DELETING','DELETED');

-- Find rows with invalid storage_class
SELECT id, storage_class FROM objects WHERE storage_class NOT IN ('hot','cold');

-- Find rows with invalid content_hash (not a 64-hex string)
SELECT id, content_hash FROM objects WHERE content_hash IS NOT NULL AND content_hash !~ '^[0-9a-fA-F]{64}$';

-- Find blobs with invalid content_hash
SELECT content_hash, storage_class FROM blobs WHERE content_hash !~ '^[0-9a-fA-F]{64}$';

-- Example fix: set invalid status to 'WRITING'
-- UPDATE objects SET status = 'WRITING' WHERE status NOT IN ('WRITING','COMMITTED','DELETING','DELETED');

-- Example fix: set invalid storage_class to 'hot'
-- UPDATE objects SET storage_class = 'hot' WHERE storage_class NOT IN ('hot','cold');
