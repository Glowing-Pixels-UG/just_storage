use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::api::errors::ApiError;
use crate::application::dto::{ListRequest, ListResponse};
use crate::application::use_cases::ListObjectsUseCase;

#[derive(Deserialize)]
pub struct ListQuery {
    namespace: String,
    tenant_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// GET /v1/objects
/// List objects with pagination
pub async fn list_handler(
    State(use_case): State<Arc<ListObjectsUseCase>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ListResponse>, ApiError> {
    let request = ListRequest {
        namespace: query.namespace,
        tenant_id: query.tenant_id,
        limit: query.limit,
        offset: query.offset,
    };

    let response = use_case.execute(request).await?;

    Ok(Json(response))
}
