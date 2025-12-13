use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, BufReader};

use crate::application::ports::StorageError;
use crate::domain::value_objects::ContentHash;

const BUFFER_SIZE: usize = 64 * 1024; // 64KB buffer

/// Utility for computing SHA-256 hashes
pub struct ContentHasher;

impl ContentHasher {
    /// Write stream to file and compute hash simultaneously
    pub async fn write_and_hash(
        dest_path: &Path,
        mut reader: impl AsyncRead + Unpin,
    ) -> Result<(ContentHash, u64), StorageError> {
        // Open temp file for writing
        let mut file = tokio::io::BufWriter::with_capacity(BUFFER_SIZE, File::create(dest_path).await?);

        // Hash while writing
        let mut hasher = Sha256::new();
        let mut total_bytes = 0u64;
        let mut buffer = vec![0u8; BUFFER_SIZE];

        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 {
                break;
            }

            // Update hash
            hasher.update(&buffer[..n]);

            // Write to file
            tokio::io::AsyncWriteExt::write_all(&mut file, &buffer[..n]).await?;

            total_bytes += n as u64;
        }

        // Flush buffer
        tokio::io::AsyncWriteExt::flush(&mut file).await?;

        // Ensure data is fsynced to disk
        file.get_mut().sync_all().await?;

        // Finalize hash
        let hash_bytes = hasher.finalize();
        let hash_hex = hex::encode(hash_bytes);
        let content_hash =
            ContentHash::from_hex(hash_hex).map_err(|e| StorageError::Internal(e.to_string()))?;

        Ok((content_hash, total_bytes))
    }

    /// Compute hash of existing file
    pub async fn hash_file(path: &Path) -> Result<ContentHash, StorageError> {
        let file = File::open(path).await?;
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; BUFFER_SIZE];

        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        let hash_bytes = hasher.finalize();
        let hash_hex = hex::encode(hash_bytes);
        ContentHash::from_hex(hash_hex).map_err(|e| StorageError::Internal(e.to_string()))
    }
}
