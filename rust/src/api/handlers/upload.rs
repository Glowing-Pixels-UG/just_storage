use axum::{
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use futures_util::StreamExt;
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::api::errors::ApiError;
use crate::application::dto::{ObjectDto, UploadRequest};
use crate::application::use_cases::UploadObjectUseCase;
use crate::domain::authorization::UserContext;
use crate::domain::value_objects::StorageClass;

#[derive(Deserialize, ToSchema)]
pub struct UploadQuery {
    /// Object namespace (e.g., 'models', 'kb', 'uploads')
    namespace: String,
    /// Tenant identifier
    tenant_id: String,
    /// Human-readable key for retrieval
    key: Option<String>,
    /// Storage class ('hot' or 'cold', default: 'hot')
    storage_class: Option<String>,
}

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
    Query(query): Query<UploadQuery>,
    body: Body,
) -> Result<(StatusCode, Json<ObjectDto>), ApiError> {
    // Validate tenant ownership - users can only upload to their own tenant
    if query.tenant_id != user_context.tenant_id {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "Cannot upload objects to other tenants".to_string(),
        ));
    }

    // Parse storage class
    let storage_class = match query.storage_class {
        Some(s) => Some(s.parse::<StorageClass>().map_err(ApiError::bad_request)?),
        None => None,
    };

    // Create request DTO
    let request = UploadRequest {
        namespace: query.namespace,
        tenant_id: query.tenant_id,
        key: query.key,
        storage_class,
    };

    // Convert body to async reader
    let stream = body.into_data_stream();
    let reader = Box::pin(tokio_util::io::StreamReader::new(
        stream.map(|result| result.map_err(std::io::Error::other)),
    ));

    // Execute use case
    let object = use_case.execute(request, reader).await?;

    Ok((StatusCode::CREATED, Json(object)))
}
