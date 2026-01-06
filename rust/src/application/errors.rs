//! Common error types for use cases to reduce duplication
//!
//! This module provides standardized error types that can be used across
//! different use cases instead of defining nearly identical error enums.

use thiserror::Error;

use crate::application::ports::{ApiKeyRepositoryError, RepositoryError, StorageError};
use crate::domain::errors::DomainError;

/// Common error type for object-related use cases
/// (upload, download, list, search, delete operations)
#[derive(Debug, Error)]
pub enum ObjectUseCaseError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// Common error type for API key-related use cases
#[derive(Debug, Error)]
pub enum ApiKeyUseCaseError {
    #[error("API key not found: {0}")]
    NotFound(String),

    #[error("Invalid API key ID: {0}")]
    InvalidId(String),

    #[error("Repository error: {0}")]
    Repository(#[from] ApiKeyRepositoryError),
}

/// Common error type for text search use cases
#[derive(Debug, Error)]
pub enum TextSearchUseCaseError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// Common error type for download use cases
#[derive(Debug, Error)]
pub enum DownloadUseCaseError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Object not found: {0}")]
    NotFound(String),

    #[error("Object not readable (status: {0})")]
    NotReadable(String),
}

/// Common error type for delete use cases
#[derive(Debug, Error)]
pub enum DeleteUseCaseError {
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),

    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Not found: {0}")]
    NotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::errors::DomainError;
    use crate::application::ports::{RepositoryError, StorageError, ApiKeyRepositoryError};

    mod object_use_case_error_tests {
        use super::*;

        #[test]
        fn test_object_use_case_error_from_domain_error() {
            let domain_err = DomainError::InvalidNamespace("test error".to_string());
            let obj_err: ObjectUseCaseError = domain_err.clone().into();

            assert!(matches!(obj_err, ObjectUseCaseError::Domain(_)));
            assert!(obj_err.to_string().contains("Domain error"));
        }

        #[test]
        fn test_object_use_case_error_from_repository_error() {
            let repo_err = RepositoryError::NotFound("test".to_string());
            let obj_err: ObjectUseCaseError = repo_err.into();

            assert!(matches!(obj_err, ObjectUseCaseError::Repository(_)));
            assert!(obj_err.to_string().contains("Repository error"));
        }

        #[test]
        fn test_object_use_case_error_from_storage_error() {
            let storage_err = StorageError::NotFound("test".to_string());
            let obj_err: ObjectUseCaseError = storage_err.into();

            assert!(matches!(obj_err, ObjectUseCaseError::Storage(_)));
            assert!(obj_err.to_string().contains("Storage error"));
        }

        #[test]
        fn test_object_use_case_error_invalid_request() {
            let obj_err = ObjectUseCaseError::InvalidRequest("test message".to_string());

            assert!(matches!(obj_err, ObjectUseCaseError::InvalidRequest(_)));
            assert!(obj_err.to_string().contains("Invalid request"));
            assert!(obj_err.to_string().contains("test message"));
        }

        #[test]
        fn test_object_use_case_error_debug() {
            let obj_err = ObjectUseCaseError::InvalidRequest("debug test".to_string());
            let debug_str = format!("{:?}", obj_err);
            assert!(debug_str.contains("InvalidRequest"));
            assert!(debug_str.contains("debug test"));
        }
    }

    mod api_key_use_case_error_tests {
        use super::*;

        #[test]
        fn test_api_key_use_case_error_from_repository_error() {
            let repo_err = ApiKeyRepositoryError::NotFound("test".to_string());
            let api_err: ApiKeyUseCaseError = repo_err.into();

            assert!(matches!(api_err, ApiKeyUseCaseError::Repository(_)));
            assert!(api_err.to_string().contains("Repository error"));
        }

        #[test]
        fn test_api_key_use_case_error_not_found() {
            let api_err = ApiKeyUseCaseError::NotFound("test key".to_string());

            assert!(matches!(api_err, ApiKeyUseCaseError::NotFound(_)));
            assert!(api_err.to_string().contains("API key not found"));
            assert!(api_err.to_string().contains("test key"));
        }

        #[test]
        fn test_api_key_use_case_error_invalid_id() {
            let api_err = ApiKeyUseCaseError::InvalidId("invalid-id".to_string());

            assert!(matches!(api_err, ApiKeyUseCaseError::InvalidId(_)));
            assert!(api_err.to_string().contains("Invalid API key ID"));
            assert!(api_err.to_string().contains("invalid-id"));
        }
    }

    mod text_search_use_case_error_tests {
        use super::*;

        #[test]
        fn test_text_search_use_case_error_from_domain_error() {
            let domain_err = DomainError::InvalidNamespace("test error".to_string());
            let search_err: TextSearchUseCaseError = domain_err.into();

            assert!(matches!(search_err, TextSearchUseCaseError::Domain(_)));
            assert!(search_err.to_string().contains("Domain error"));
        }

        #[test]
        fn test_text_search_use_case_error_from_repository_error() {
            let repo_err = RepositoryError::NotFound("test".to_string());
            let search_err: TextSearchUseCaseError = repo_err.into();

            assert!(matches!(search_err, TextSearchUseCaseError::Repository(_)));
            assert!(search_err.to_string().contains("Repository error"));
        }

        #[test]
        fn test_text_search_use_case_error_invalid_request() {
            let search_err = TextSearchUseCaseError::InvalidRequest("test message".to_string());

            assert!(matches!(search_err, TextSearchUseCaseError::InvalidRequest(_)));
            assert!(search_err.to_string().contains("Invalid request"));
            assert!(search_err.to_string().contains("test message"));
        }
    }

    mod download_use_case_error_tests {
        use super::*;

        #[test]
        fn test_download_use_case_error_from_repository_error() {
            let repo_err = RepositoryError::NotFound("test".to_string());
            let download_err: DownloadUseCaseError = repo_err.into();

            assert!(matches!(download_err, DownloadUseCaseError::Repository(_)));
            assert!(download_err.to_string().contains("Repository error"));
        }

        #[test]
        fn test_download_use_case_error_from_storage_error() {
            let storage_err = StorageError::NotFound("test".to_string());
            let download_err: DownloadUseCaseError = storage_err.into();

            assert!(matches!(download_err, DownloadUseCaseError::Storage(_)));
            assert!(download_err.to_string().contains("Storage error"));
        }

        #[test]
        fn test_download_use_case_error_not_found() {
            let download_err = DownloadUseCaseError::NotFound("test object".to_string());

            assert!(matches!(download_err, DownloadUseCaseError::NotFound(_)));
            assert!(download_err.to_string().contains("Not found"));
            assert!(download_err.to_string().contains("test object"));
        }
    }

    #[test]
    fn test_error_display_formatting() {
        // Test that all error types format correctly
        let obj_err = ObjectUseCaseError::InvalidRequest("test".to_string());
        let api_err = ApiKeyUseCaseError::NotFound("test".to_string());
        let search_err = TextSearchUseCaseError::InvalidRequest("test".to_string());
        let download_err = DownloadUseCaseError::NotFound("test".to_string());

        assert!(obj_err.to_string().contains("Invalid request"));
        assert!(api_err.to_string().contains("API key not found"));
        assert!(search_err.to_string().contains("Invalid request"));
        assert!(download_err.to_string().contains("Not found"));
    }

    #[test]
    fn test_error_debug_formatting() {
        // Test that all error types can be debug formatted
        let obj_err = ObjectUseCaseError::InvalidRequest("test".to_string());
        let api_err = ApiKeyUseCaseError::NotFound("test".to_string());
        let search_err = TextSearchUseCaseError::InvalidRequest("test".to_string());
        let download_err = DownloadUseCaseError::NotFound("test".to_string());

        // These should not panic
        let _ = format!("{:?}", obj_err);
        let _ = format!("{:?}", api_err);
        let _ = format!("{:?}", search_err);
        let _ = format!("{:?}", download_err);
    }
}
