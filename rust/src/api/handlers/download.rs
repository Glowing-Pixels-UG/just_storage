use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio_util::io::ReaderStream;
use utoipa::ToSchema;

use crate::api::errors::ApiError;
use crate::application::use_cases::DownloadObjectUseCase;
use crate::domain::authorization::UserContext;
use crate::domain::value_objects::ObjectId;

#[derive(Deserialize, ToSchema)]
pub struct DownloadQuery {
    /// Tenant identifier for authorization
    tenant_id: String,
}

/// GET /v1/objects/{id}
/// Download object by ID with streaming response
#[utoipa::path(
    get,
    path = "/v1/objects/{id}",
    tag = "objects",
    params(
        ("id" = String, Path, description = "Object UUID"),
        ("tenant_id" = String, Query, description = "Tenant identifier for authorization")
    ),
    responses(
        (status = 200, description = "Object downloaded successfully", content_type = "application/octet-stream"),
        (status = 400, description = "Invalid object ID"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Access forbidden"),
        (status = 404, description = "Object not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn download_handler(
    State(use_case): State<Arc<DownloadObjectUseCase>>,
    axum::extract::Extension(user_context): axum::extract::Extension<UserContext>,
    Path(id): Path<String>,
    Query(query): Query<DownloadQuery>,
) -> Result<Response, ApiError> {
    // Validate tenant ownership - users can only download from their own tenant
    if query.tenant_id != user_context.tenant_id {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "Cannot download objects from other tenants".to_string(),
        ));
    }

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

/// GET /v1/objects/by-key/{namespace}/{tenant_id}/{key}
/// Download object by key with streaming response
#[utoipa::path(
    get,
    path = "/v1/objects/by-key/{namespace}/{tenant_id}/{key}",
    tag = "objects",
    params(
        ("namespace" = String, Path, description = "Object namespace"),
        ("tenant_id" = String, Path, description = "Tenant identifier"),
        ("key" = String, Path, description = "Object key")
    ),
    responses(
        (status = 200, description = "Object downloaded successfully", content_type = "application/octet-stream"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Access forbidden"),
        (status = 404, description = "Object not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn download_by_key_handler(
    State(use_case): State<Arc<DownloadObjectUseCase>>,
    axum::extract::Extension(user_context): axum::extract::Extension<UserContext>,
    Path((namespace, tenant_id, key)): Path<(String, String, String)>,
) -> Result<Response, ApiError> {
    // Validate tenant ownership - users can only download from their own tenant
    if tenant_id != user_context.tenant_id {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "Cannot download objects from other tenants".to_string(),
        ));
    }
    // Execute use case
    let (metadata, reader) = use_case
        .execute_by_key(&namespace, &tenant_id, &key)
        .await?;

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
