use async_trait::async_trait;
use sqlx::PgPool;

use crate::application::ports::{BlobRepository, RepositoryError};
use crate::domain::entities::Blob;
use crate::domain::value_objects::{ContentHash, StorageClass};

pub struct PostgresBlobRepository {
    pool: PgPool,
}

impl PostgresBlobRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BlobRepository for PostgresBlobRepository {
    async fn get_or_create(
        &self,
        content_hash: &ContentHash,
        storage_class: StorageClass,
        size_bytes: u64,
    ) -> Result<Blob, RepositoryError> {
        let hash = content_hash.as_hex();
        let class = storage_class.to_string();
        let size = size_bytes as i64;

        // Try insert, on conflict do nothing and select
        let row = sqlx::query_as::<_, BlobRow>(
            r#"
            INSERT INTO blobs (content_hash, storage_class, size_bytes, ref_count)
            VALUES ($1, $2, $3, 1)
            ON CONFLICT (content_hash) DO UPDATE SET ref_count = blobs.ref_count + 1, last_used_at = now()
            RETURNING content_hash, storage_class, size_bytes, ref_count, created_at
            "#,
        )
        .bind(hash)
        .bind(class)
        .bind(size)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into_domain())
    }

    async fn increment_ref(&self, content_hash: &ContentHash) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            UPDATE blobs
            SET ref_count = ref_count + 1
            WHERE content_hash = $1
            "#,
        )
        .bind(content_hash.as_hex())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn decrement_ref(&self, content_hash: &ContentHash) -> Result<i32, RepositoryError> {
        let row = sqlx::query_as::<_, (i64,)>(
            r#"
            UPDATE blobs
            SET ref_count = GREATEST(ref_count - 1, 0)
            WHERE content_hash = $1
            RETURNING ref_count
            "#,
        )
        .bind(content_hash.as_hex())
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0 as i32)
    }

    async fn find_orphaned(&self, limit: i64) -> Result<Vec<Blob>, RepositoryError> {
        let rows = sqlx::query_as::<_, BlobRow>(
            r#"
            SELECT content_hash, storage_class, size_bytes, ref_count, created_at
            FROM blobs
            WHERE ref_count = 0
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_domain()).collect())
    }

    async fn delete(&self, content_hash: &ContentHash) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM blobs WHERE content_hash = $1")
            .bind(content_hash.as_hex())
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct BlobRow {
    content_hash: String,
    storage_class: String,
    size_bytes: i64,
    ref_count: i64,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl BlobRow {
    fn into_domain(self) -> Blob {
        let content_hash =
            ContentHash::from_hex(self.content_hash).unwrap_or_else(|_| ContentHash::default());
        let storage_class = self
            .storage_class
            .parse::<StorageClass>()
            .unwrap_or(StorageClass::Hot);

        Blob::reconstruct(
            content_hash,
            storage_class,
            self.size_bytes as u64,
            self.ref_count as i32,
            self.created_at,
        )
    }
}
