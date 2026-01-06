use async_trait::async_trait;
use sqlx::PgPool;

use crate::application::dto::{SearchRequest, TextSearchRequest};
use crate::application::ports::{ObjectRepository, RepositoryError};
use crate::domain::entities::Object;
use crate::domain::value_objects::{
    ContentHash, Namespace, ObjectId, ObjectMetadata, ObjectStatus, StorageClass, TenantId,
};
use crate::infrastructure::persistence::query_builder::QueryBuilder;

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
        let tenant_id = object.tenant_id().to_string();
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
        let sql = format!(
            "{} WHERE id = $1 AND status = 'COMMITTED'",
            QueryBuilder::OBJECT_SELECT
        );
        let row = sqlx::query_as::<_, ObjectRow>(&sql)
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
        let sql = format!(
            "{} {} AND key = $3",
            QueryBuilder::OBJECT_SELECT,
            QueryBuilder::namespace_tenant_where(namespace.as_str(), &tenant_id.to_string())
        );
        let row = sqlx::query_as::<_, ObjectRow>(&sql)
            .bind(namespace.as_str())
            .bind(tenant_id.to_string())
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
        let sql = format!(
            "{} {} ORDER BY created_at DESC LIMIT $3 OFFSET $4",
            QueryBuilder::OBJECT_SELECT,
            QueryBuilder::namespace_tenant_where(namespace.as_str(), &tenant_id.to_string())
        );
        let rows = sqlx::query_as::<_, ObjectRow>(&sql)
            .bind(namespace.as_str())
            .bind(tenant_id.to_string())
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        // `into_domain` returns a Result, so use iterator collect to gather results or return first error.
        rows.into_iter().map(|r| r.into_domain()).collect()
    }

    async fn search(&self, request: &SearchRequest) -> Result<Vec<Object>, RepositoryError> {
        use crate::application::dto::{SortDirection, SortField};

        let sort_field = request.sort_by.as_ref().unwrap_or(&SortField::CreatedAt);
        let sort_direction = request
            .sort_direction
            .as_ref()
            .unwrap_or(&SortDirection::Desc);
        let limit = request.limit.unwrap_or(100).min(1000);
        let offset = request.offset.unwrap_or(0);

        // Validate sort column to prevent SQL injection
        let sort_column = match sort_field {
            SortField::CreatedAt => "created_at",
            SortField::UpdatedAt => "updated_at",
            SortField::SizeBytes => "size_bytes",
            SortField::Key => "key",
            SortField::ContentType => "content_type",
        };
        let sort_dir = match sort_direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };

        // Use parameterized query with validated column names
        // Note: SQLx doesn't support dynamic column names in ORDER BY, so we validate them above
        let sql = format!(
            "{} {} ORDER BY {} {} LIMIT $3 OFFSET $4",
            QueryBuilder::OBJECT_SELECT,
            QueryBuilder::namespace_tenant_where(&request.namespace, &request.tenant_id),
            sort_column,
            sort_dir
        );

        // Use query_as with validated SQL (column names are validated above, not user input)
        let rows: Vec<ObjectRow> = sqlx::query_as::<_, ObjectRow>(&sql)
            .bind(&request.namespace)
            .bind(&request.tenant_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(|r| r.into_domain()).collect()
    }

    async fn text_search(
        &self,
        request: &TextSearchRequest,
    ) -> Result<Vec<Object>, RepositoryError> {
        let search_in_metadata = request.search_in_metadata.unwrap_or(true);
        let search_in_key = request.search_in_key.unwrap_or(true);
        let limit = request.limit.unwrap_or(100).min(1000);
        let offset = request.offset.unwrap_or(0);

        // Build query with proper parameterization to prevent SQL injection
        // Use conditional WHERE clauses based on search options
        let rows: Vec<ObjectRow> = if search_in_key && search_in_metadata {
            // Search in both key and metadata
            let sql = format!(
                "{} {} AND (key ILIKE $3 OR metadata::text ILIKE $4) ORDER BY created_at DESC LIMIT $5 OFFSET $6",
                QueryBuilder::OBJECT_SELECT,
                QueryBuilder::namespace_tenant_where(&request.namespace, &request.tenant_id)
            );
            sqlx::query_as::<_, ObjectRow>(&sql)
                .bind(&request.namespace)
                .bind(&request.tenant_id)
                .bind(format!("%{}%", request.query))
                .bind(format!("%{}%", request.query))
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        } else if search_in_key {
            // Search only in key
            let sql = format!(
                "{} {} AND key ILIKE $3 ORDER BY created_at DESC LIMIT $4 OFFSET $5",
                QueryBuilder::OBJECT_SELECT,
                QueryBuilder::namespace_tenant_where(&request.namespace, &request.tenant_id)
            );
            sqlx::query_as::<_, ObjectRow>(&sql)
                .bind(&request.namespace)
                .bind(&request.tenant_id)
                .bind(format!("%{}%", request.query))
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        } else if search_in_metadata {
            // Search only in metadata
            let sql = format!(
                "{} {} AND metadata::text ILIKE $3 ORDER BY created_at DESC LIMIT $4 OFFSET $5",
                QueryBuilder::OBJECT_SELECT,
                QueryBuilder::namespace_tenant_where(&request.namespace, &request.tenant_id)
            );
            sqlx::query_as::<_, ObjectRow>(&sql)
                .bind(&request.namespace)
                .bind(&request.tenant_id)
                .bind(format!("%{}%", request.query))
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        } else {
            // No search conditions = no results (but still valid query)
            let sql = format!(
                "{} {} AND FALSE ORDER BY created_at DESC LIMIT $3 OFFSET $4",
                QueryBuilder::OBJECT_SELECT,
                QueryBuilder::namespace_tenant_where(&request.namespace, &request.tenant_id)
            );
            sqlx::query_as::<_, ObjectRow>(&sql)
                .bind(&request.namespace)
                .bind(&request.tenant_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
        };
        rows.into_iter().map(|r| r.into_domain()).collect()
    }

    async fn delete(&self, id: &ObjectId) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM objects WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn find_stuck_writing_objects(
        &self,
        age_hours: i64,
        limit: i64,
    ) -> Result<Vec<ObjectId>, RepositoryError> {
        #[derive(sqlx::FromRow)]
        struct StuckObjectRow {
            id: uuid::Uuid,
        }

        let rows = sqlx::query_as::<_, StuckObjectRow>(
            r#"
            SELECT id
            FROM objects
            WHERE status = 'WRITING'
              AND created_at < now() - ($1 || ' hours')::interval
            ORDER BY created_at ASC
            LIMIT $2
            "#,
        )
        .bind(age_hours)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ObjectId::from_uuid(row.id))
            .collect())
    }

    async fn cleanup_stuck_uploads(&self, age_hours: i64) -> Result<usize, RepositoryError> {
        // Use the database function for atomic cleanup
        let result: (i64,) = sqlx::query_as("SELECT cleanup_stuck_uploads($1) as deleted_count")
            .bind(age_hours)
            .fetch_one(&self.pool)
            .await?;

        Ok(result.0 as usize)
    }
}

// Internal row mapping struct
#[derive(sqlx::FromRow)]
struct ObjectRow {
    id: uuid::Uuid,
    namespace: String,
    tenant_id: String,
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

        let tenant_id = TenantId::from_string(&self.tenant_id).map_err(|e| {
            RepositoryError::SerializationError(format!("Invalid tenant_id: {}", e))
        })?;

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
