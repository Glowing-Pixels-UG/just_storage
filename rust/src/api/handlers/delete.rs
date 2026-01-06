use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::api::errors::ApiError;
use crate::application::use_cases::DeleteObjectUseCase;
use crate::domain::authorization::UserContext;
use crate::domain::value_objects::ObjectId;

#[derive(Deserialize, ToSchema)]
pub struct DeleteQuery {
    /// Tenant identifier for authorization
    tenant_id: String,
}

/// DELETE /v1/objects/{id}
/// Delete object by ID
#[utoipa::path(
    delete,
    path = "/v1/objects/{id}",
    tag = "objects",
    params(
        ("id" = String, Path, description = "Object UUID"),
        ("tenant_id" = String, Query, description = "Tenant identifier for authorization")
    ),
    responses(
        (status = 204, description = "Object deleted successfully"),
        (status = 400, description = "Invalid object ID"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Access forbidden"),
        (status = 404, description = "Object not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_handler(
    State(use_case): State<Arc<DeleteObjectUseCase>>,
    axum::extract::Extension(user_context): axum::extract::Extension<UserContext>,
    Path(id): Path<String>,
    Query(query): Query<DeleteQuery>,
) -> Result<StatusCode, ApiError> {
    // Validate tenant ownership - users can only delete from their own tenant
    if query.tenant_id != user_context.tenant_id {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "Cannot delete objects from other tenants".to_string(),
        ));
    }

    // Parse object ID
    let object_id = id
        .parse::<ObjectId>()
        .map_err(|e| ApiError::bad_request(format!("Invalid object ID: {}", e)))?;

    // Execute use case
    use_case.execute(&object_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
