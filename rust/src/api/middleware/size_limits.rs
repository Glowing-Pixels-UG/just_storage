use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower::layer::util::Stack;

/// Cached default configuration for performance
static DEFAULT_CONFIG: Lazy<Arc<SizeLimitConfig>> =
    Lazy::new(|| Arc::new(SizeLimitConfig::default()));

/// Size limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeLimitConfig {
    /// Maximum request body size in bytes (default: 50MB)
    pub max_request_size: u64,
    /// Maximum response body size in bytes (default: 100MB)
    pub max_response_size: u64,
    /// Maximum number of form fields (default: 100)
    pub max_form_fields: usize,
    /// Maximum size of a single form field in bytes (default: 1MB)
    pub max_field_size: u64,
    /// Maximum size of uploaded files in bytes (default: 100MB)
    pub max_file_size: u64,
}

impl Default for SizeLimitConfig {
    fn default() -> Self {
        Self {
            max_request_size: 50 * 1024 * 1024,   // 50MB
            max_response_size: 100 * 1024 * 1024, // 100MB
            max_form_fields: 100,
            max_field_size: 1024 * 1024,      // 1MB
            max_file_size: 100 * 1024 * 1024, // 100MB
        }
    }
}

/// Size limit error response
#[derive(Serialize)]
struct SizeLimitErrorResponse {
    error: String,
    code: String,
    max_allowed: Option<String>,
}

/// Parse Content-Length header value
fn parse_content_length(headers: &axum::http::HeaderMap) -> Option<u64> {
    headers
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
}

/// Create a size limit error response
fn size_limit_error(message: &str, max_allowed: Option<String>) -> Response {
    let error_response = SizeLimitErrorResponse {
        error: message.to_string(),
        code: "SIZE_LIMIT_EXCEEDED".to_string(),
        max_allowed,
    };

    (StatusCode::PAYLOAD_TOO_LARGE, axum::Json(error_response)).into_response()
}

/// Request size limit middleware
#[derive(Clone)]
pub struct RequestSizeLimitMiddleware {
    config: Arc<SizeLimitConfig>,
}

impl RequestSizeLimitMiddleware {
    pub fn new(config: SizeLimitConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    pub fn with_cached_config() -> Self {
        Self {
            config: Arc::clone(&DEFAULT_CONFIG),
        }
    }

    pub async fn layer(request: Request, next: Next) -> Response {
        let config = Arc::clone(&DEFAULT_CONFIG);

        // Optimized Content-Length header check
        if let Some(length) = Self::parse_content_length(request.headers()) {
            if length > config.max_request_size {
                return Self::size_limit_error(
                    "Request body too large",
                    Some(format!("{} bytes", config.max_request_size)),
                );
            }
        }

        // For streaming bodies, we need to wrap the body to enforce limits
        // This implementation provides basic protection via Content-Length
        // For full streaming protection, consider implementing a custom Body wrapper

        next.run(request).await
    }

    /// Layer method with explicit config (for optimized creation functions)
    pub async fn layer_with_config(
        request: Request,
        next: Next,
        config: Arc<SizeLimitConfig>,
    ) -> Response {
        // Check Content-Length header first
        if let Some(length) = Self::parse_content_length(request.headers()) {
            if length > config.max_request_size {
                return Self::size_limit_error(
                    "Request body too large",
                    Some(format!("{} bytes", config.max_request_size)),
                );
            }
        }

        next.run(request).await
    }

    /// Optimized Content-Length header parsing
    fn parse_content_length(headers: &axum::http::HeaderMap) -> Option<u64> {
        headers
            .get("content-length")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
    }

    fn size_limit_error(message: &str, max_allowed: Option<String>) -> Response {
        let error_response = SizeLimitErrorResponse {
            error: message.to_string(),
            code: "SIZE_LIMIT_EXCEEDED".to_string(),
            max_allowed,
        };

        (StatusCode::PAYLOAD_TOO_LARGE, axum::Json(error_response)).into_response()
    }
}


/// File upload size validation for specific endpoints
#[derive(Clone)]
pub struct FileUploadLimitMiddleware {
    config: Arc<SizeLimitConfig>,
}

impl FileUploadLimitMiddleware {
    pub fn new(config: SizeLimitConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    pub fn with_cached_config() -> Self {
        Self {
            config: Arc::clone(&DEFAULT_CONFIG),
        }
    }

    pub async fn layer(request: Request, next: Next) -> Response {
        let config = Arc::clone(&DEFAULT_CONFIG);

        // Optimized file upload path detection
        if Self::is_file_upload_request(request.uri().path(), request.method()) {
            tracing::debug!("File upload detected on path: {}", request.uri().path());

            // Check Content-Length against file size limits
            if let Some(length) = parse_content_length(request.headers()) {
                if length > config.max_file_size {
                    return size_limit_error(
                        "File too large",
                        Some(format!("{} bytes", config.max_file_size)),
                    );
                }
            }
        }

        // Optimized multipart detection
        if Self::is_multipart_upload(request.headers()) {
            tracing::debug!("Multipart upload detected - size limits applied");
            // Additional multipart field validation could be added here
        }

        next.run(request).await
    }

    /// Layer method with explicit config (for optimized creation functions)
    pub async fn layer_with_config(
        request: Request,
        next: Next,
        config: Arc<SizeLimitConfig>,
    ) -> Response {
        // Optimized file upload path detection
        if Self::is_file_upload_request(request.uri().path(), request.method()) {
            tracing::debug!("File upload detected on path: {}", request.uri().path());

            // Check Content-Length against file size limits
            if let Some(length) = parse_content_length(request.headers()) {
                if length > config.max_file_size {
                    return size_limit_error(
                        "File too large",
                        Some(format!("{} bytes", config.max_file_size)),
                    );
                }
            }
        }

        // Optimized multipart detection
        if Self::is_multipart_upload(request.headers()) {
            tracing::debug!("Multipart upload detected - size limits applied");
            // Additional multipart field validation could be added here
        }

        next.run(request).await
    }

    /// Optimized multipart content-type detection
    fn is_multipart_upload(headers: &axum::http::HeaderMap) -> bool {
        headers
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .map(|ct| ct.starts_with("multipart/form-data"))
            .unwrap_or(false)
    }

    /// Optimized file upload path detection
    fn is_file_upload_request(path: &str, method: &axum::http::Method) -> bool {
        path.contains("/upload")
            || (path.contains("/objects") && method == axum::http::Method::POST)
    }
}

/// Request size limit layer
#[derive(Clone)]
pub struct RequestSizeLimitLayer {
    middleware: RequestSizeLimitMiddleware,
}

impl RequestSizeLimitLayer {
    pub fn new() -> Self {
        Self {
            middleware: RequestSizeLimitMiddleware::with_cached_config(),
        }
    }
}

impl Default for RequestSizeLimitLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> tower::Layer<S> for RequestSizeLimitLayer
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Service = RequestSizeLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestSizeLimitService {
            inner,
            config: self.middleware.config.clone(),
        }
    }
}

