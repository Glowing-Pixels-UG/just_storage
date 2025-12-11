use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use std::sync::Arc;

use crate::api::errors::ApiError;
use crate::application::use_cases::DeleteObjectUseCase;
use crate::domain::value_objects::ObjectId;

/// DELETE /v1/objects/{id}
/// Delete object by ID
pub async fn delete_handler(
    State(use_case): State<Arc<DeleteObjectUseCase>>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // Parse object ID
    let object_id = id
        .parse::<ObjectId>()
        .map_err(|e| ApiError::bad_request(format!("Invalid object ID: {}", e)))?;

    // Execute use case
    use_case.execute(&object_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
