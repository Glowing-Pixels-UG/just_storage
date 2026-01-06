use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::api::errors::ApiError;
use crate::api::middleware::validation::validate_and_respond;
use crate::application::{
    dto::{ApiKeyDto, ApiKeyListResponse, CreateApiKeyRequest, UpdateApiKeyRequest},
    use_cases::{
        CreateApiKeyUseCase, DeleteApiKeyUseCase, GetApiKeyUseCase, ListApiKeysUseCase,
        UpdateApiKeyUseCase,
    },
};
use crate::domain::authorization::UserContext;

/// Query parameters for listing API keys
#[derive(Deserialize, utoipa::ToSchema)]
pub struct ListApiKeysQuery {
    /// Number of results per page (default: 50, max: 100)
    limit: Option<i64>,
    /// Pagination offset (default: 0)
    offset: Option<i64>,
}

/// POST /v1/api-keys
/// Create a new API key
#[utoipa::path(
    post,
    path = "/v1/api-keys",
    tag = "api-keys",
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API key created successfully", body = ApiKeyDto),
        (status = 400, description = "Invalid request parameters"),
        (status = 401, description = "Authentication required"),
        (status = 422, description = "Validation failed"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn create_api_key_handler(
    State(use_case): State<Arc<CreateApiKeyUseCase>>,
    axum::extract::Extension(user_context): axum::extract::Extension<UserContext>,
    Json(request): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiKeyDto>), ApiError> {
    // Validate the request
    if let Err((status, error_response)) = validate_and_respond(&request) {
        return Err(ApiError::new(
            status,
            serde_json::to_string(&error_response)
                .unwrap_or_else(|_| "Validation error".to_string()),
        ));
    }

    // Get tenant_id from authentication context
    let tenant_id = user_context.tenant_id.clone();

    let api_key = use_case.execute(tenant_id, request).await?;
    Ok((StatusCode::CREATED, Json(api_key)))
}

/// GET /v1/api-keys
/// List API keys for the tenant
#[utoipa::path(
    get,
    path = "/v1/api-keys",
    tag = "api-keys",
    params(
        ("limit" = Option<i64>, Query, description = "Results per page (default: 50, max: 100)"),
        ("offset" = Option<i64>, Query, description = "Pagination offset (default: 0)")
    ),
    responses(
        (status = 200, description = "API keys retrieved successfully", body = ApiKeyListResponse),
        (status = 401, description = "Authentication required"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_api_keys_handler(
    State(use_case): State<Arc<ListApiKeysUseCase>>,
    axum::extract::Extension(user_context): axum::extract::Extension<UserContext>,
    Query(query): Query<ListApiKeysQuery>,
) -> Result<Json<ApiKeyListResponse>, ApiError> {
    // Get tenant_id from authentication context
    let tenant_id = user_context.tenant_id.clone();

    let response = use_case
        .execute(tenant_id, query.limit, query.offset)
        .await?;
    Ok(Json(response))
}

/// GET /v1/api-keys/{id}
/// Get a specific API key
#[utoipa::path(
    get,
    path = "/v1/api-keys/{id}",
    tag = "api-keys",
    params(
        ("id" = String, Path, description = "API key ID")
    ),
    responses(
        (status = 200, description = "API key retrieved successfully", body = ApiKeyDto),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "API key not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_api_key_handler(
    State(use_case): State<Arc<GetApiKeyUseCase>>,
    axum::extract::Extension(user_context): axum::extract::Extension<UserContext>,
    Path(api_key_id): Path<String>,
) -> Result<Json<ApiKeyDto>, ApiError> {
    // Get tenant_id from authentication context
    let tenant_id = &user_context.tenant_id;

    let api_key = use_case.execute(tenant_id, &api_key_id).await?;
    Ok(Json(api_key))
}

/// PUT /v1/api-keys/{id}
/// Update an API key
#[utoipa::path(
    put,
    path = "/v1/api-keys/{id}",
    tag = "api-keys",
    params(
        ("id" = String, Path, description = "API key ID")
    ),
    request_body = UpdateApiKeyRequest,
    responses(
        (status = 200, description = "API key updated successfully", body = ApiKeyDto),
        (status = 400, description = "Invalid request parameters"),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "API key not found"),
        (status = 422, description = "Validation failed"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn update_api_key_handler(
    State(use_case): State<Arc<UpdateApiKeyUseCase>>,
    axum::extract::Extension(user_context): axum::extract::Extension<UserContext>,
    Path(api_key_id): Path<String>,
    Json(request): Json<UpdateApiKeyRequest>,
) -> Result<Json<ApiKeyDto>, ApiError> {
    // Validate the request
    if let Err((status, error_response)) = validate_and_respond(&request) {
        return Err(ApiError::new(
            status,
            serde_json::to_string(&error_response)
                .unwrap_or_else(|_| "Validation error".to_string()),
        ));
    }

    // Get tenant_id from authentication context
    let tenant_id = &user_context.tenant_id;

    let api_key = use_case.execute(tenant_id, &api_key_id, request).await?;
    Ok(Json(api_key))
}

/// DELETE /v1/api-keys/{id}
/// Delete an API key
#[utoipa::path(
    delete,
    path = "/v1/api-keys/{id}",
    tag = "api-keys",
    params(
        ("id" = String, Path, description = "API key ID")
    ),
    responses(
        (status = 204, description = "API key deleted successfully"),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "API key not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn delete_api_key_handler(
    State(use_case): State<Arc<DeleteApiKeyUseCase>>,
    axum::extract::Extension(user_context): axum::extract::Extension<UserContext>,
    Path(api_key_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // Get tenant_id from authentication context
    let tenant_id = &user_context.tenant_id;

    use_case.execute(tenant_id, &api_key_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
