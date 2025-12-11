use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
};
use std::sync::Arc;
use tokio_util::io::ReaderStream;

use crate::api::errors::ApiError;
use crate::application::use_cases::DownloadObjectUseCase;
use crate::domain::value_objects::ObjectId;

/// GET /v1/objects/{id}
/// Download object by ID with streaming response
pub async fn download_handler(
    State(use_case): State<Arc<DownloadObjectUseCase>>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    // Parse object ID
    let object_id = id
        .parse::<ObjectId>()
        .map_err(|e| ApiError::bad_request(format!("Invalid object ID: {}", e)))?;

    // Execute use case
    let (metadata, reader) = use_case.execute_by_id(&object_id).await?;

    // Convert reader to stream
    let stream = ReaderStream::new(reader);
    let body = Body::from_stream(stream);

    // Build response with headers
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, metadata.size_bytes.to_string())
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header("X-Content-Hash", metadata.content_hash)
        .body(body)
        .map_err(|e| ApiError::internal_error(format!("Failed to build response: {}", e)))?;

    Ok(response)
}
