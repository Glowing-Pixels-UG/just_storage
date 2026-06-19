use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OidcConfig {
    pub enabled: bool,
    pub issuer_url: Option<String>,
    pub audience: Option<String>,
}

impl OidcConfig {
    pub fn new(enabled: bool, issuer_url: Option<String>, audience: Option<String>) -> Self {
        Self {
            enabled,
            issuer_url,
            audience,
        }
    }
}
