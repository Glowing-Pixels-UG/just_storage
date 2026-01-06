use axum::{extract::Request, http::StatusCode, response::Response};

use super::config::ErrorHandlingConfig;
use super::sanitizers::ErrorSanitizer;
use super::utils::ErrorUtils;


/// Error handling middleware layer
#[derive(Clone)]
pub struct ErrorHandlingLayer;

impl<S> tower::Layer<S> for ErrorHandlingLayer
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Service = ErrorHandlingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ErrorHandlingService { inner }
    }
}

/// Error handling service wrapper
#[derive(Clone)]
pub struct ErrorHandlingService<S> {
    inner: S,
}

impl<S> tower::Service<Request> for ErrorHandlingService<S>
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    S::Response: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let uri = req.uri().clone();
            let method = req.method().clone();

            let result = inner.call(req).await;

            // Check if this is an error response that needs sanitization
            match result {
                Ok(response) => {
                    if response.status().is_client_error() || response.status().is_server_error() {
                        let sanitized_response = Self::sanitize_error_response(
                            response,
                            &uri,
                            &method,
                            &ErrorHandlingConfig::default(),
                        )
                        .await;

                        // Log the error for monitoring (but without sensitive details in production)
                        ErrorUtils::log_error(
                            &sanitized_response,
                            &uri,
                            &method,
                            &ErrorHandlingConfig::default(),
                        );

                        Ok(sanitized_response)
                    } else {
                        Ok(response)
                    }
                }
                Err(err) => Err(err),
            }
        })
    }
}

impl<S> ErrorHandlingService<S> {
    async fn sanitize_error_response(
        response: Response,
        uri: &axum::http::Uri,
        method: &axum::http::Method,
        config: &ErrorHandlingConfig,
    ) -> Response {
        let status = response.status();

        // For certain status codes, we want to provide generic responses
        match status {
            StatusCode::INTERNAL_SERVER_ERROR => {
                // Always return a generic 500 error
                ErrorSanitizer::create_generic_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error",
                    Some("INTERNAL_ERROR"),
                )
            }
            StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => {
                // For client errors, preserve some details but sanitize sensitive information
                Self::sanitize_client_error(response, uri, method, config).await
            }
            StatusCode::UNAUTHORIZED => ErrorSanitizer::create_generic_error_response(
                StatusCode::UNAUTHORIZED,
                "Authentication required",
                Some("AUTHENTICATION_REQUIRED"),
            ),
            StatusCode::FORBIDDEN => ErrorSanitizer::create_generic_error_response(
                StatusCode::FORBIDDEN,
                "Access denied",
                Some("ACCESS_DENIED"),
            ),
            StatusCode::NOT_FOUND => ErrorSanitizer::create_generic_error_response(
                StatusCode::NOT_FOUND,
                "Resource not found",
                Some("NOT_FOUND"),
            ),
            _ => response, // For other status codes, return as-is
        }
    }

    async fn sanitize_client_error(
        response: Response,
        _uri: &axum::http::Uri,
        _method: &axum::http::Method,
        _config: &ErrorHandlingConfig,
    ) -> Response {
        // For now, just return the response as-is
        // In the future, we could parse the body and sanitize sensitive information
        response
    }
}

/// Create error handling middleware
pub fn create_error_handling_middleware() -> ErrorHandlingLayer {
    ErrorHandlingLayer
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json;

    #[test]
    fn test_error_handling_config() {
        let config = ErrorHandlingConfig::default();

        // In development, include debug info
        #[cfg(debug_assertions)]
        assert!(config.include_debug_info);

        #[cfg(not(debug_assertions))]
        assert!(!config.include_debug_info);

        assert!(config.log_sensitive_errors);
        assert!(!config.sensitive_patterns.is_empty());

        // Check that common sensitive patterns are included
        assert!(config.sensitive_patterns.contains("password"));
        assert!(config.sensitive_patterns.contains("token"));
        assert!(config.sensitive_patterns.contains("key"));
        assert!(config.sensitive_patterns.contains("secret"));
    }

    #[test]
    fn test_sensitive_pattern_detection() {
        let config = ErrorHandlingConfig::default();

        // Test the patterns are detected
        assert!(config.sensitive_patterns.contains("password"));
        assert!(config.sensitive_patterns.contains("token"));
        assert!(config.sensitive_patterns.contains("key"));
        assert!(config.sensitive_patterns.contains("secret"));
        assert!(config.sensitive_patterns.contains("database"));
        assert!(config.sensitive_patterns.contains("connection"));
        assert!(config.sensitive_patterns.contains("sql"));
    }

    #[test]
    fn test_error_message_patterns() {
        let config = ErrorHandlingConfig::default();

        // Test that messages containing sensitive patterns are sanitized
        assert_eq!(
            ErrorSanitizer::sanitize_error_message(
                "Connection string: postgresql://user:secret@localhost/db",
                &config
            ),
            "An error occurred"
        );

        assert_eq!(
            ErrorSanitizer::sanitize_error_message("API key: sk-1234567890abcdef", &config),
            "An error occurred"
        );

        assert_eq!(
            ErrorSanitizer::sanitize_error_message("Password: mySecretPassword123", &config),
            "An error occurred"
        );

        // Test that non-sensitive messages pass through
        assert_eq!(
            ErrorSanitizer::sanitize_error_message("Invalid input format", &config),
            "Invalid input format"
        );

        assert_eq!(
            ErrorSanitizer::sanitize_error_message("Required field missing: email", &config),
            "Required field missing: email"
        );
    }
}
