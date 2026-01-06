// Re-export types and functions from the split modules for backward compatibility

pub use super::security_config::SecurityHeadersConfig;
pub use super::security_headers_impl::{
    create_request_sanitization_middleware, create_security_headers_middleware,
    RequestSanitizationMiddleware, SecurityHeadersMiddleware,
};
