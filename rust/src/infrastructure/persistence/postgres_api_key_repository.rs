use async_trait::async_trait;
use sqlx::{PgPool, Row};

use crate::application::ports::{ApiKeyRepository, ApiKeyRepositoryError};
use crate::domain::{
    entities::{ApiKey, ApiKeyDbData},
    value_objects::{ApiKeyId, ApiKeyPermissions, ApiKeyValue},
};

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
        sqlx::query(
            r#"
            INSERT INTO api_keys (
                id, api_key, tenant_id, name, description,
                permissions, is_active, expires_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(api_key.id().as_uuid())
        .bind(api_key.api_key().as_str())
        .bind(api_key.tenant_id())
        .bind(api_key.name())
        .bind(api_key.description())
        .bind(serde_json::to_value(api_key.permissions())?)
        .bind(api_key.is_active())
        .bind(api_key.expires_at().cloned())
        .bind(*api_key.created_at())
        .bind(*api_key.updated_at())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &ApiKeyId) -> Result<Option<ApiKey>, ApiKeyRepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT
                id, api_key, tenant_id, name, description,
                permissions, is_active, expires_at, 
                created_at, updated_at, last_used_at
            FROM api_keys
            WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let permissions: ApiKeyPermissions =
                    serde_json::from_value(row.try_get("permissions")?)?;
                let api_key_value = ApiKeyValue::from_string(row.try_get("api_key")?);

                let db_data = ApiKeyDbData {
                    id: ApiKeyId::from_uuid(row.try_get("id")?),
                    api_key: api_key_value,
                    tenant_id: row.try_get("tenant_id")?,
                    name: row.try_get("name")?,
                    description: row.try_get("description")?,
                    permissions,
                    is_active: row.try_get("is_active")?,
                    expires_at: row.try_get("expires_at")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                    last_used_at: row.try_get("last_used_at")?,
                };

                let api_key = ApiKey::from_db(db_data);

                Ok(Some(api_key))
            }
            None => Ok(None),
        }
    }

    async fn find_by_key(&self, key: &str) -> Result<Option<ApiKey>, ApiKeyRepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT
                id, api_key, tenant_id, name, description,
                permissions, is_active, expires_at, 
                created_at, updated_at, last_used_at
            FROM api_keys
            WHERE api_key = $1 AND is_active = true
            "#,
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let permissions: ApiKeyPermissions =
                    serde_json::from_value(row.try_get("permissions")?)?;
                let api_key_value = ApiKeyValue::from_string(row.try_get("api_key")?);

                let db_data = ApiKeyDbData {
                    id: ApiKeyId::from_uuid(row.try_get("id")?),
                    api_key: api_key_value,
                    tenant_id: row.try_get("tenant_id")?,
                    name: row.try_get("name")?,
                    description: row.try_get("description")?,
                    permissions,
                    is_active: row.try_get("is_active")?,
                    expires_at: row.try_get("expires_at")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                    last_used_at: row.try_get("last_used_at")?,
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
        let rows = sqlx::query(
            r#"
            SELECT
                id, api_key, tenant_id, name, description,
                permissions, is_active, expires_at, 
                created_at, updated_at, last_used_at
            FROM api_keys
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut api_keys = Vec::new();
        for row in rows {
            let permissions: ApiKeyPermissions =
                serde_json::from_value(row.try_get("permissions")?)?;
            let api_key_value = ApiKeyValue::from_string(row.try_get("api_key")?);

            let db_data = ApiKeyDbData {
                id: ApiKeyId::from_uuid(row.try_get("id")?),
                api_key: api_key_value,
                tenant_id: row.try_get("tenant_id")?,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                permissions,
                is_active: row.try_get("is_active")?,
                expires_at: row.try_get("expires_at")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                last_used_at: row.try_get("last_used_at")?,
            };

            api_keys.push(ApiKey::from_db(db_data));
        }

        Ok(api_keys)
    }

    async fn count_by_tenant(&self, tenant_id: &str) -> Result<i64, ApiKeyRepositoryError> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*)
            FROM api_keys
            WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = row.try_get(0)?;
        Ok(count)
    }

    async fn update(&self, api_key: &ApiKey) -> Result<(), ApiKeyRepositoryError> {
        sqlx::query(
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
        )
        .bind(api_key.id().as_uuid())
        .bind(api_key.name())
        .bind(api_key.description())
        .bind(serde_json::to_value(api_key.permissions())?)
        .bind(api_key.is_active())
        .bind(api_key.expires_at().cloned())
        .bind(*api_key.updated_at())
        .bind(api_key.last_used_at().cloned())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, id: &ApiKeyId) -> Result<(), ApiKeyRepositoryError> {
        let result = sqlx::query(
            r#"
            DELETE FROM api_keys
            WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ApiKeyRepositoryError::NotFound(id.to_string()));
        }

        Ok(())
    }

    async fn mark_used(&self, id: &ApiKeyId) -> Result<(), ApiKeyRepositoryError> {
        sqlx::query(
            r#"
            UPDATE api_keys
            SET last_used_at = now()
            WHERE id = $1 AND is_active = true
            "#,
        )
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<i64, ApiKeyRepositoryError> {
        let result = sqlx::query(
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