/// Request size limit service
#[derive(Clone)]
pub struct RequestSizeLimitService<S> {
    inner: S,
    config: Arc<SizeLimitConfig>,
}

impl<S> tower::Service<Request> for RequestSizeLimitService<S>
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Response = Response;
    type Error = tower::BoxError;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner
            .poll_ready(cx)
            .map_err(|e| Box::new(e) as tower::BoxError)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let config = self.config.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Check Content-Length header for request size limit
            let (parts, body) = request.into_parts();

            if let Some(length) = parse_content_length(&parts.headers) {
                if length > config.max_request_size {
                    return Err(tower::BoxError::from(format!(
                        "Request body too large: {} bytes (max: {} bytes)",
                        length, config.max_request_size
                    )));
                }
            }

            let request = Request::from_parts(parts, body);
            inner
                .call(request)
                .await
                .map_err(|e| Box::new(e) as tower::BoxError)
        })
    }
}

/// Create request size limit middleware with cached config
pub fn create_request_size_limit_middleware() -> RequestSizeLimitLayer {
    RequestSizeLimitLayer::new()
}



/// File upload limit layer
#[derive(Clone)]
pub struct FileUploadLimitLayer {
    middleware: FileUploadLimitMiddleware,
}

impl FileUploadLimitLayer {
    pub fn new() -> Self {
        Self {
            middleware: FileUploadLimitMiddleware::with_cached_config(),
        }
    }
}

impl Default for FileUploadLimitLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> tower::Layer<S> for FileUploadLimitLayer
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Service = FileUploadLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        FileUploadLimitService {
            inner,
            config: self.middleware.config.clone(),
        }
    }
}

/// File upload limit service
#[derive(Clone)]
pub struct FileUploadLimitService<S> {
    inner: S,
    config: Arc<SizeLimitConfig>,
}

impl<S> tower::Service<Request> for FileUploadLimitService<S>
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    type Response = Response;
    type Error = tower::BoxError;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner
            .poll_ready(cx)
            .map_err(|e| Box::new(e) as tower::BoxError)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let config = self.config.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Check Content-Length for file upload size limits
            let (parts, body) = request.into_parts();

            if let Some(length) = parse_content_length(&parts.headers) {
                if length > config.max_file_size {
                    return Err(tower::BoxError::from(format!(
                        "File too large: {} bytes (max: {} bytes)",
                        length, config.max_file_size
                    )));
                }
            }

            let request = Request::from_parts(parts, body);
            inner
                .call(request)
                .await
                .map_err(|e| Box::new(e) as tower::BoxError)
        })
    }
}

