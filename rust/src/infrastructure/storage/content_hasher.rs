use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader};

use crate::application::ports::StorageError;
use crate::domain::value_objects::ContentHash;

/// Buffer size for I/O operations. 256KB provides optimal throughput
/// for most modern storage systems while balancing memory usage.
const BUFFER_SIZE: usize = 256 * 1024;

/// Minimum buffer size for small files to avoid excessive memory allocation
const MIN_BUFFER_SIZE: usize = 8 * 1024;

/// Tiny file threshold - files smaller than this bypass adaptive buffering
/// to avoid peek overhead for very small files
const TINY_FILE_THRESHOLD: usize = 512;

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

    /// Fast path for tiny files - uses already peeked data
    async fn write_and_hash_tiny_file_from_peek(
        _dest_path: &Path,
        file: File,
        mut reader: impl AsyncRead + Unpin,
        durable: bool,
        initial_data: &[u8],
    ) -> Result<(ContentHash, u64), StorageError> {
        let mut file = tokio::io::BufWriter::with_capacity(1024, file);

        let mut hasher = Sha256::new();
        let mut total_bytes = initial_data.len() as u64;

        // Process initial peeked data
        hasher.update(initial_data);
        file.write_all(initial_data).await?;

        // Read remaining data (should be minimal for tiny files)
        let mut buffer = vec![0u8; TINY_FILE_THRESHOLD];
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

        // Finalize hash: SHA-256 produces 32 bytes = 64 hex characters
        let hash_bytes = hasher.finalize();
        let hash_hex = hex::encode(hash_bytes);
        let content_hash =
            ContentHash::from_hex(hash_hex).map_err(|e| StorageError::Internal(e.to_string()))?;

        Ok((content_hash, total_bytes))
    }

    /// Simple path without adaptive buffering (fixed buffer size)
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

    /// Regular path with adaptive buffering for larger files
    async fn write_and_hash_adaptive(
        dest_path: &Path,
        mut reader: impl AsyncRead + Unpin,
        durable: bool,
    ) -> Result<(ContentHash, u64), StorageError> {
        // Open temp file for writing
        let file = File::create(dest_path).await?;

        // Try to peek at first chunk to estimate size for adaptive buffering
        // Use a small initial buffer to avoid over-allocation
        let mut peek_buffer = vec![0u8; 4096]; // Small initial peek
        let peek_size = reader.read(&mut peek_buffer).await?;

        // FAST PATH: If first read is tiny, use simplified path
        if peek_size > 0 && peek_size <= TINY_FILE_THRESHOLD {
            return Self::write_and_hash_tiny_file_from_peek(
                dest_path,
                file,
                reader,
                durable,
                &peek_buffer[..peek_size],
            )
            .await;
        }

        // Determine optimal buffer size based on initial read
        // For small files, use smaller buffers to reduce memory overhead
        let buffer_size = if peek_size == 0 {
            // Empty file
            1024
        } else if peek_size < 4096 {
            // Very small file, use minimal buffer
            peek_size.max(1024)
        } else if peek_size < 64 * 1024 {
            // Small-medium file, use moderate buffer
            MIN_BUFFER_SIZE
        } else {
            // Large file, use full buffer
            BUFFER_SIZE
        };

        // Create BufWriter with adaptive buffer capacity
        // Using 2x buffer size for BufWriter to minimize syscalls
        let mut file = tokio::io::BufWriter::with_capacity(buffer_size * 2, file);

        // Initialize SHA-256 hasher (with SIMD optimizations if available)
        let mut hasher = Sha256::new();
        let mut total_bytes = peek_size as u64;

        // Process initial peek data if any
        if peek_size > 0 {
            hasher.update(&peek_buffer[..peek_size]);
            file.write_all(&peek_buffer[..peek_size]).await?;
        }

        // Allocate buffer for remaining data
        let mut buffer = vec![0u8; buffer_size];

        // Stream data: hash and write simultaneously
        loop {
            let n = reader.read(&mut buffer).await?;
            if n == 0 {
                break;
            }

            // Update hash and write to file in one operation
            // This single-pass approach is more efficient than separate hash/write passes
            hasher.update(&buffer[..n]);
            file.write_all(&buffer[..n]).await?;

            total_bytes += n as u64;
        }

        // Flush buffered writes
        file.flush().await?;

        // Ensure data is fsynced to disk if durability is required
        // Note: fsync() is expensive but necessary for durability guarantees
        if durable {
            file.get_mut().sync_all().await?;
        }

        // Finalize hash: SHA-256 produces 32 bytes = 64 hex characters
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
