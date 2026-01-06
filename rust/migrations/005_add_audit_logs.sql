-- Add audit logs table for security monitoring and compliance
-- This enables persistent storage of security events, API usage, and system activities

CREATE TABLE IF NOT EXISTS audit_logs (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_type TEXT NOT NULL,
    user_id TEXT,
    tenant_id TEXT,
    api_key_id TEXT,
    ip_address INET,
    user_agent TEXT,
    method TEXT,
    path TEXT,
    query TEXT,
    status_code INTEGER,
    response_time_ms BIGINT,
    error_message TEXT,
    additional_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs (timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_event_type ON audit_logs (event_type);
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs (user_id) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_audit_logs_tenant_id ON audit_logs (tenant_id) WHERE tenant_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_audit_logs_api_key_id ON audit_logs (api_key_id) WHERE api_key_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_audit_logs_ip_address ON audit_logs (ip_address) WHERE ip_address IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_audit_logs_path ON audit_logs (path);

-- Partial index for error events (more likely to be queried)
CREATE INDEX IF NOT EXISTS idx_audit_logs_errors ON audit_logs (timestamp DESC)
WHERE error_message IS NOT NULL;

-- GIN index for flexible JSONB queries on additional_data
CREATE INDEX IF NOT EXISTS idx_audit_logs_additional_data_gin ON audit_logs USING gin (additional_data jsonb_path_ops);

-- Partitioning strategy for large audit logs (can be applied later if needed)
-- This table may grow very large, so partitioning by month could be beneficial
-- CREATE TABLE audit_logs_y2024m01 PARTITION OF audit_logs
-- FOR VALUES FROM ('2024-01-01') TO ('2024-02-01');

-- Example queries enabled by these indexes:
--
-- Recent security events:
-- SELECT * FROM audit_logs
-- WHERE event_type IN ('AuthenticationFailure', 'AuthorizationDenied', 'RateLimitExceeded')
--   AND timestamp > NOW() - INTERVAL '24 hours'
-- ORDER BY timestamp DESC;
--
-- API usage by tenant:
-- SELECT tenant_id, COUNT(*) as request_count
-- FROM audit_logs
-- WHERE event_type = 'ObjectRead'
--   AND timestamp > NOW() - INTERVAL '7 days'
-- GROUP BY tenant_id
-- ORDER BY request_count DESC;
--
-- Failed requests:
-- SELECT path, status_code, error_message, COUNT(*) as count
-- FROM audit_logs
-- WHERE status_code >= 400
--   AND timestamp > NOW() - INTERVAL '1 hour'
-- GROUP BY path, status_code, error_message
-- ORDER BY count DESC;

-- Retention policy (can be implemented as a scheduled job)
-- DELETE FROM audit_logs WHERE timestamp < NOW() - INTERVAL '1 year';