use axum::{extract::State, response::Json};
use std::sync::Arc;

use crate::api::errors::ApiError;
use crate::application::dto::{SearchRequest, SearchResponse};
use crate::application::use_cases::SearchObjectsUseCase;

/// POST /v1/objects/search
/// Advanced search with filters
pub async fn search_handler(
    State(use_case): State<Arc<SearchObjectsUseCase>>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, ApiError> {
    let response = use_case.execute(request).await?;
    Ok(Json(response))
}
