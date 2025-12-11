use async_trait::async_trait;
use sqlx::PgPool;

use crate::application::ports::{ObjectRepository, RepositoryError};
use crate::domain::entities::Object;
use crate::domain::value_objects::{
    ContentHash, Namespace, ObjectId, ObjectMetadata, ObjectStatus, StorageClass, TenantId,
};

pub struct PostgresObjectRepository {
    pool: PgPool,
}

impl PostgresObjectRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ObjectRepository for PostgresObjectRepository {
    async fn save(&self, object: &Object) -> Result<(), RepositoryError> {
        let id = object.id().as_uuid();
        let namespace = object.namespace().as_str();
        let tenant_id = object.tenant_id().as_uuid();
        let key = object.key();
        let status = object.status().to_string();
        let storage_class = object.storage_class().to_string();
        let content_hash = object.content_hash().map(|h| h.as_hex().to_string());
        let size_bytes = object.size_bytes().map(|s| s as i64);
        let content_type = object.content_type();
        let metadata = object
            .metadata()
            .to_json()
            .map_err(|e| RepositoryError::SerializationError(e.to_string()))?;
        let created_at = object.created_at();
        let updated_at = object.updated_at();

        sqlx::query(
            r#"
            INSERT INTO objects (
                id, namespace, tenant_id, key, status, storage_class,
                content_hash, size_bytes, content_type, metadata,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (id) DO UPDATE SET
                status = EXCLUDED.status,
                content_hash = EXCLUDED.content_hash,
                size_bytes = EXCLUDED.size_bytes,
                content_type = EXCLUDED.content_type,
                metadata = EXCLUDED.metadata,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(id)
        .bind(namespace)
        .bind(tenant_id)
        .bind(key)
        .bind(status)
        .bind(storage_class)
        .bind(content_hash)
        .bind(size_bytes)
        .bind(content_type)
        .bind(metadata)
        .bind(created_at)
        .bind(updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &ObjectId) -> Result<Option<Object>, RepositoryError> {
        let row = sqlx::query_as::<_, ObjectRow>(
            r#"
            SELECT id, namespace, tenant_id, key, status, storage_class,
                   content_hash, size_bytes, content_type, metadata,
                   created_at, updated_at
            FROM objects
            WHERE id = $1 AND status = 'COMMITTED'
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(r.into_domain()?)),
            None => Ok(None),
        }
    }

    async fn find_by_key(
        &self,
        namespace: &Namespace,
        tenant_id: &TenantId,
        key: &str,
    ) -> Result<Option<Object>, RepositoryError> {
        let row = sqlx::query_as::<_, ObjectRow>(
            r#"
            SELECT id, namespace, tenant_id, key, status, storage_class,
                   content_hash, size_bytes, content_type, metadata,
                   created_at, updated_at
            FROM objects
            WHERE namespace = $1 AND tenant_id = $2 AND key = $3 AND status = 'COMMITTED'
            "#,
        )
        .bind(namespace.as_str())
        .bind(tenant_id.as_uuid())
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(r.into_domain()?)),
            None => Ok(None),
        }
    }

    async fn list(
        &self,
        namespace: &Namespace,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Object>, RepositoryError> {
        let rows = sqlx::query_as::<_, ObjectRow>(
            r#"
            SELECT id, namespace, tenant_id, key, status, storage_class,
                   content_hash, size_bytes, created_at, updated_at
            FROM objects
            WHERE namespace = $1 AND tenant_id = $2 AND status = 'COMMITTED'
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(namespace.as_str())
        .bind(tenant_id.as_uuid())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        // `into_domain` returns a Result, so use iterator collect to gather results or return first error.
        rows.into_iter().map(|r| r.into_domain()).collect()
    }

    async fn delete(&self, id: &ObjectId) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM objects WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

// Internal row mapping struct
#[derive(sqlx::FromRow)]
struct ObjectRow {
    id: uuid::Uuid,
    namespace: String,
    tenant_id: uuid::Uuid,
    key: Option<String>,
    status: String,
    storage_class: String,
    content_hash: Option<String>,
    size_bytes: Option<i64>,
    content_type: Option<String>,
    metadata: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl ObjectRow {
    fn into_domain(self) -> Result<Object, RepositoryError> {
        // Parse namespace
        let namespace = Namespace::new(self.namespace)
            .map_err(|e| RepositoryError::SerializationError(e.to_string()))?;

        let tenant_id = TenantId::new(self.tenant_id);

        // Parse status and storage class with errors propagated
        let status = self
            .status
            .parse::<ObjectStatus>()
            .map_err(RepositoryError::SerializationError)?;

        let storage_class = self
            .storage_class
            .parse::<StorageClass>()
            .map_err(RepositoryError::SerializationError)?;

        // Parse optional content hash
        let content_hash = match self.content_hash {
            Some(h) => Some(
                ContentHash::from_hex(h)
                    .map_err(|e| RepositoryError::SerializationError(e.to_string()))?,
            ),
            None => None,
        };

        let metadata = ObjectMetadata::from_json(&self.metadata)
            .map_err(|e| RepositoryError::SerializationError(e.to_string()))?;

        Ok(Object::reconstruct(
            ObjectId::from_uuid(self.id),
            namespace,
            tenant_id,
            self.key,
            status,
            storage_class,
            content_hash,
            self.size_bytes.map(|s| s as u64),
            self.content_type,
            metadata,
            self.created_at,
            self.updated_at,
        ))
    }
}
