use axum::{extract::State, response::Json};
use std::sync::Arc;

use crate::api::errors::ApiError;
use crate::application::dto::{TextSearchRequest, TextSearchResponse};
use crate::application::use_cases::TextSearchObjectsUseCase;

/// POST /v1/objects/search/text
/// Full-text search across metadata and keys
pub async fn text_search_handler(
    State(use_case): State<Arc<TextSearchObjectsUseCase>>,
    Json(request): Json<TextSearchRequest>,
) -> Result<Json<TextSearchResponse>, ApiError> {
    let response = use_case.execute(request).await?;
    Ok(Json(response))
}
