use axum::{
    http::StatusCode,
    middleware as axum_middleware,
    routing::{delete, get, post},
    Router,
};
use sqlx::PgPool;
use std::sync::Arc;

use crate::api::handlers::{
    api_keys::{
        create_api_key_handler, delete_api_key_handler, get_api_key_handler, list_api_keys_handler,
        update_api_key_handler,
    },
    delete_handler, download_by_key_handler, download_handler, health_handler, list_handler,
    readiness_handler, search, text_search, upload_handler,
};
use crate::api::internal::create_internal_router;
use crate::api::middleware::{
    authorization, config::MiddlewareConfig, content_type, factory::MiddlewareFactory, size_limits,
};
use crate::api::openapi::ApiDoc;
use crate::application::gc::GarbageCollector;
use crate::application::ports::{ApiKeyRepository, AuditRepository, BlobStore};
use crate::application::use_cases::{
    CreateApiKeyUseCase, DeleteApiKeyUseCase, DeleteObjectUseCase, DownloadObjectUseCase,
    GetApiKeyUseCase, ListApiKeysUseCase, ListObjectsUseCase, SearchObjectsUseCase,
    TextSearchObjectsUseCase, UpdateApiKeyUseCase, UploadObjectUseCase,
};
use axum::routing::put;
use utoipa::OpenApi;

use crate::config::Config;

use std::time::Instant;

/// Application state container
#[derive(Clone)]
pub struct AppState {
    pub pool: Arc<PgPool>,
    pub upload_use_case: Arc<UploadObjectUseCase>,
    pub download_use_case: Arc<DownloadObjectUseCase>,
    pub delete_use_case: Arc<DeleteObjectUseCase>,
    pub list_use_case: Arc<ListObjectsUseCase>,
    pub search_use_case: Arc<SearchObjectsUseCase>,
    pub text_search_use_case: Arc<TextSearchObjectsUseCase>,
    pub create_api_key_use_case: Arc<CreateApiKeyUseCase>,
    pub list_api_keys_use_case: Arc<ListApiKeysUseCase>,
    pub get_api_key_use_case: Arc<GetApiKeyUseCase>,
    pub update_api_key_use_case: Arc<UpdateApiKeyUseCase>,
    pub delete_api_key_use_case: Arc<DeleteApiKeyUseCase>,
    pub audit_repo: Arc<dyn AuditRepository>,
    pub blob_store: Arc<dyn BlobStore>,
    pub gc: Option<Arc<GarbageCollector>>,
    pub config: Config,
    pub expected_migration_count: usize,
    pub start_time: Instant,
}

impl AppState {
    pub fn config(&self) -> &Config {
        &self.config
    }
}

/// Create router with all routes and middleware
pub fn create_router(
    state: AppState,
    api_key_repo: Arc<dyn ApiKeyRepository + Send + Sync>,
    audit_repo: Arc<dyn AuditRepository + Send + Sync>,
) -> Router {
    create_router_with_middleware(state, api_key_repo, audit_repo, MiddlewareConfig::default())
}

/// Create router with custom middleware configuration
pub fn create_router_with_middleware(
    state: AppState,
    api_key_repo: Arc<dyn ApiKeyRepository + Send + Sync>,
    audit_repo: Arc<dyn AuditRepository + Send + Sync>,
    middleware_config: MiddlewareConfig,
) -> Router {
    let middleware_factory = MiddlewareFactory::new(middleware_config);

    // 3. Internal routes (have their own middleware and auth)
    let internal_router = create_internal_router(state.clone());

    // Initialize the main router
    let mut router = Router::new();
    
    // 1. Internal routes (nest under /dashboard if admin_port is not set)
    if state.config.admin_port.is_none() {
        println!("DEBUG: Registering dashboard routes under /dashboard");
        router = router.nest("/dashboard", internal_router);
    }

    // 2. Public routes (no auth, no main middleware)
    let mut public_router = Router::new();
    public_router = add_health_routes(public_router, &state);
    public_router = add_openapi_routes(public_router);
    
    // Merge public routes into main router
    router = router.merge(public_router);

    // 3. API routes (require main middleware stack including auth)
    let mut api_router = Router::new();
    api_router = add_object_routes(api_router, &state);
    api_router = add_api_key_routes(api_router, &state);

    // Apply middleware stack only to API routes
    api_router = apply_middleware_stack(
        api_router,
        &middleware_factory,
        Arc::clone(&api_key_repo),
        audit_repo,
    );

    // Merge API router into main router
    router = router.merge(api_router);

    // Apply global middleware (security headers, etc.) to the entire application
    router = router
        .layer(axum_middleware::from_fn(|req, next| async move {
            crate::api::middleware::security_headers::SecurityHeadersMiddleware::default()
                .layer(req, next)
                .await
        }))
        .layer(axum_middleware::from_fn(
            crate::api::middleware::security_headers::RequestSanitizationMiddleware::layer,
        ));

    router
}

/// Add health check routes
fn add_health_routes(router: Router, state: &AppState) -> Router {
    router
        .route("/health", get(health_handler))
        .route(
            "/health/ready",
            get(readiness_handler).with_state(Arc::clone(&state.pool)),
        )
        .route("/favicon.ico", get(|| async { StatusCode::NO_CONTENT }))
}

