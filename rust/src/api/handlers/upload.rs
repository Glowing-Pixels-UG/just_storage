use axum::body::Body;
use axum::http::StatusCode;
use futures_util::TryStreamExt;
use std::io;
use std::sync::Arc;
use tokio_util::io::StreamReader;

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
    let namespace = query_params.get("namespace").cloned().unwrap_or_default();
    let tenant_id = query_params.get("tenant_id").cloned().unwrap_or_default();
    let key = query_params.get("key").cloned();
    let storage_class = query_params
        .get("storage_class")
        .map(|sc| sc.parse::<StorageClass>())
        .transpose()
        .map_err(ApiError::bad_request)?;

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

    // Convert the Axum body into a data stream, map its errors to standard io errors
    let stream = body
        .into_data_stream()
        .map_err(|e| io::Error::other(e.to_string()));

    // Create an AsyncRead from the stream
    let reader = Box::pin(StreamReader::new(stream));

    // Create request DTO
    let request = UploadRequest {
        namespace,
        tenant_id,
        key,
        storage_class,
    };

    // Execute use case, passing the async reader directly
    let object = use_case.execute(request, reader).await?;

    Ok((StatusCode::CREATED, Json(object)))
}
