//! Audit middleware implementation
//!
//! This module contains the middleware logic for audit logging.

use axum::{
    extract::Request,
    http::{Method, Uri},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use super::audit_config::AuditConfig;
use super::audit_types::{AuditEventType, AuditLogEntry, AuditLogger};
use crate::domain::authorization::UserContext;

/// Audit middleware for logging security events
#[derive(Clone)]
pub struct AuditMiddleware {
    logger: Arc<dyn AuditLogger>,
}

impl AuditMiddleware {
    pub fn new(logger: Arc<dyn AuditLogger>) -> Self {
        Self { logger }
    }

    pub async fn layer(&self, request: Request, next: Next) -> Response {
        // Use default config for backward compatibility
        let config = AuditConfig::default();
        self.layer_with_config(request, next, &config).await
    }

    /// Layer with explicit config for performance optimization
    pub async fn layer_with_config(
        &self,
        request: Request,
        next: Next,
        config: &AuditConfig,
    ) -> Response {
        let start_time = std::time::Instant::now();

        // Extract request information
        let method = request.method().clone();
        let uri = request.uri().clone();
        let headers = request.headers();

        // Extract IP address
        let ip_address = extract_ip_address(headers);

        // Extract User-Agent
        let user_agent = headers
            .get("user-agent")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        // Clone user context for logging (if available)
        let user_context = request.extensions().get::<UserContext>().cloned();

        // Process the request
        let response = next.run(request).await;

        let response_time = start_time.elapsed().as_millis();
        let status_code = response.status().as_u16();

        // Determine event type
        let event_type = determine_event_type(&method, &uri, status_code);

        // Skip logging if event type is excluded by config
        if !config.should_log_event(&format!("{:?}", event_type)) {
            return response;
        }

        // Create audit log entry
        let mut entry = AuditLogEntry {
            timestamp: chrono::Utc::now(),
            event_type,
            user_id: user_context.as_ref().map(|ctx| ctx.user_id.clone()),
            tenant_id: user_context.as_ref().map(|ctx| ctx.tenant_id.clone()),
            api_key_id: user_context.as_ref().and_then(|ctx| ctx.api_key_id.clone()),
            ip_address: Some(ip_address),
            user_agent,
            method: method.to_string(),
            path: uri.path().to_string(),
            query: uri.query().map(|s| s.to_string()),
            status_code: Some(status_code),
            response_time_ms: Some(response_time),
            error_message: None,
            additional_data: None,
        };

        // Add error message for failed requests
        if status_code >= 400 {
            entry.error_message = Some(format!("HTTP {}", status_code));
        }

        // Log the event asynchronously (don't block the response)
        let logger = Arc::clone(&self.logger);
        tokio::spawn(async move {
            if let Err(e) = logger.log_event(entry).await {
                tracing::error!("Failed to log audit event: {}", e);
            }
        });

        response
    }
}

/// Extract IP address from request headers
fn extract_ip_address(headers: &axum::http::HeaderMap) -> String {
    // Try X-Forwarded-For first (for proxies/load balancers)
    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded_for.to_str() {
            // Take the first IP in case of multiple
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    // Try X-Real-IP
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }

    // Try X-Forwarded
    if let Some(forwarded) = headers.get("x-forwarded") {
        if let Ok(ip_str) = forwarded.to_str() {
            return ip_str.to_string();
        }
    }

    // Fallback to "unknown"
    "unknown".to_string()
}

/// Determine event type based on request/response
fn determine_event_type(method: &Method, uri: &Uri, status_code: u16) -> AuditEventType {
    let path = uri.path();

    // Health check endpoints
    if path == "/health" || path.starts_with("/health/") {
        return AuditEventType::HealthCheck;
    }

    // API key management
    if path.starts_with("/v1/api-keys") {
        return match *method {
            Method::POST => AuditEventType::ApiKeyCreated,
            Method::GET => AuditEventType::PermissionChecked,
            Method::PUT => AuditEventType::ApiKeyUpdated,
            Method::DELETE => AuditEventType::ApiKeyDeleted,
            _ => AuditEventType::PermissionChecked,
        };
    }

    // Object operations
    if path.starts_with("/v1/objects") {
        return match *method {
            Method::POST => AuditEventType::ObjectCreated,
            Method::GET => AuditEventType::ObjectRead,
            Method::PUT => AuditEventType::ObjectUpdated,
            Method::DELETE => AuditEventType::ObjectDeleted,
            _ => AuditEventType::PermissionChecked,
        };
    }

    // Authentication failures
    if status_code == 401 {
        return AuditEventType::AuthenticationFailure;
    }

    // Authorization failures
    if status_code == 403 {
        return AuditEventType::AuthorizationDenied;
    }

    // Rate limiting
    if status_code == 429 {
        return AuditEventType::RateLimitExceeded;
    }

    // Suspicious requests (4xx errors)
    if (400..500).contains(&status_code) {
        return AuditEventType::SuspiciousRequest;
    }

    // Invalid input (4xx client errors)
    if (400..500).contains(&status_code) {
        return AuditEventType::InvalidInput;
    }

    // Default to permission check for other requests
    AuditEventType::PermissionChecked
}

// TODO: Re-enable audit middleware when tower layer compatibility is resolved
/*
pub fn create_audit_middleware(
    audit_repo: std::sync::Arc<dyn crate::application::ports::AuditRepository>,
    config: &AuditConfig,
) -> impl tower::Layer<
    axum::middleware::Next,
    Service = tower::util::BoxCloneService<Request, Response, tower::BoxError>
> + Clone {
    let logger: Arc<dyn AuditLogger> = Arc::new(super::audit_loggers::DatabaseAuditLogger::new(audit_repo));
    let config = config.clone();

    tower::layer::layer_fn(move |mut request: Request, next: axum::middleware::Next| {
        let logger = Arc::clone(&logger);
        let config = config.clone();
        async move {
            let start_time = std::time::Instant::now();

            // Extract request information
            let method = request.method().clone();
            let uri = request.uri().clone();
            let headers = request.headers();

            // Extract IP address
            let ip_address = extract_ip_address(headers);

            // Extract User-Agent
            let user_agent = headers
                .get("user-agent")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            // Clone user context for logging (if available)
            let user_context = request.extensions().get::<UserContext>().cloned();

            // Process the request
            let response = next.run(request).await;

            let response_time = start_time.elapsed().as_millis();
            let status_code = response.status().as_u16();

            // Determine event type
            let event_type = determine_event_type(&method, &uri, status_code);

            // Skip logging if event type is excluded by config
            if !config.should_log_event(&format!("{:?}", event_type)) {
                return Ok(response);
            }

            // Create audit log entry
            let entry = AuditLogEntry {
                timestamp: chrono::Utc::now(),
                event_type,
                user_id: user_context.as_ref().map(|ctx| ctx.user_id.clone()),
                tenant_id: user_context.as_ref().map(|ctx| ctx.tenant_id.clone()),
                api_key_id: user_context.as_ref().and_then(|ctx| ctx.api_key_id.clone()),
                ip_address: Some(ip_address),
                user_agent,
                method: method.to_string(),
                path: uri.path().to_string(),
                query: uri.query().map(|s| s.to_string()),
                status_code: Some(status_code),
                response_time_ms: Some(response_time as u128),
                error_message: None,
                additional_data: None,
            };

            // Log the event asynchronously (don't block the response)
            tokio::spawn(async move {
                if let Err(e) = logger.log_event(entry).await {
                    tracing::error!("Failed to log audit event: {}", e);
                }
            });

            Ok(response)
        }
    })
}
*/
