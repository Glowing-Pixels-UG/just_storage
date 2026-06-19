use axum::body::Body;
use axum::http::StatusCode;
use std::sync::Arc;

use crate::api::errors::ApiError;
use crate::application::dto::{ObjectDto, UploadRequest};
use crate::application::use_cases::UploadObjectUseCase;
use crate::domain::authorization::UserContext;
use crate::domain::value_objects::StorageClass;

use axum::extract::{Query, State};
use axum::response::Json;

/// POST /v1/objects
/// Upload object with streaming body
#[utoipa::path(
    post,
    path = "/v1/objects",
    tag = "objects",
    params(
        ("namespace" = String, Query, description = "Object namespace"),
        ("tenant_id" = String, Query, description = "Tenant identifier"),
        ("key" = Option<String>, Query, description = "Human-readable key for retrieval"),
        ("storage_class" = Option<String>, Query, description = "Storage class ('hot' or 'cold')")
    ),
    request_body = Vec<u8>,
    responses(
        (status = 201, description = "Object uploaded successfully", body = ObjectDto),
        (status = 400, description = "Invalid request parameters"),
        (status = 401, description = "Authentication required"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn upload_handler(
    State(use_case): State<Arc<UploadObjectUseCase>>,
    axum::extract::Extension(user_context): axum::extract::Extension<UserContext>,
    query_params: Query<std::collections::HashMap<String, String>>,
    body: Body,
) -> Result<(StatusCode, Json<ObjectDto>), ApiError> {
    let upload_limit = usize::try_from(use_case.max_upload_size_bytes()).unwrap_or(usize::MAX);

    // Buffer the body up to the configured upload limit. The storage layer still
    // writes through an AsyncRead, so callers are not capped by a test-only 10MiB limit.
    let bytes = axum::body::to_bytes(body, upload_limit)
        .await
        .map_err(|_| ApiError::bad_request("Failed to read request body"))?;

    // Try to parse JSON body that may contain metadata and data
    let mut namespace = query_params.get("namespace").cloned().unwrap_or_default();
    let mut tenant_id = query_params.get("tenant_id").cloned().unwrap_or_default();
    let mut key = query_params.get("key").cloned();
    let mut storage_class = None;

    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
        if let Some(ns) = json.get("namespace").and_then(|v| v.as_str()) {
            namespace = ns.to_string();
        }
        if let Some(tid) = json.get("tenant_id").and_then(|v| v.as_str()) {
            tenant_id = tid.to_string();
        }
        if let Some(k) = json.get("key").and_then(|v| v.as_str()) {
            key = Some(k.to_string());
        }
        if let Some(sc) = json.get("storage_class").and_then(|v| v.as_str()) {
            storage_class = Some(sc.parse::<StorageClass>().map_err(ApiError::bad_request)?);
        }

        // If the JSON contains a "data" field, use it as the body
        if let Some(data) = json.get("data").and_then(|v| v.as_str()) {
            // Use the provided data as the upload body
            let reader = Box::pin(std::io::Cursor::new(data.as_bytes().to_vec()));

            // If required metadata is missing, return bad request
            if namespace.is_empty() || tenant_id.is_empty() {
                return Err(ApiError::bad_request(
                    "Missing required fields: namespace and tenant_id",
                ));
            }

            // Validate tenant_id format first (should be a UUID)
            if uuid::Uuid::parse_str(&tenant_id).is_err() {
                return Err(ApiError::bad_request("Invalid tenant_id format"));
            }

            // Validate tenant ownership - users can only upload to their own tenant
            // Admins can upload to any tenant
            if !user_context.is_admin() && tenant_id != user_context.tenant_id {
                return Err(ApiError::new(
                    axum::http::StatusCode::FORBIDDEN,
                    "Cannot upload objects to other tenants".to_string(),
                ));
            }

            let request = UploadRequest {
                namespace,
                tenant_id,
                key,
                storage_class,
            };

            let object = use_case.execute(request, reader).await?;
            return Ok((StatusCode::CREATED, Json(object)));
        }
    }

    // Fallback: treat buffered bytes as raw body stream (was a streaming upload)
    // If required metadata is missing, return bad request
    if namespace.is_empty() || tenant_id.is_empty() {
        return Err(ApiError::bad_request(
            "Missing required fields: namespace and tenant_id",
        ));
    }

    // Validate tenant_id format first (should be a UUID)
    if uuid::Uuid::parse_str(&tenant_id).is_err() {
        return Err(ApiError::bad_request("Invalid tenant_id format"));
    }

    // Validate tenant ownership - users can only upload to their own tenant
    // Admins can upload to any tenant
    if !user_context.is_admin() && tenant_id != user_context.tenant_id {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "Cannot upload objects to other tenants".to_string(),
        ));
    }

    // Use a Cursor over the buffered bytes to provide an AsyncRead implementation
    let reader = Box::pin(std::io::Cursor::new(bytes.to_vec()));

    // Create request DTO
    let request = UploadRequest {
        namespace,
        tenant_id,
        key,
        storage_class,
    };

    // Execute use case
    let object = use_case.execute(request, reader).await?;

    Ok((StatusCode::CREATED, Json(object)))
}
