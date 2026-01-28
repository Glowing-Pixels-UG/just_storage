-- Fix return types for GC functions to match Rust i64 (BIGINT)
-- We must DROP first because CREATE OR REPLACE cannot change return type
DROP FUNCTION IF EXISTS cleanup_stuck_uploads(BIGINT);

CREATE OR REPLACE FUNCTION cleanup_stuck_uploads(p_age_hours BIGINT DEFAULT 1)
RETURNS BIGINT AS $$
DECLARE
    v_deleted BIGINT;
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
