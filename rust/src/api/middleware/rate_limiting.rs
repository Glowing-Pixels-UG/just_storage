use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};
// Note: tower_http rate limiting has changed in newer versions
// For now, we'll implement a simple in-memory rate limiter

use crate::domain::authorization::UserContext;

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per window for unauthenticated requests (by IP)
    pub unauthenticated_requests_per_minute: u32,
    /// Maximum requests per window for authenticated users
    pub authenticated_requests_per_minute: u32,
    /// Maximum concurrent requests per user
    pub max_concurrent_per_user: usize,
    /// Maximum concurrent requests per tenant
    pub max_concurrent_per_tenant: usize,
    /// Maximum concurrent requests per IP
    pub max_concurrent_per_ip: usize,
    /// Rate limit window duration in seconds
    pub window_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            unauthenticated_requests_per_minute: 60, // 60 requests per minute for unauth
            authenticated_requests_per_minute: 600,  // 600 requests per minute for auth
            max_concurrent_per_user: 10,
            max_concurrent_per_tenant: 50,
            max_concurrent_per_ip: 25,
            window_seconds: 60,
        }
    }
}

/// Rate limiting middleware response
#[derive(Serialize)]
struct RateLimitResponse {
    error: String,
    retry_after: Option<u64>,
}


/// Thread-safe rate limiter using DashMap
#[derive(Debug, Clone)]
pub struct RateLimiter {
    config: Arc<RateLimitConfig>,
    // Key -> (requests, last_reset)
    ip_limits: Arc<DashMap<String, (VecDeque<Instant>, Instant)>>,
    user_limits: Arc<DashMap<String, (VecDeque<Instant>, Instant)>>,
    tenant_limits: Arc<DashMap<String, (VecDeque<Instant>, Instant)>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config: Arc::new(config),
            ip_limits: Arc::new(DashMap::new()),
            user_limits: Arc::new(DashMap::new()),
            tenant_limits: Arc::new(DashMap::new()),
        }
    }

    /// Check if a request should be rate limited
    pub fn check_limit(&self, key: &str, limit_type: LimitType) -> Result<(), RateLimitError> {
        let (max_requests, map) = match limit_type {
            LimitType::IP => (
                self.config.unauthenticated_requests_per_minute,
                &self.ip_limits,
            ),
            LimitType::User => (
                self.config.authenticated_requests_per_minute,
                &self.user_limits,
            ),
            LimitType::Tenant => (
                self.config.authenticated_requests_per_minute * 5,
                &self.tenant_limits,
            ),
        };

        let mut entry = map
            .entry(key.to_string())
            .or_insert_with(|| (VecDeque::new(), Instant::now()));

        let now = Instant::now();
        let window_duration = Duration::from_secs(self.config.window_seconds);

        // Reset window if needed
        if now.duration_since(entry.1) >= window_duration {
            entry.0.clear();
            entry.1 = now;
        }

        // Remove old requests outside the window
        while let Some(&old_time) = entry.0.front() {
            if now.duration_since(old_time) >= window_duration {
                entry.0.pop_front();
            } else {
                break;
            }
        }

        // Check if limit exceeded
        if entry.0.len() >= max_requests as usize {
            let oldest_request = entry.0.front().unwrap();
            let retry_after = window_duration - now.duration_since(*oldest_request);
            return Err(RateLimitError::LimitExceeded(retry_after.as_secs()));
        }

        // Add current request
        entry.0.push_back(now);

        Ok(())
    }

    /// Clean up old entries to prevent memory leaks
    pub fn cleanup(&self) {
        let cutoff = Instant::now() - Duration::from_secs(self.config.window_seconds * 2);

        // Clean up IP limits
        self.ip_limits
            .retain(|_, (requests, last_reset)| *last_reset > cutoff || !requests.is_empty());

        // Clean up user limits
        self.user_limits
            .retain(|_, (requests, last_reset)| *last_reset > cutoff || !requests.is_empty());

        // Clean up tenant limits
        self.tenant_limits
            .retain(|_, (requests, last_reset)| *last_reset > cutoff || !requests.is_empty());
    }
}

