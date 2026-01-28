use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthMiddlewareConfig {
    pub enabled: bool,
    pub legacy_auth_enabled: bool,
    pub admin_token: Option<String>,
}

impl AuthMiddlewareConfig {
    pub fn new(enabled: bool, legacy_auth_enabled: bool, admin_token: Option<String>) -> Self {
        Self { enabled, legacy_auth_enabled, admin_token }
    }
}
