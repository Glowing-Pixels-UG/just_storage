use async_trait::async_trait;
use sqlx::PgPool;
use time::OffsetDateTime;
use chrono::{DateTime, Utc};

use crate::application::ports::{ApiKeyRepository, ApiKeyRepositoryError};
use crate::domain::{
    entities::{ApiKey, ApiKeyDbData},
    value_objects::{ApiKeyId, ApiKeyPermissions, ApiKeyValue},
};

fn to_chrono(odt: OffsetDateTime) -> DateTime<Utc> {
    DateTime::from_timestamp(odt.unix_timestamp(), 0).unwrap_or_default()
}

fn to_offset(dt: DateTime<Utc>) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(dt.timestamp()).unwrap()
}

/// PostgreSQL implementation of API key repository
pub struct PostgresApiKeyRepository {
    pool: PgPool,
}

impl PostgresApiKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ApiKeyRepository for PostgresApiKeyRepository {
    async fn create(&self, api_key: ApiKey) -> Result<(), ApiKeyRepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO api_keys (
                id, api_key, tenant_id, name, description,
                permissions, is_active, expires_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            api_key.id().as_uuid(),
            api_key.api_key().as_str(),
            api_key.tenant_id(),
            api_key.name(),
            api_key.description(),
            serde_json::to_value(api_key.permissions())?,
            api_key.is_active(),
            api_key.expires_at().cloned().map(to_offset),
            to_offset(*api_key.created_at()),
            to_offset(*api_key.updated_at()),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &ApiKeyId) -> Result<Option<ApiKey>, ApiKeyRepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT
                id, api_key, tenant_id, name, description,
                permissions, is_active, expires_at as "expires_at: OffsetDateTime", 
                created_at as "created_at: OffsetDateTime", 
                updated_at as "updated_at: OffsetDateTime", 
                last_used_at as "last_used_at: OffsetDateTime"
            FROM api_keys
            WHERE id = $1
            "#,
            id.as_uuid(),
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let permissions: ApiKeyPermissions =
                    serde_json::from_value(row.permissions.unwrap_or(serde_json::Value::Null))?;
                let api_key_value = ApiKeyValue::from_string(row.api_key);

                let db_data = ApiKeyDbData {
                    id: ApiKeyId::from_uuid(row.id),
                    api_key: api_key_value,
                    tenant_id: row.tenant_id,
                    name: row.name,
                    description: row.description,
                    permissions,
                    is_active: row.is_active,
                    expires_at: row.expires_at.map(to_chrono),
                    created_at: to_chrono(row.created_at),
                    updated_at: to_chrono(row.updated_at),
                    last_used_at: row.last_used_at.map(to_chrono),
                };

                let api_key = ApiKey::from_db(db_data);

                Ok(Some(api_key))
            }
            None => Ok(None),
        }
    }

    async fn find_by_key(&self, key: &str) -> Result<Option<ApiKey>, ApiKeyRepositoryError> {
        let row = sqlx::query!(
            r#"
            SELECT
                id, api_key, tenant_id, name, description,
                permissions, is_active, expires_at as "expires_at: OffsetDateTime", 
                created_at as "created_at: OffsetDateTime", 
                updated_at as "updated_at: OffsetDateTime", 
                last_used_at as "last_used_at: OffsetDateTime"
            FROM api_keys
            WHERE api_key = $1
            "#,
            key,
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let permissions: ApiKeyPermissions =
                    serde_json::from_value(row.permissions.unwrap_or(serde_json::Value::Null))?;
                let api_key_value = ApiKeyValue::from_string(row.api_key);

                let db_data = ApiKeyDbData {
                    id: ApiKeyId::from_uuid(row.id),
                    api_key: api_key_value,
                    tenant_id: row.tenant_id,
                    name: row.name,
                    description: row.description,
                    permissions,
                    is_active: row.is_active,
                    expires_at: row.expires_at.map(to_chrono),
                    created_at: to_chrono(row.created_at),
                    updated_at: to_chrono(row.updated_at),
                    last_used_at: row.last_used_at.map(to_chrono),
                };

                let api_key = ApiKey::from_db(db_data);

                Ok(Some(api_key))
            }
            None => Ok(None),
        }
    }

    async fn list_by_tenant(
        &self,
        tenant_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ApiKey>, ApiKeyRepositoryError> {
        let rows = sqlx::query!(
            r#"
            SELECT
                id, api_key, tenant_id, name, description,
                permissions, is_active, expires_at as "expires_at: OffsetDateTime", 
                created_at as "created_at: OffsetDateTime", 
                updated_at as "updated_at: OffsetDateTime", 
                last_used_at as "last_used_at: OffsetDateTime"
            FROM api_keys
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            tenant_id,
            limit,
            offset,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut api_keys = Vec::new();
        for row in rows {
            let permissions: ApiKeyPermissions =
                serde_json::from_value(row.permissions.unwrap_or(serde_json::Value::Null))?;
            let api_key_value = ApiKeyValue::from_string(row.api_key);

            let db_data = ApiKeyDbData {
                id: ApiKeyId::from_uuid(row.id),
                api_key: api_key_value,
                tenant_id: row.tenant_id,
                name: row.name,
                description: row.description,
                permissions,
                is_active: row.is_active,
                expires_at: row.expires_at.map(to_chrono),
                created_at: to_chrono(row.created_at),
                updated_at: to_chrono(row.updated_at),
                last_used_at: row.last_used_at.map(to_chrono),
            };

            let api_key = ApiKey::from_db(db_data);
            api_keys.push(api_key);
        }

        Ok(api_keys)
    }

    async fn count_by_tenant(&self, tenant_id: &str) -> Result<i64, ApiKeyRepositoryError> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM api_keys
            WHERE tenant_id = $1
            "#,
            tenant_id,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    async fn update(&self, api_key: &ApiKey) -> Result<(), ApiKeyRepositoryError> {
        sqlx::query!(
            r#"
            UPDATE api_keys
            SET
                name = $2,
                description = $3,
                permissions = $4,
                is_active = $5,
                expires_at = $6,
                updated_at = $7,
                last_used_at = $8
            WHERE id = $1
            "#,
            api_key.id().as_uuid(),
            api_key.name(),
            api_key.description(),
            serde_json::to_value(api_key.permissions())?,
            api_key.is_active(),
            api_key.expires_at().cloned().map(to_offset),
            to_offset(*api_key.updated_at()),
            api_key.last_used_at().cloned().map(to_offset),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, id: &ApiKeyId) -> Result<(), ApiKeyRepositoryError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM api_keys
            WHERE id = $1
            "#,
            id.as_uuid(),
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ApiKeyRepositoryError::NotFound(id.to_string()));
        }

        Ok(())
    }

    async fn mark_used(&self, id: &ApiKeyId) -> Result<(), ApiKeyRepositoryError> {
        sqlx::query!(
            r#"
            UPDATE api_keys
            SET last_used_at = now()
            WHERE id = $1 AND is_active = true
            "#,
            id.as_uuid(),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<i64, ApiKeyRepositoryError> {
        let result = sqlx::query!(
            r#"
            DELETE FROM api_keys
            WHERE expires_at < now() AND is_active = true
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }
}