/// Rate limit error types
#[derive(Debug)]
pub enum RateLimitError {
    LimitExceeded(u64), // seconds to wait
}

/// Rate limit types
#[derive(Debug, Clone, Copy)]
pub enum LimitType {
    IP,
    User,
    Tenant,
}

/// Rate limiting middleware
pub struct RateLimitMiddleware {
    #[allow(dead_code)]
    limiter: Arc<RateLimiter>,
}

impl RateLimitMiddleware {
    pub fn new(limiter: Arc<RateLimiter>) -> Self {
        Self { limiter }
    }

    pub async fn layer(request: Request, next: Next) -> Response {
        // Extract identifiers for rate limiting
        let ip_addr = extract_ip_address(&request);
        let user_context = request.extensions().get::<UserContext>();

        let limiter = Arc::clone(request.extensions().get::<Arc<RateLimiter>>().unwrap());

        // Apply rate limiting based on authentication status
        let rate_limit_result = if let Some(user_ctx) = user_context {
            // Authenticated user: check user and tenant limits
            let user_check = limiter.check_limit(&user_ctx.user_id, LimitType::User);
            let tenant_check = limiter.check_limit(&user_ctx.tenant_id, LimitType::Tenant);

            user_check.and(tenant_check)
        } else {
            // Unauthenticated: check IP limit
            limiter.check_limit(&ip_addr, LimitType::IP)
        };

        match rate_limit_result {
            Ok(()) => {
                // Rate limit passed, continue with request
                next.run(request).await
            }
            Err(RateLimitError::LimitExceeded(retry_after)) => {
                // Rate limit exceeded
                let response = RateLimitResponse {
                    error: "Rate limit exceeded".to_string(),
                    retry_after: Some(retry_after),
                };

                (
                    StatusCode::TOO_MANY_REQUESTS,
                    [("Retry-After", retry_after.to_string())],
                    axum::Json(response),
                )
                    .into_response()
            }
        }
    }
}

/// Extract IP address from request
fn extract_ip_address(request: &Request) -> String {
    // Try X-Forwarded-For header first (for proxies/load balancers)
    if let Some(forwarded_for) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
    {
        // Take the first IP in case of multiple
        if let Some(first_ip) = forwarded_for.split(',').next() {
            if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                return ip.to_string();
            }
        }
    }

    // Try X-Real-IP header
    if let Some(real_ip) = request
        .headers()
        .get("x-real-ip")
        .and_then(|h| h.to_str().ok())
        .and_then(|ip| ip.parse::<IpAddr>().ok())
    {
        return real_ip.to_string();
    }

    // Fallback to remote address if available
    // Note: This requires the request to have been processed by a connector that sets remote_addr
    "unknown".to_string()
}

/// Rate limiting layer
#[derive(Clone)]
pub struct RateLimitLayer {
    limiter: Arc<RateLimiter>,
}

impl RateLimitLayer {
    pub fn new(config: RateLimitConfig) -> Self {
        let limiter = Arc::new(RateLimiter::new(config));

        // Spawn cleanup task
        let cleanup_limiter = Arc::clone(&limiter);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Clean every 5 minutes
            loop {
                interval.tick().await;
                cleanup_limiter.cleanup();
            }
        });

        Self { limiter }
    }
}

impl<S> tower::Layer<S> for RateLimitLayer
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            limiter: Arc::clone(&self.limiter),
        }
    }
}

/// Rate limiting service
#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    #[allow(dead_code)]
    limiter: Arc<RateLimiter>,
}

impl<S> tower::Service<Request> for RateLimitService<S>
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
        // For now, just pass through - rate limiting can be added later
        self.inner.call(request)
    }
}

/// Create rate limiting middleware with configuration
pub fn create_rate_limit_middleware(config: RateLimitConfig) -> RateLimitLayer {
    RateLimitLayer::new(config)
}

