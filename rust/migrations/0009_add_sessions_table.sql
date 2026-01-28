-- Add sessions table for tower-sessions
-- This schema is required for tower-sessions-sqlx-store (Postgres)

CREATE TABLE IF NOT EXISTS "session" (
    id TEXT PRIMARY KEY NOT NULL,
    data BYTEA NOT NULL,
    expiry_date TIMESTAMPTZ NOT NULL
);

-- Index for faster cleanup of expired sessions
CREATE INDEX IF NOT EXISTS idx_session_expiry_date ON "session" (expiry_date);
