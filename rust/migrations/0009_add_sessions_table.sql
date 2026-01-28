-- Add sessions table for tower-sessions
-- This schema is required for tower-sessions-sqlx-store (Postgres)

CREATE SCHEMA IF NOT EXISTS tower_sessions;

CREATE TABLE IF NOT EXISTS tower_sessions.session (
    id TEXT PRIMARY KEY NOT NULL,
    data BYTEA NOT NULL,
    expiry_date TIMESTAMPTZ NOT NULL
);

-- Index for faster cleanup of expired sessions
CREATE INDEX IF NOT EXISTS idx_session_expiry_date ON tower_sessions.session (expiry_date);
