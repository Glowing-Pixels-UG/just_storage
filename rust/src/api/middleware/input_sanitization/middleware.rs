use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use once_cell::sync::Lazy;

/// Cached default configuration for performance
static DEFAULT_CONFIG: Lazy<std::sync::Arc<InputSanitizationConfig>> =
    Lazy::new(|| std::sync::Arc::new(InputSanitizationConfig::default()));

use super::config::InputSanitizationConfig;

/// Error type for input sanitization failures
#[derive(Debug)]
pub enum InputSanitizationError {
    InvalidHeader(String),
    InvalidUri(String),
    MalformedData(String),
}

impl IntoResponse for InputSanitizationError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            InputSanitizationError::InvalidHeader(msg) => (StatusCode::BAD_REQUEST, msg),
            InputSanitizationError::InvalidUri(msg) => (StatusCode::BAD_REQUEST, msg),
            InputSanitizationError::MalformedData(msg) => (StatusCode::BAD_REQUEST, msg),
        };
        (status, message).into_response()
    }
}



/// Create input sanitization middleware with cached config
use tower::Layer;

#[derive(Clone)]
pub struct InputSanitizationLayer {
    config: std::sync::Arc<InputSanitizationConfig>,
}

impl InputSanitizationLayer {
    pub fn new(config: InputSanitizationConfig) -> Self {
        Self {
            config: std::sync::Arc::new(config),
        }
    }

    pub fn with_cached_config() -> Self {
        Self {
            config: std::sync::Arc::clone(&DEFAULT_CONFIG),
        }
    }
}

impl<S> Layer<S> for InputSanitizationLayer
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Service = InputSanitizationService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        InputSanitizationService {
            inner,
            config: std::sync::Arc::clone(&self.config),
        }
    }
}

#[derive(Clone)]
pub struct InputSanitizationService<S> {
    inner: S,
    #[allow(dead_code)]
    config: std::sync::Arc<InputSanitizationConfig>,
}

impl<S> tower::Service<Request> for InputSanitizationService<S>
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
        // For now, just pass through - sanitization logic can be added later
        self.inner.call(request)
    }
}

pub fn create_input_sanitization_middleware() -> InputSanitizationLayer {
    InputSanitizationLayer::with_cached_config()
}
