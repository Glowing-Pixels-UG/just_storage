use async_trait::async_trait;
use sqlx::{PgPool, Row};

use crate::api::middleware::audit::AuditLogEntry;
use crate::application::ports::{AuditQueryFilter, AuditRepository, AuditRepositoryError};

pub struct PostgresAuditRepository {
    pool: PgPool,
}

impl PostgresAuditRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuditRepository for PostgresAuditRepository {
    async fn store(&self, entry: AuditLogEntry) -> Result<(), AuditRepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO audit_logs (
                timestamp, event_type, user_id, tenant_id, api_key_id,
                ip_address, user_agent, method, path, query, status_code,
                response_time_ms, error_message, additional_data
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )
        .bind(entry.timestamp)
        .bind(entry.event_type.to_string())
        .bind(entry.user_id)
        .bind(entry.tenant_id)
        .bind(entry.api_key_id)
        .bind(entry.ip_address)
        .bind(entry.user_agent)
        .bind(entry.method)
        .bind(entry.path)
        .bind(entry.query)
        .bind(entry.status_code.map(|c| c as i32))
        .bind(entry.response_time_ms.map(|t| t as i64))
        .bind(entry.error_message)
        .bind(
            entry
                .additional_data
                .map(|d| serde_json::to_value(d).unwrap_or_default()),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn query(
        &self,
        filter: AuditQueryFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<AuditLogEntry>, AuditRepositoryError> {
        let mut query_builder = sqlx::QueryBuilder::new("SELECT * FROM audit_logs WHERE 1=1");

        // Build WHERE clauses based on filter
        if let Some(event_types) = &filter.event_types {
            if !event_types.is_empty() {
                query_builder.push(" AND event_type = ANY(");
                query_builder.push_bind(event_types.as_slice());
                query_builder.push(")");
            }
        }

        if let Some(user_id) = &filter.user_id {
            query_builder.push(" AND user_id = ");
            query_builder.push_bind(user_id);
        }

        if let Some(tenant_id) = &filter.tenant_id {
            query_builder.push(" AND tenant_id = ");
            query_builder.push_bind(tenant_id);
        }

        if let Some(api_key_id) = &filter.api_key_id {
            query_builder.push(" AND api_key_id = ");
            query_builder.push_bind(api_key_id);
        }

        if let Some(ip_address) = &filter.ip_address {
            query_builder.push(" AND ip_address = ");
            query_builder.push_bind(ip_address.clone());
        }

        if let Some(path_pattern) = &filter.path_pattern {
            query_builder.push(" AND path LIKE ");
            query_builder.push_bind(format!("%{}%", path_pattern));
        }

        if let Some(min_code) = filter.status_code_min {
            query_builder.push(" AND status_code >= ");
            query_builder.push_bind(min_code);
        }

        if let Some(max_code) = filter.status_code_max {
            query_builder.push(" AND status_code <= ");
            query_builder.push_bind(max_code);
        }

        if let Some(from_ts) = filter.from_timestamp {
            query_builder.push(" AND timestamp >= ");
            query_builder.push_bind(from_ts);
        }

        if let Some(to_ts) = filter.to_timestamp {
            query_builder.push(" AND timestamp <= ");
            query_builder.push_bind(to_ts);
        }

        if let Some(has_error) = filter.has_error {
            if has_error {
                query_builder.push(" AND error_message IS NOT NULL");
            } else {
                query_builder.push(" AND error_message IS NULL");
            }
        }

        // Order by timestamp descending (most recent first)
        query_builder.push(" ORDER BY timestamp DESC");

        // Add pagination
        query_builder.push(" LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let query = query_builder.build();

        let rows = query.fetch_all(&self.pool).await?;

        let mut entries = Vec::new();
        for row in rows {
            let event_type_str: String = row.try_get("event_type")?;
            let event_type = match event_type_str.as_str() {
                "AuthenticationSuccess" => {
                    crate::api::middleware::audit::AuditEventType::AuthenticationSuccess
                }
                "AuthenticationFailure" => {
                    crate::api::middleware::audit::AuditEventType::AuthenticationFailure
                }
                "ApiKeyUsed" => crate::api::middleware::audit::AuditEventType::ApiKeyUsed,
                "ApiKeyExpired" => crate::api::middleware::audit::AuditEventType::ApiKeyExpired,
                "ApiKeyRevoked" => crate::api::middleware::audit::AuditEventType::ApiKeyRevoked,
                "AuthorizationGranted" => {
                    crate::api::middleware::audit::AuditEventType::AuthorizationGranted
                }
                "AuthorizationDenied" => {
                    crate::api::middleware::audit::AuditEventType::AuthorizationDenied
                }
                "PermissionChecked" => {
                    crate::api::middleware::audit::AuditEventType::PermissionChecked
                }
                "ObjectCreated" => crate::api::middleware::audit::AuditEventType::ObjectCreated,
                "ObjectRead" => crate::api::middleware::audit::AuditEventType::ObjectRead,
                "ObjectUpdated" => crate::api::middleware::audit::AuditEventType::ObjectUpdated,
                "ObjectDeleted" => crate::api::middleware::audit::AuditEventType::ObjectDeleted,
                "ApiKeyCreated" => crate::api::middleware::audit::AuditEventType::ApiKeyCreated,
                "ApiKeyUpdated" => crate::api::middleware::audit::AuditEventType::ApiKeyUpdated,
                "ApiKeyDeleted" => crate::api::middleware::audit::AuditEventType::ApiKeyDeleted,
                "RateLimitExceeded" => {
                    crate::api::middleware::audit::AuditEventType::RateLimitExceeded
                }
                "SuspiciousRequest" => {
                    crate::api::middleware::audit::AuditEventType::SuspiciousRequest
                }
                "InvalidInput" => crate::api::middleware::audit::AuditEventType::InvalidInput,
                "CorsViolation" => crate::api::middleware::audit::AuditEventType::CorsViolation,
                "HealthCheck" => crate::api::middleware::audit::AuditEventType::HealthCheck,
                "ConfigurationChange" => {
                    crate::api::middleware::audit::AuditEventType::ConfigurationChange
                }
                "BackupOperation" => crate::api::middleware::audit::AuditEventType::BackupOperation,
                _ => continue, // Skip unknown event types
            };

            let additional_data = row
                .try_get::<Option<serde_json::Value>, _>("additional_data")?
                .and_then(|v| serde_json::from_value(v).ok());

            let entry = AuditLogEntry {
                timestamp: row.try_get("timestamp")?,
                event_type,
                user_id: row.try_get("user_id")?,
                tenant_id: row.try_get("tenant_id")?,
                api_key_id: row.try_get("api_key_id")?,
                ip_address: row.try_get("ip_address")?,
                user_agent: row.try_get("user_agent")?,
                method: row.try_get("method")?,
                path: row.try_get("path")?,
                query: row.try_get("query")?,
                status_code: row
                    .try_get::<Option<i32>, _>("status_code")?
                    .map(|c| c as u16),
                response_time_ms: row
                    .try_get::<Option<i64>, _>("response_time_ms")?
                    .map(|t| t as u128),
                error_message: row.try_get("error_message")?,
                additional_data,
            };

            entries.push(entry);
        }

        Ok(entries)
    }

    async fn count(&self, filter: AuditQueryFilter) -> Result<i64, AuditRepositoryError> {
        let mut query_builder =
            sqlx::QueryBuilder::new("SELECT COUNT(*) as count FROM audit_logs WHERE 1=1");

        // Apply same filters as query method
        if let Some(event_types) = &filter.event_types {
            if !event_types.is_empty() {
                query_builder.push(" AND event_type = ANY(");
                query_builder.push_bind(event_types.as_slice());
                query_builder.push(")");
            }
        }

        if let Some(user_id) = &filter.user_id {
            query_builder.push(" AND user_id = ");
            query_builder.push_bind(user_id);
        }

        if let Some(tenant_id) = &filter.tenant_id {
            query_builder.push(" AND tenant_id = ");
            query_builder.push_bind(tenant_id);
        }

        if let Some(api_key_id) = &filter.api_key_id {
            query_builder.push(" AND api_key_id = ");
            query_builder.push_bind(api_key_id);
        }

        if let Some(ip_address) = &filter.ip_address {
            query_builder.push(" AND ip_address = ");
            query_builder.push_bind(ip_address.clone());
        }

        if let Some(path_pattern) = &filter.path_pattern {
            query_builder.push(" AND path LIKE ");
            query_builder.push_bind(format!("%{}%", path_pattern));
        }

        if let Some(min_code) = filter.status_code_min {
            query_builder.push(" AND status_code >= ");
            query_builder.push_bind(min_code);
        }

        if let Some(max_code) = filter.status_code_max {
            query_builder.push(" AND status_code <= ");
            query_builder.push_bind(max_code);
        }

        if let Some(from_ts) = filter.from_timestamp {
            query_builder.push(" AND timestamp >= ");
            query_builder.push_bind(from_ts);
        }

        if let Some(to_ts) = filter.to_timestamp {
            query_builder.push(" AND timestamp <= ");
            query_builder.push_bind(to_ts);
        }

        if let Some(has_error) = filter.has_error {
            if has_error {
                query_builder.push(" AND error_message IS NOT NULL");
            } else {
                query_builder.push(" AND error_message IS NULL");
            }
        }

        let query = query_builder.build_query_as::<(i64,)>();

        let (count,) = query.fetch_one(&self.pool).await?;
        Ok(count)
    }

    async fn cleanup_old_logs(&self, retention_days: i32) -> Result<i64, AuditRepositoryError> {
        let result =
            sqlx::query("DELETE FROM audit_logs WHERE timestamp < NOW() - INTERVAL '1 day' * $1")
                .bind(retention_days)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() as i64)
    }
}
