//! Security headers middleware implementation
//!
//! This module contains the actual middleware logic for adding
//! security headers to HTTP responses.

use axum::{
    extract::Request,
    http::{header, HeaderMap, StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use super::security_config::SecurityHeadersConfig;

/// Security headers middleware
#[derive(Clone)]
pub struct SecurityHeadersMiddleware {
    config: Arc<SecurityHeadersConfig>,
}

impl SecurityHeadersMiddleware {
    pub fn new(config: Arc<SecurityHeadersConfig>) -> Self {
        Self { config }
    }
}

impl Default for SecurityHeadersMiddleware {
    fn default() -> Self {
        Self::new(SecurityHeadersConfig::default_cached())
    }
}

/// Security headers layer
#[derive(Clone, Default)]
pub struct SecurityHeadersLayer {
    middleware: SecurityHeadersMiddleware,
}

impl SecurityHeadersLayer {
    pub fn new() -> Self {
        Self {
            middleware: SecurityHeadersMiddleware::default(),
        }
    }
}

impl<S> tower::Layer<S> for SecurityHeadersLayer
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Service = SecurityHeadersService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityHeadersService {
            inner,
            middleware: self.middleware.clone(),
        }
    }
}

/// Security headers service
#[derive(Clone)]
pub struct SecurityHeadersService<S> {
    inner: S,
    #[allow(dead_code)]
    middleware: SecurityHeadersMiddleware,
}

impl<S> tower::Service<Request> for SecurityHeadersService<S>
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        // For now, just pass through - security headers can be added later
        self.inner.call(request)
    }
}

/// Create security headers middleware with cached config
pub fn create_security_headers_middleware() -> SecurityHeadersLayer {
    SecurityHeadersLayer::new()
}

impl SecurityHeadersMiddleware {
    /// Layer method with explicit config (for optimized creation functions)
    pub async fn layer_with_config(
        request: Request,
        next: Next,
        config: Arc<SecurityHeadersConfig>,
    ) -> Response {
        let mut response = next.run(request).await;

        let headers = response.headers_mut();

        // Add security headers
        if let Some(csp) = &config.content_security_policy {
            headers.insert(header::CONTENT_SECURITY_POLICY, csp.parse().unwrap());
        }

        if let Some(max_age) = config.hsts_max_age {
            let mut hsts_value = format!("max-age={}", max_age);
            if config.hsts_include_subdomains {
                hsts_value.push_str("; includeSubDomains");
            }
            headers.insert(
                header::STRICT_TRANSPORT_SECURITY,
                hsts_value.parse().unwrap(),
            );
        }

        if let Some(xfo) = &config.x_frame_options {
            headers.insert("x-frame-options", xfo.parse().unwrap());
        }

        if let Some(xcto) = &config.x_content_type_options {
            headers.insert("x-content-type-options", xcto.parse().unwrap());
        }

        if let Some(rp) = &config.referrer_policy {
            headers.insert("referrer-policy", rp.parse().unwrap());
        }

        if let Some(pp) = &config.permissions_policy {
            headers.insert("permissions-policy", pp.parse().unwrap());
        }

        if let Some(coep) = &config.cross_origin_embedder_policy {
            headers.insert("cross-origin-embedder-policy", coep.parse().unwrap());
        }

        if let Some(coop) = &config.cross_origin_opener_policy {
            headers.insert("cross-origin-opener-policy", coop.parse().unwrap());
        }

        if let Some(corp) = &config.cross_origin_resource_policy {
            headers.insert("cross-origin-resource-policy", corp.parse().unwrap());
        }

        response
    }

    pub async fn layer(&self, request: Request, next: Next) -> Response {
        Self::layer_with_config(request, next, Arc::clone(&self.config)).await
    }
}

/// Request sanitization middleware to prevent common attacks
#[derive(Clone)]
pub struct RequestSanitizationMiddleware;

impl RequestSanitizationMiddleware {
    /// Sanitize request headers and path
    fn sanitize_request_parts(headers: &mut HeaderMap, uri: &mut Uri) -> Result<(), StatusCode> {
        // Sanitize Host header
        if let Some(host) = headers.get(header::HOST) {
            let host_str = host.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;
            if host_str.contains('\0') || host_str.contains('\r') || host_str.contains('\n') {
                return Err(StatusCode::BAD_REQUEST);
            }
        }

        // Sanitize User-Agent header
        if let Some(ua) = headers.get(header::USER_AGENT) {
            let ua_str = ua.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;
            if ua_str.contains('\0') {
                return Err(StatusCode::BAD_REQUEST);
            }
        }

        // Sanitize URI path
        let path = uri.path();
        if path.contains('\0') || path.contains('\r') || path.contains('\n') {
            return Err(StatusCode::BAD_REQUEST);
        }

        // Check for path traversal attempts
        if path.contains("../") || path.contains("..\\") {
            return Err(StatusCode::BAD_REQUEST);
        }

        Ok(())
    }

    pub async fn layer(request: Request, next: Next) -> Response {
        let (mut parts, body) = request.into_parts();

        // Sanitize request
        if let Err(status) = Self::sanitize_request_parts(&mut parts.headers, &mut parts.uri) {
            return (status, "Bad Request").into_response();
        }

        let request = Request::from_parts(parts, body);
        next.run(request).await
    }
}

/// Request sanitization layer
#[derive(Clone)]
pub struct RequestSanitizationLayer;

impl<S> tower::Layer<S> for RequestSanitizationLayer
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Service = RequestSanitizationService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestSanitizationService { inner }
    }
}

/// Request sanitization service
#[derive(Clone)]
pub struct RequestSanitizationService<S> {
    inner: S,
}

impl<S> tower::Service<Request> for RequestSanitizationService<S>
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
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let (mut parts, body) = request.into_parts();

            // Sanitize request
            if let Err(status) = RequestSanitizationMiddleware::sanitize_request_parts(
                &mut parts.headers,
                &mut parts.uri,
            ) {
                return Ok((status, "Bad Request").into_response());
            }

            let request = Request::from_parts(parts, body);
            inner
                .call(request)
                .await
                .map_err(|e| Box::new(e) as tower::BoxError)
        })
    }
}

/// Create request sanitization middleware
pub fn create_request_sanitization_middleware() -> RequestSanitizationLayer {
    RequestSanitizationLayer
}