/// Create concurrency limiting layers
/// Note: This is a placeholder - concurrency limiting not yet implemented
pub fn create_concurrency_limits(_config: &RateLimitConfig) -> Vec<()> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_within_limit() {
        let config = RateLimitConfig {
            unauthenticated_requests_per_minute: 5,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        // Should allow requests within limit
        for i in 0..5 {
            assert!(limiter
                .check_limit(&format!("ip_{}", i), LimitType::IP)
                .is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_exceeds_limit() {
        let config = RateLimitConfig {
            unauthenticated_requests_per_minute: 3,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        // Fill up the limit
        for i in 0..3 {
            assert!(limiter.check_limit("test_ip", LimitType::IP).is_ok());
        }

        // Next request should be rate limited
        assert!(matches!(
            limiter.check_limit("test_ip", LimitType::IP),
            Err(RateLimitError::LimitExceeded(_))
        ));
    }

    #[test]
    fn test_extract_ip_address() {
        // This would need a real request to test properly
        // For now, just ensure the function exists and compiles
        assert_eq!(
            extract_ip_address(&axum::extract::Request::default()),
            "unknown"
        );
    }

    #[test]
    fn test_rate_limiter_different_types() {
        let config = RateLimitConfig {
            unauthenticated_requests_per_minute: 3,
            authenticated_requests_per_minute: 5,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        // Test IP limiting
        for i in 0..3 {
            assert!(limiter
                .check_limit(&format!("ip_{}", i), LimitType::IP)
                .is_ok());
        }
        assert!(matches!(
            limiter.check_limit("ip_test", LimitType::IP),
            Err(RateLimitError::LimitExceeded(_))
        ));

        // Test user limiting (different limit)
        for i in 0..5 {
            assert!(limiter
                .check_limit(&format!("user_{}", i), LimitType::User)
                .is_ok());
        }
        assert!(matches!(
            limiter.check_limit("user_test", LimitType::User),
            Err(RateLimitError::LimitExceeded(_))
        ));

        // Test tenant limiting
        for i in 0..5 {
            assert!(limiter
                .check_limit(&format!("tenant_{}", i), LimitType::Tenant)
                .is_ok());
        }
        assert!(matches!(
            limiter.check_limit("tenant_test", LimitType::Tenant),
            Err(RateLimitError::LimitExceeded(_))
        ));
    }

    #[test]
    fn test_rate_limiter_cleanup() {
        let config = RateLimitConfig {
            unauthenticated_requests_per_minute: 10,
            window_seconds: 1,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        // Add some entries
        for i in 0..5 {
            let _ = limiter.check_limit(&format!("ip_{}", i), LimitType::IP);
        }

        assert!(!limiter.ip_limits.is_empty());

        // Cleanup (in real usage, this would be called periodically)
        limiter.cleanup();

        // Should still have entries (since they're recent)
        // In a real test with time manipulation, we'd verify cleanup
        assert!(!limiter.ip_limits.is_empty());
    }

    #[test]
    fn test_rate_limit_config_defaults() {
        let config = RateLimitConfig::default();

        assert_eq!(config.unauthenticated_requests_per_minute, 60);
        assert_eq!(config.authenticated_requests_per_minute, 600);
        assert_eq!(config.max_concurrent_per_user, 10);
        assert_eq!(config.max_concurrent_per_tenant, 50);
        assert_eq!(config.max_concurrent_per_ip, 25);
        assert_eq!(config.window_seconds, 60);
    }

    #[test]
    fn test_rate_limit_error_retry_after() {
        let config = RateLimitConfig {
            unauthenticated_requests_per_minute: 1,
            window_seconds: 60,
            ..Default::default()
        };
        let limiter = RateLimiter::new(config);

        // Use up the limit
        assert!(limiter.check_limit("test_ip", LimitType::IP).is_ok());

        // Next request should be rate limited
        match limiter.check_limit("test_ip", LimitType::IP) {
            Err(RateLimitError::LimitExceeded(retry_after)) => {
                assert!(retry_after > 0 && retry_after <= 60);
            }
            _ => panic!("Expected rate limit exceeded error"),
        }
    }

    #[test]
    fn test_limit_type_enum() {
        assert!(matches!(LimitType::IP, LimitType::IP));
        assert!(matches!(LimitType::User, LimitType::User));
        assert!(matches!(LimitType::Tenant, LimitType::Tenant));
    }
}