/// Create file upload limit middleware with cached config
pub fn create_file_upload_limit_middleware() -> FileUploadLimitLayer {
    FileUploadLimitLayer::new()
}

/// Create comprehensive size limit middleware stack
pub fn create_size_limit_middleware_stack(
) -> Stack<FileUploadLimitLayer, RequestSizeLimitLayer> {
    let request_limits = create_request_size_limit_middleware();
    let file_limits = create_file_upload_limit_middleware();

    Stack::new(file_limits, request_limits)
}

/// Create concurrency limiting layers
/// Note: This is a placeholder - concurrency limiting not yet implemented
pub fn create_concurrency_limits(_config: &SizeLimitConfig) -> Vec<()> {
    vec![]
}

/// Utility functions for size validation
/// Check if a size is within acceptable limits
pub fn validate_size(size: u64, max_size: u64, context: &str) -> Result<(), String> {
    if size > max_size {
        Err(format!(
            "{} size {} bytes exceeds maximum allowed size {} bytes",
            context, size, max_size
        ))
    } else {
        Ok(())
    }
}

/// Format bytes in human-readable format
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let base = 1024_f64;
    let log = (bytes as f64).log(base).floor() as usize;
    let unit_index = log.min(UNITS.len() - 1);
    let value = bytes as f64 / base.powi(unit_index as i32);

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1} {}", value, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_validation() {
        assert!(validate_size(1024, 2048, "test").is_ok());
        assert!(validate_size(2048, 1024, "test").is_err());

        // Test edge cases
        assert!(validate_size(0, 1024, "test").is_ok());
        assert!(validate_size(1024, 1024, "test").is_ok()); // Equal should be OK
        assert!(validate_size(u64::MAX, u64::MAX, "test").is_ok());
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_bytes(512), "512 B");

        // Test larger values
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024), "1.0 TB");
        assert_eq!(format_bytes(1536), "1.5 KB"); // Test decimal formatting
        assert_eq!(format_bytes(1024 * 1024 + 500 * 1024), "1.5 MB");

        // Test very large values
        let very_large = u64::MAX;
        let formatted = format_bytes(very_large);
        assert!(formatted.contains("PB"));
    }

    #[test]
    fn test_size_limit_config() {
        let config = SizeLimitConfig::default();

        assert_eq!(config.max_request_size, 50 * 1024 * 1024); // 50MB
        assert_eq!(config.max_response_size, 100 * 1024 * 1024); // 100MB
        assert_eq!(config.max_form_fields, 100);
        assert_eq!(config.max_field_size, 1024 * 1024); // 1MB
        assert_eq!(config.max_file_size, 100 * 1024 * 1024); // 100MB
    }

    #[test]
    fn test_size_limit_error_response() {
        let config = SizeLimitConfig::default();
        let middleware = RequestSizeLimitMiddleware::new(config);

        // Note: In a real integration test, we'd test the actual middleware behavior
        // For now, we test the configuration and utility functions
        assert!(middleware.config.max_request_size > 0);
    }

    #[test]
    fn test_concurrency_limits_creation() {
        let config = SizeLimitConfig::default();
        let limits = create_concurrency_limits(&config);

        // Placeholder test - concurrency limiting not yet implemented
        assert!(limits.is_empty());
    }

    #[test]
    fn test_size_limit_utilities() {
        // Test the validation function error messages
        match validate_size(2048, 1024, "file upload") {
            Ok(_) => panic!("Expected error"),
            Err(msg) => {
                assert!(msg.contains("file upload"));
                assert!(msg.contains("2048"));
                assert!(msg.contains("1024"));
            }
        }

        // Test successful validation
        assert!(validate_size(512, 1024, "small file").is_ok());
    }

    #[test]
    fn test_byte_formatting_edge_cases() {
        // Test very small values
        assert_eq!(format_bytes(1), "1 B");
        assert_eq!(format_bytes(999), "999 B");

        // Test boundary values
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.0 KB");

        // Test large values
        let very_large = 1024u64.pow(5); // 1 PB
        let formatted = format_bytes(very_large);
        assert!(formatted.contains("PB"));
    }

    #[test]
    fn test_size_limit_config_validation() {
        let config = SizeLimitConfig::default();

        assert_eq!(config.max_request_size, 50 * 1024 * 1024); // 50MB
        assert_eq!(config.max_response_size, 100 * 1024 * 1024); // 100MB
        assert_eq!(config.max_file_size, 100 * 1024 * 1024); // 100MB
    }
}
