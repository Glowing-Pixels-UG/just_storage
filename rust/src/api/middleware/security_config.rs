//! Security headers configuration
//!
//! This module provides configuration structures and validation
//! for security headers middleware.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Cached default security headers configuration
static DEFAULT_CONFIG: Lazy<Arc<SecurityHeadersConfig>> =
    Lazy::new(|| Arc::new(SecurityHeadersConfig::default()));

/// Security headers configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityHeadersConfig {
    /// Content Security Policy
    pub content_security_policy: Option<String>,
    /// Strict Transport Security max age in seconds
    pub hsts_max_age: Option<u64>,
    /// Include subdomains in HSTS
    pub hsts_include_subdomains: bool,
    /// X-Frame-Options value
    pub x_frame_options: Option<String>,
    /// X-Content-Type-Options
    pub x_content_type_options: Option<String>,
    /// Referrer-Policy
    pub referrer_policy: Option<String>,
    /// Permissions-Policy (formerly Feature-Policy)
    pub permissions_policy: Option<String>,
    /// Cross-Origin-Embedder-Policy
    pub cross_origin_embedder_policy: Option<String>,
    /// Cross-Origin-Opener-Policy
    pub cross_origin_opener_policy: Option<String>,
    /// Cross-Origin-Resource-Policy
    pub cross_origin_resource_policy: Option<String>,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            content_security_policy: Some("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'".to_string()),
            hsts_max_age: Some(31536000), // 1 year
            hsts_include_subdomains: true,
            x_frame_options: Some("DENY".to_string()),
            x_content_type_options: Some("nosniff".to_string()),
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            permissions_policy: Some("camera=(), microphone=(), geolocation=(), interest-cohort=()".to_string()),
            cross_origin_embedder_policy: Some("require-corp".to_string()),
            cross_origin_opener_policy: Some("same-origin".to_string()),
            cross_origin_resource_policy: Some("same-origin".to_string()),
        }
    }
}

impl SecurityHeadersConfig {
    /// Create a secure production configuration
    pub fn production() -> Self {
        Self::default()
    }

    /// Create a permissive development configuration
    pub fn development() -> Self {
        Self {
            content_security_policy: Some("default-src * 'unsafe-inline' 'unsafe-eval'; script-src * 'unsafe-inline' 'unsafe-eval'; style-src * 'unsafe-inline'".to_string()),
            hsts_max_age: None, // Disable HSTS in development
            hsts_include_subdomains: false,
            x_frame_options: Some("SAMEORIGIN".to_string()),
            ..Self::default()
        }
    }

    /// Validate the security headers configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate Content Security Policy
        if let Some(csp) = &self.content_security_policy {
            if csp.trim().is_empty() {
                return Err("Content Security Policy cannot be empty".to_string());
            }
            // Basic CSP validation - check for dangerous patterns
            if csp.contains("unsafe-inline") && !csp.contains("script-src") {
                return Err(
                    "Content Security Policy with 'unsafe-inline' should specify script-src"
                        .to_string(),
                );
            }
        }

        // Validate HSTS max age
        if let Some(max_age) = self.hsts_max_age {
            if max_age == 0 {
                return Err("HSTS max-age must be greater than 0".to_string());
            }
            if max_age > 2147483647 {
                return Err("HSTS max-age is too large (maximum 2147483647)".to_string());
            }
        }

        // Validate X-Frame-Options
        if let Some(xfo) = &self.x_frame_options {
            let valid_values = ["DENY", "SAMEORIGIN"];
            let xfo_upper = xfo.to_uppercase();
            if !valid_values.contains(&xfo_upper.as_str()) {
                return Err(format!(
                    "Invalid X-Frame-Options value: {}. Valid values are: {}",
                    xfo,
                    valid_values.join(", ")
                ));
            }
        }

        // Validate other headers
        if let Some(rp) = &self.referrer_policy {
            let valid_policies = [
                "no-referrer",
                "no-referrer-when-downgrade",
                "origin",
                "origin-when-cross-origin",
                "same-origin",
                "strict-origin",
                "strict-origin-when-cross-origin",
                "unsafe-url",
            ];
            if !valid_policies.contains(&rp.as_str()) {
                return Err(format!("Invalid Referrer-Policy value: {}", rp));
            }
        }

        Ok(())
    }

    /// Get the cached default configuration
    pub fn default_cached() -> Arc<Self> {
        Arc::clone(&DEFAULT_CONFIG)
    }
}
