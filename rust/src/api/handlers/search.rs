use axum::{extract::State, response::Json};
use std::sync::Arc;

use crate::api::errors::ApiError;
use crate::api::middleware::validation::validate_and_respond;
use crate::application::dto::{SearchRequest, SearchResponse};
use crate::application::use_cases::SearchObjectsUseCase;
use crate::domain::authorization::UserContext;

/// POST /v1/objects/search
/// Advanced search with filters
#[utoipa::path(
    post,
    path = "/v1/objects/search",
    tag = "search",
    request_body = SearchRequest,
    responses(
        (status = 200, description = "Search completed successfully", body = SearchResponse),
        (status = 400, description = "Invalid search parameters"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Access forbidden"),
        (status = 422, description = "Validation failed"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn search_handler(
    State(use_case): State<Arc<SearchObjectsUseCase>>,
    axum::extract::Extension(user_context): axum::extract::Extension<UserContext>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, ApiError> {
    // Validate the request
    if let Err((status, error_response)) = validate_and_respond(&request) {
        return Err(ApiError::new(
            status,
            serde_json::to_string(&error_response)
                .unwrap_or_else(|_| "Validation error".to_string()),
        ));
    }

    // Validate tenant ownership - users can only search objects from their own tenant
    if request.tenant_id != user_context.tenant_id {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "Cannot search objects from other tenants".to_string(),
        ));
    }

    let response = use_case.execute(request).await?;
    Ok(Json(response))
}
