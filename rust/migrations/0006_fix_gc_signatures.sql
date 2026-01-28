CREATE OR REPLACE FUNCTION cleanup_stuck_uploads(p_age_hours BIGINT DEFAULT 1)
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

CREATE OR REPLACE FUNCTION get_blobs_for_gc(p_limit BIGINT DEFAULT 100)
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
