use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::api::errors::ApiError;
use crate::application::dto::{ListRequest, ListResponse};
use crate::application::use_cases::ListObjectsUseCase;
use crate::domain::authorization::UserContext;

#[derive(Deserialize, ToSchema)]
pub struct ListQuery {
    /// Filter by namespace
    namespace: String,
    /// Filter by tenant
    tenant_id: String,
    /// Results per page (default: 100, max: 1000)
    limit: Option<i64>,
    /// Pagination offset (default: 0)
    offset: Option<i64>,
}

/// GET /v1/objects
/// List objects with pagination
#[utoipa::path(
    get,
    path = "/v1/objects",
    tag = "objects",
    params(
        ("namespace" = String, Query, description = "Filter by namespace"),
        ("tenant_id" = String, Query, description = "Filter by tenant"),
        ("limit" = Option<i64>, Query, description = "Results per page (default: 100, max: 1000)"),
        ("offset" = Option<i64>, Query, description = "Pagination offset (default: 0)")
    ),
    responses(
        (status = 200, description = "Objects retrieved successfully", body = ListResponse),
        (status = 400, description = "Invalid request parameters"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Access forbidden"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_handler(
    State(use_case): State<Arc<ListObjectsUseCase>>,
    axum::extract::Extension(user_context): axum::extract::Extension<UserContext>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ListResponse>, ApiError> {
    // Validate tenant ownership - users can only list objects from their own tenant
    if query.tenant_id != user_context.tenant_id {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "Cannot list objects from other tenants".to_string(),
        ));
    }

    // Validate pagination parameters
    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let offset = query.offset.unwrap_or(0).max(0);

    let request = ListRequest {
        namespace: query.namespace,
        tenant_id: query.tenant_id,
        limit: Some(limit),
        offset: Some(offset),
    };

    let response = use_case.execute(request).await?;

    Ok(Json(response))
}