/// Add OpenAPI documentation routes
fn add_openapi_routes(router: Router) -> Router {
    router.route(
        "/api-docs/openapi.json",
        get(|| async { axum::Json(ApiDoc::openapi()) }),
    )
}

/// Add object management routes
fn add_object_routes(router: Router, state: &AppState) -> Router {
    let upload_state = Arc::clone(&state.upload_use_case);
    let download_state = Arc::clone(&state.download_use_case);
    let delete_state = Arc::clone(&state.delete_use_case);
    let list_state = Arc::clone(&state.list_use_case);
    let search_state = Arc::clone(&state.search_use_case);
    let text_search_state = Arc::clone(&state.text_search_use_case);

    router
        // Object CRUD operations
        .route(
            "/v1/objects",
            post(upload_handler)
                .layer(axum_middleware::from_fn(
                    authorization::require_object_write,
                )) // Validate content-type and JSON body before handlers (so malformed or wrong types return 4xx without requiring auth)
                .layer(axum_middleware::from_fn(
                    crate::api::middleware::content_type::validate_json_for_objects,
                ))
                .with_state(upload_state),
        )
        .route(
            "/v1/objects",
            get(list_handler)
                .layer(axum_middleware::from_fn(authorization::require_object_read))
                .with_state(list_state),
        )
        .route(
            "/v1/objects/{id}",
            get(download_handler)
                .layer(axum_middleware::from_fn(authorization::require_object_read))
                .with_state(Arc::clone(&download_state)),
        )
        .route(
            "/v1/objects/{id}",
            delete(delete_handler)
                .layer(axum_middleware::from_fn(
                    authorization::require_object_delete,
                ))
                .with_state(delete_state),
        )
        // Object search operations
        .route(
            "/v1/objects/search",
            post(search::search_handler)
                .layer(axum_middleware::from_fn(authorization::require_object_read))
                .with_state(search_state),
        )
        .route(
            "/v1/objects/search/text",
            post(text_search::text_search_handler)
                .layer(axum_middleware::from_fn(authorization::require_object_read))
                .with_state(text_search_state),
        )
        // Key-based object access
        .route(
            "/v1/objects/by-key/{namespace}/{tenant_id}/{key}",
            get(download_by_key_handler)
                .layer(axum_middleware::from_fn(authorization::require_object_read))
                .with_state(download_state),
        )
}

/// Add API key management routes
fn add_api_key_routes(router: Router, state: &AppState) -> Router {
    let create_api_key_state = Arc::clone(&state.create_api_key_use_case);
    let list_api_keys_state = Arc::clone(&state.list_api_keys_use_case);
    let get_api_key_state = Arc::clone(&state.get_api_key_use_case);
    let update_api_key_state = Arc::clone(&state.update_api_key_use_case);
    let delete_api_key_state = Arc::clone(&state.delete_api_key_use_case);

    router
        .route(
            "/v1/api-keys",
            post(create_api_key_handler)
                .layer(axum_middleware::from_fn(
                    authorization::require_api_key_management,
                ))
                .with_state(create_api_key_state),
        )
        .route(
            "/v1/api-keys",
            get(list_api_keys_handler)
                .layer(axum_middleware::from_fn(
                    authorization::require_api_key_management,
                ))
                .with_state(list_api_keys_state),
        )
        .route(
            "/v1/api-keys/{id}",
            get(get_api_key_handler)
                .layer(axum_middleware::from_fn(
                    authorization::require_api_key_management,
                ))
                .with_state(get_api_key_state),
        )
        .route(
            "/v1/api-keys/{id}",
            put(update_api_key_handler)
                .layer(axum_middleware::from_fn(
                    authorization::require_api_key_management,
                ))
                .with_state(update_api_key_state),
        )
        .route(
            "/v1/api-keys/{id}",
            delete(delete_api_key_handler)
                .layer(axum_middleware::from_fn(
                    authorization::require_api_key_management,
                ))
                .with_state(delete_api_key_state),
        )
}

/// Apply the complete middleware stack to the router
fn apply_middleware_stack(
    router: Router,
    middleware_factory: &MiddlewareFactory,
    api_key_repo: Arc<dyn crate::application::ports::ApiKeyRepository + Send + Sync>,
    audit_repo: Arc<dyn crate::application::ports::AuditRepository + Send + Sync>,
) -> Router {
    // Apply middleware in order (innermost/last = runs first):
    // 1. Security headers (outermost - adds headers to response)
    // 2. Metrics
    // 3. Audit (runs after auth to have user context)
    // 4. Auth (runs after validation to allow proper error codes)
    // 5. Content-type validation (runs before auth)
    // 6. Size limits (runs before auth)
    // 7. CORS (innermost - runs first)
    let audit_layer = middleware_factory.create_audit_layer(audit_repo);

    router
        .layer(middleware_factory.create_metrics_layer())
        .layer(axum::middleware::from_fn(move |req, next| {
            let audit_layer = audit_layer.clone();
            async move { audit_layer.layer(req, next).await }
        }))
        .layer(middleware_factory.create_auth_layer(api_key_repo))
        .layer(axum::middleware::from_fn(
            content_type::validate_json_for_objects,
        ))
        .layer(axum::middleware::from_fn(
            size_limits::RequestSizeLimitMiddleware::layer,
        ))
        .layer(middleware_factory.create_cors_layer())
}
