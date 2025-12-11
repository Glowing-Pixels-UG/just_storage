use axum::{
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use futures_util::StreamExt;
use serde::Deserialize;
use std::sync::Arc;

use crate::api::errors::ApiError;
use crate::application::dto::{ObjectDto, UploadRequest};
use crate::application::use_cases::UploadObjectUseCase;
use crate::domain::value_objects::StorageClass;

#[derive(Deserialize)]
pub struct UploadQuery {
    namespace: String,
    tenant_id: String,
    key: Option<String>,
    storage_class: Option<String>,
}

/// POST /v1/objects
/// Upload object with streaming body
pub async fn upload_handler(
    State(use_case): State<Arc<UploadObjectUseCase>>,
    Query(query): Query<UploadQuery>,
    body: Body,
) -> Result<(StatusCode, Json<ObjectDto>), ApiError> {
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
