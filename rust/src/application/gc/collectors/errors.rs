use std::fmt;
use thiserror::Error;

/// Errors that can occur during garbage collection operations
#[derive(Debug, Error)]
pub enum GcError {
    /// Error occurred while querying for items to collect
    #[error("Failed to query for garbage collection candidates: {source}")]
    QueryError {
        #[from]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Error occurred while deleting items during collection
    #[error("Failed to delete item during collection: {source}")]
    DeletionError {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Partial failure during batch deletion operations
    #[error(
        "Partial failure during batch deletion: {successful}/{total} items deleted successfully"
    )]
    PartialBatchFailure {
        successful: usize,
        total: usize,
        failures: Vec<String>,
    },

    /// Configuration error
    #[error("Configuration error: {message}")]
    ConfigError { message: String },

    /// Internal error for unexpected conditions
    #[error("Internal garbage collection error: {message}")]
    InternalError { message: String },
}

/// Result type for GC operations
pub type GcResult<T> = Result<T, GcError>;

impl GcError {
    /// Create a new deletion error from any error source
    pub fn deletion_error(source: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::DeletionError {
            source: source.into(),
        }
    }
}

/// Errors specific to blob deletion operations
#[derive(Debug, Error)]
pub enum BlobDeletionError {
    /// File deletion failed
    #[error("Failed to delete blob file {content_hash}: {source}")]
    FileDeletionError {
        content_hash: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Database deletion failed
    #[error("Failed to delete blob database entry {content_hash}: {source}")]
    DatabaseDeletionError {
        content_hash: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Both file and database deletion failed
    #[error("Failed to delete blob {content_hash} from both file system and database")]
    CompleteDeletionFailure { content_hash: String },
}

impl BlobDeletionError {
    pub fn file_deletion_error(
        content_hash: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::FileDeletionError {
            content_hash: content_hash.into(),
            source: source.into(),
        }
    }

    pub fn database_deletion_error(
        content_hash: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::DatabaseDeletionError {
            content_hash: content_hash.into(),
            source: source.into(),
        }
    }

    pub fn complete_deletion_failure(content_hash: impl Into<String>) -> Self {
        Self::CompleteDeletionFailure {
            content_hash: content_hash.into(),
        }
    }
}

/// Errors specific to batch processing operations
#[derive(Debug, Error)]
pub enum BatchProcessingError {
    /// Task panicked during processing
    #[error("Batch processing task panicked: {message}")]
    TaskPanic { message: String },

    /// All items in a batch failed to process
    #[error("All {count} items in batch failed to process")]
    CompleteBatchFailure { count: usize },

    /// Timeout occurred during batch processing
    #[error("Batch processing timed out after {timeout:?}")]
    Timeout { timeout: std::time::Duration },
}

/// Detailed result of a blob deletion attempt
#[derive(Debug, Clone)]
pub struct BlobDeletionAttempt {
    pub content_hash: String,
    pub success: bool,
    pub file_deleted: bool,
    pub db_deleted: bool,
    pub errors: Vec<String>,
}

impl BlobDeletionAttempt {
    pub fn success(content_hash: impl Into<String>) -> Self {
        Self {
            content_hash: content_hash.into(),
            success: true,
            file_deleted: true,
            db_deleted: true,
            errors: Vec::new(),
        }
    }

    pub fn partial_success(
        content_hash: impl Into<String>,
        file_deleted: bool,
        db_deleted: bool,
    ) -> Self {
        Self {
            content_hash: content_hash.into(),
            success: db_deleted, // Consider DB deletion as primary success metric
            file_deleted,
            db_deleted,
            errors: Vec::new(),
        }
    }

    pub fn failure(content_hash: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            content_hash: content_hash.into(),
            success: false,
            file_deleted: false,
            db_deleted: false,
            errors: vec![error.into()],
        }
    }

    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }
}

impl fmt::Display for BlobDeletionAttempt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Blob {}: success={}, file_deleted={}, db_deleted={}",
            self.content_hash, self.success, self.file_deleted, self.db_deleted
        )?;
        if !self.errors.is_empty() {
            write!(f, ", errors: {:?}", self.errors)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gc_error_display() {
        let error = GcError::ConfigError {
            message: "Invalid batch size".to_string(),
        };
        assert_eq!(error.to_string(), "Configuration error: Invalid batch size");
    }

    #[test]
    fn test_blob_deletion_error_display() {
        let error = BlobDeletionError::file_deletion_error("testhash", "File not found");
        assert!(error
            .to_string()
            .contains("Failed to delete blob file testhash"));
    }

    #[test]
    fn test_blob_deletion_attempt_success() {
        let attempt = BlobDeletionAttempt::success("testhash");
        assert!(attempt.success);
        assert!(attempt.file_deleted);
        assert!(attempt.db_deleted);
        assert!(attempt.errors.is_empty());
    }

    #[test]
    fn test_blob_deletion_attempt_partial() {
        let attempt = BlobDeletionAttempt::partial_success("testhash", false, true);
        assert!(attempt.success); // DB deletion is primary success
        assert!(!attempt.file_deleted);
        assert!(attempt.db_deleted);
    }

    #[test]
    fn test_blob_deletion_attempt_failure() {
        let attempt = BlobDeletionAttempt::failure("testhash", "Network error");
        assert!(!attempt.success);
        assert_eq!(attempt.errors.len(), 1);
    }
}
