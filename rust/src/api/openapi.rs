use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::application::dto::{
    DateRange, DownloadMetadata, ListRequest, ListResponse, ObjectDto, SearchRequest,
    SearchResponse, SizeRange, SortDirection, SortField, TextSearchRequest, TextSearchResponse,
    UploadRequest,
};

/// OpenAPI specification for JustStorage API
#[derive(OpenApi)]
#[openapi(
    info(
        title = "JustStorage API",
        version = "1.0.0",
        description = "Object storage service with content deduplication and advanced search capabilities"
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server"),
        (url = "https://api.juststorage.com", description = "Production server")
    ),
    paths(
        crate::api::handlers::health::health_handler,
        crate::api::handlers::health::readiness_handler,
        crate::api::handlers::upload::upload_handler,
        crate::api::handlers::list::list_handler,
        crate::api::handlers::download::download_handler,
        crate::api::handlers::download::download_by_key_handler,
        crate::api::handlers::delete::delete_handler,
        crate::api::handlers::search::search_handler,
        crate::api::handlers::text_search::text_search_handler,
    ),
    components(
        schemas(
            ObjectDto,
            UploadRequest,
            ListRequest,
            ListResponse,
            SearchRequest,
            SearchResponse,
            TextSearchRequest,
            TextSearchResponse,
            DownloadMetadata,
            SortField,
            SortDirection,
            DateRange,
            SizeRange,
        )
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "objects", description = "Object storage operations"),
        (name = "search", description = "Search and filtering operations")
    )
)]
pub struct ApiDoc;

/// Create the Swagger UI route
pub fn swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui/*tail").url("/api-docs/openapi.json", ApiDoc::openapi())
}
