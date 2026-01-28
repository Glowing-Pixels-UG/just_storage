use async_trait::async_trait;
use tower_sessions::{session::{Id, Record}, SessionStore};
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use sqlx::PgPool;
use time::OffsetDateTime;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<SessionError> for tower_sessions::session_store::Error {
    fn from(e: SessionError) -> Self {
        tower_sessions::session_store::Error::Backend(e.to_string())
    }
}

/// A session store that encrypts session data at rest in PostgreSQL
#[derive(Clone, Debug)]
pub struct EncryptedPostgresStore {
    pool: PgPool,
    key: [u8; 32],
}

impl EncryptedPostgresStore {
    pub fn new(pool: PgPool, key: [u8; 32]) -> Self {
        Self { pool, key }
    }

    /// Delete expired sessions from the database
    pub async fn delete_expired(&self) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM tower_sessions.session WHERE expiry_date <= $1")
            .bind(OffsetDateTime::now_utc())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, SessionError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| SessionError::Encryption(e.to_string()))?;
        
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from(nonce_bytes);

        let ciphertext = cipher
            .encrypt(&nonce, Payload { msg: data, aad: &[] })
            .map_err(|e| SessionError::Encryption(e.to_string()))?;

        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, SessionError> {
        if data.len() < 12 {
            return Err(SessionError::Encryption("Invalid encrypted data".into()));
        }

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| SessionError::Encryption(e.to_string()))?;
        
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = <&[u8; 12]>::try_from(nonce_bytes)
            .map_err(|_| SessionError::Encryption("Invalid nonce length".into()))?;

        let plaintext = cipher
            .decrypt(nonce.into(), Payload { msg: ciphertext, aad: &[] })
            .map_err(|e| SessionError::Encryption(e.to_string()))?;

        Ok(plaintext)
    }
}

#[async_trait]
impl SessionStore for EncryptedPostgresStore {
    async fn load(&self, session_id: &Id) -> tower_sessions::session_store::Result<Option<Record>> {
        let row: Option<(Vec<u8>, OffsetDateTime)> = sqlx::query_as(
            "SELECT data, expiry_date FROM tower_sessions.session WHERE id = $1 AND expiry_date > $2"
        )
        .bind(session_id.to_string())
        .bind(OffsetDateTime::now_utc())
        .fetch_optional(&self.pool)
        .await
        .map_err(SessionError::Sqlx)?;

        match row {
            Some((encrypted_data, expiry_date)) => {
                let decrypted_data = self.decrypt(&encrypted_data).map_err(|e| {
                    tracing::error!("Failed to decrypt session data: {}", e);
                    e
                })?;
                
                let data: HashMap<String, serde_json::Value> = serde_json::from_slice(&decrypted_data)
                    .map_err(SessionError::Serialization)?;

                Ok(Some(Record {
                    id: *session_id,
                    data,
                    expiry_date,
                }))
            }
            None => Ok(None),
        }
    }

    async fn save(&self, record: &Record) -> tower_sessions::session_store::Result<()> {
        let data_json = serde_json::to_vec(&record.data).map_err(SessionError::Serialization)?;
        let encrypted_data = self.encrypt(&data_json)?;

        sqlx::query(
            r#"
            INSERT INTO tower_sessions.session (id, data, expiry_date)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE
            SET data = EXCLUDED.data, expiry_date = EXCLUDED.expiry_date
            "#
        )
        .bind(record.id.to_string())
        .bind(encrypted_data)
        .bind(record.expiry_date)
        .execute(&self.pool)
        .await
        .map_err(SessionError::Sqlx)?;

        Ok(())
    }

    async fn delete(&self, session_id: &Id) -> tower_sessions::session_store::Result<()> {
        sqlx::query("DELETE FROM tower_sessions.session WHERE id = $1")
            .bind(session_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(SessionError::Sqlx)?;

        Ok(())
    }
}
