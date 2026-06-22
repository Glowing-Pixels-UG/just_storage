use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::application::ports::StorageError;
use crate::domain::value_objects::ContentHash;

/// Buffer size for I/O operations. 256KB provides optimal throughput
/// for most modern storage systems while balancing memory usage.
const BUFFER_SIZE: usize = 256 * 1024;

/// Utility for computing SHA-256 content hashes.
///
/// # Design Decision: SHA-256 for Content-Addressable Storage
///
/// This implementation uses SHA-256 exclusively for content addressing:
///
/// 1. **Industry Standard**: SHA-256 is the de facto standard for CAS systems
///    (Git, IPFS, Docker, etc.), ensuring compatibility and interoperability.
///
/// 2. **Cryptographic Security**: SHA-256 provides strong collision resistance
///    (2^128 security level), which is critical for content integrity.
///
/// 3. **Fixed Format**: ContentHash is designed around SHA-256's 32-byte output
///    (64 hex characters), which enables efficient directory fan-out strategies.
///
/// 4. **Performance**: With SIMD optimizations enabled via the `asm` feature,
///    SHA-256 performance is excellent on modern CPUs while maintaining
///    cryptographic guarantees.
///
/// 5. **Consistency**: Using a single hash algorithm ensures all content hashes
///    are comparable and prevents hash collisions between different algorithms.
///
/// # Performance Optimizations
///
/// - **SIMD Acceleration**: Enabled via `sha2` crate's `asm` feature for
///   hardware-accelerated hash computation on x86_64 and ARM64.
/// - **Large Buffers**: 256KB buffers optimize I/O throughput for sequential
///   operations while streaming data.
/// - **Streaming Hash**: Hash computation happens simultaneously with file I/O,
///   eliminating the need for a second pass over the data.
pub struct ContentHasher;

impl ContentHasher {
    /// Write stream to file and compute SHA-256 hash simultaneously.
    ///
    /// This method performs both operations in a single pass for optimal performance.
    /// The hash is computed while streaming data to disk, eliminating the need for
    /// a second read pass.
    ///
    /// # Arguments
    ///
    /// * `dest_path` - Path where the file will be written
    /// * `reader` - Async reader providing the data to hash and write
    ///
    /// # Returns
    ///
    /// Tuple of (ContentHash, size_bytes) where:
    /// - `ContentHash`: SHA-256 hash of the content (64 hex characters)
    /// - `size_bytes`: Total number of bytes written
    pub async fn write_and_hash(
        dest_path: &Path,
        reader: impl AsyncRead + Unpin,
    ) -> Result<(ContentHash, u64), StorageError> {
        Self::write_and_hash_with_durability(dest_path, reader, true).await
    }

    /// Write stream to file and compute SHA-256 hash with durability control.
    ///
    /// This method allows controlling whether to perform expensive `fsync()` operations
    /// for durability guarantees. For benchmarking or when durability is handled
    /// at a higher level, set `durable` to `false` for better performance.
    ///
    /// # Arguments
    ///
    /// * `dest_path` - Path where the file will be written
    /// * `reader` - Async reader providing the data to hash and write
    /// * `durable` - If `true`, performs `fsync()` to ensure data is on disk
    ///
    /// # Returns
    ///
    /// Tuple of (ContentHash, size_bytes) where:
    /// - `ContentHash`: SHA-256 hash of the content (64 hex characters)
    /// - `size_bytes`: Total number of bytes written
    pub async fn write_and_hash_with_durability(
        dest_path: &Path,
        reader: impl AsyncRead + Unpin,
        durable: bool,
    ) -> Result<(ContentHash, u64), StorageError> {
        Self::write_and_hash_with_durability_adaptive(dest_path, reader, durable, true).await
    }

    /// Write and hash with optional adaptive buffering control
    pub async fn write_and_hash_with_durability_adaptive(
        dest_path: &Path,
        reader: impl AsyncRead + Unpin,
        durable: bool,
        use_adaptive_buffering: bool,
    ) -> Result<(ContentHash, u64), StorageError> {
        // REGULAR PATH: Adaptive buffering for larger files (or when disabled)
        if use_adaptive_buffering {
            Self::write_and_hash_adaptive(dest_path, reader, durable).await
        } else {
            Self::write_and_hash_simple(dest_path, reader, durable).await
        }
    }

    /// Regular path with adaptive buffering for larger files (now redirects to simple path)
    async fn write_and_hash_adaptive(
        dest_path: &Path,
        reader: impl AsyncRead + Unpin,
        durable: bool,
    ) -> Result<(ContentHash, u64), StorageError> {
        Self::write_and_hash_simple(dest_path, reader, durable).await
    }

    /// Write a stream to `dest_path` and compute its SHA-256 hash in a single
    /// streaming pass.
    ///
    /// Data is written through a buffered writer while the SHA-256 digest is
    /// updated simultaneously, then flushed. When `durable` is `true`, the file
    /// is `fsync()`ed via `sync_all()` so the bytes are guaranteed on disk
    /// before returning.
    async fn write_and_hash_simple(
        dest_path: &Path,
        mut reader: impl AsyncRead + Unpin,
        durable: bool,
    ) -> Result<(ContentHash, u64), StorageError> {
        let file = File::create(dest_path).await?;
        let mut file = tokio::io::BufWriter::with_capacity(BUFFER_SIZE * 2, file);

        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; BUFFER_SIZE];
        let mut total_bytes = 0u64;

        // Stream data: hash and write simultaneously
        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
            file.write_all(&buffer[..n]).await?;
            total_bytes += n as u64;
        }

        file.flush().await?;

        if durable {
            file.get_mut().sync_all().await?;
        }

        let hash_bytes = hasher.finalize();
        let hash_hex = hex::encode(hash_bytes);
        let content_hash =
            ContentHash::from_hex(hash_hex).map_err(|e| StorageError::Internal(e.to_string()))?;

        Ok((content_hash, total_bytes))
    }

    /// Compute SHA-256 hash of an existing file.
    ///
    /// This method reads the file and computes its hash. For new content,
    /// prefer `write_and_hash()` which performs both operations in a single pass.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to hash
    ///
    /// # Returns
    ///
    /// ContentHash representing the SHA-256 hash of the file (64 hex characters)
    pub async fn hash_file(path: &Path) -> Result<ContentHash, StorageError> {
        let file = File::open(path).await?;
        let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
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
