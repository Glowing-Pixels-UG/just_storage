use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::application::use_cases::{DeleteError, DownloadError, ListError, UploadError};

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

impl From<UploadError> for ApiError {
    fn from(err: UploadError) -> Self {
        match err {
            UploadError::InvalidRequest(msg) => ApiError::bad_request(msg),
            UploadError::Domain(e) => ApiError::bad_request(e.to_string()),
            UploadError::Repository(e) => {
                ApiError::internal_error(format!("Repository error: {}", e))
            }
            UploadError::Storage(e) => ApiError::internal_error(format!("Storage error: {}", e)),
        }
    }
}

impl From<DownloadError> for ApiError {
    fn from(err: DownloadError) -> Self {
        match err {
            DownloadError::NotFound(msg) => ApiError::not_found(msg),
            DownloadError::NotReadable(msg) => {
                ApiError::bad_request(format!("Not readable: {}", msg))
            }
            DownloadError::Repository(e) => {
                ApiError::internal_error(format!("Repository error: {}", e))
            }
            DownloadError::Storage(e) => ApiError::internal_error(format!("Storage error: {}", e)),
        }
    }
}

impl From<DeleteError> for ApiError {
    fn from(err: DeleteError) -> Self {
        match err {
            DeleteError::NotFound(msg) => ApiError::not_found(msg),
            DeleteError::Domain(e) => ApiError::bad_request(e.to_string()),
            DeleteError::Repository(e) => {
                ApiError::internal_error(format!("Repository error: {}", e))
            }
            DeleteError::Storage(e) => ApiError::internal_error(format!("Storage error: {}", e)),
        }
    }
}

impl From<ListError> for ApiError {
    fn from(err: ListError) -> Self {
        match err {
            ListError::InvalidRequest(msg) => ApiError::bad_request(msg),
            ListError::Domain(e) => ApiError::bad_request(e.to_string()),
            ListError::Repository(e) => {
                ApiError::internal_error(format!("Repository error: {}", e))
            }
        }
    }
}
