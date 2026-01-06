use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::application::{
    errors::{
        DeleteUseCaseError, DownloadUseCaseError, ObjectUseCaseError, TextSearchUseCaseError,
    },
    use_cases::ApiKeyUseCaseError,
};

/// API error response
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, message)
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({
            "error": self.message,
        }));

        (self.status, body).into_response()
    }
}

// Convert use case errors to API errors

impl From<ObjectUseCaseError> for ApiError {
    fn from(err: ObjectUseCaseError) -> Self {
        match err {
            ObjectUseCaseError::InvalidRequest(msg) => ApiError::bad_request(msg),
            ObjectUseCaseError::Domain(e) => ApiError::bad_request(e.to_string()),
            ObjectUseCaseError::Repository(e) => {
                ApiError::internal_error(format!("Repository error: {}", e))
            }
            ObjectUseCaseError::Storage(e) => {
                ApiError::internal_error(format!("Storage error: {}", e))
            }
        }
    }
}

impl From<DownloadUseCaseError> for ApiError {
    fn from(err: DownloadUseCaseError) -> Self {
        match err {
            DownloadUseCaseError::NotFound(msg) => ApiError::not_found(msg),
            DownloadUseCaseError::NotReadable(msg) => {
                ApiError::bad_request(format!("Not readable: {}", msg))
            }
            DownloadUseCaseError::Repository(e) => {
                ApiError::internal_error(format!("Repository error: {}", e))
            }
            DownloadUseCaseError::Storage(e) => {
                ApiError::internal_error(format!("Storage error: {}", e))
            }
        }
    }
}

impl From<DeleteUseCaseError> for ApiError {
    fn from(err: DeleteUseCaseError) -> Self {
        match err {
            DeleteUseCaseError::Domain(e) => {
                ApiError::internal_error(format!("Domain error: {}", e))
            }
            DeleteUseCaseError::NotFound(msg) => ApiError::not_found(msg),
            DeleteUseCaseError::Repository(e) => {
                ApiError::internal_error(format!("Repository error: {}", e))
            }
            DeleteUseCaseError::Storage(e) => {
                ApiError::internal_error(format!("Storage error: {}", e))
            }
        }
    }
}

// ListError and SearchError now use ObjectUseCaseError, so no separate impl needed

impl From<TextSearchUseCaseError> for ApiError {
    fn from(err: TextSearchUseCaseError) -> Self {
        match err {
            TextSearchUseCaseError::InvalidRequest(msg) => ApiError::bad_request(msg),
            TextSearchUseCaseError::Domain(e) => ApiError::bad_request(e.to_string()),
            TextSearchUseCaseError::Repository(e) => {
                ApiError::internal_error(format!("Repository error: {}", e))
            }
        }
    }
}

impl From<ApiKeyUseCaseError> for ApiError {
    fn from(err: ApiKeyUseCaseError) -> Self {
        match err {
            ApiKeyUseCaseError::NotFound(id) => {
                ApiError::not_found(format!("API key not found: {}", id))
            }
            ApiKeyUseCaseError::InvalidId(id) => {
                ApiError::bad_request(format!("Invalid API key ID: {}", id))
            }
            ApiKeyUseCaseError::Repository(e) => {
                ApiError::internal_error(format!("Repository error: {}", e))
            }
        }
    }
}
