use serde::{Deserialize, Serialize};

use crate::domain::errors::DomainError;

/// SHA-256 content hash (32 bytes = 64 hex chars)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(String);

impl Default for ContentHash {
    fn default() -> Self {
        // zero-hash is an unlikely but valid placeholder
        Self("0".repeat(64))
    }
}

impl ContentHash {
    /// Create from validated hex string
    pub fn from_hex(hex: String) -> Result<Self, DomainError> {
        if hex.len() != 64 {
            return Err(DomainError::ContentHashMismatch {
                expected: "64 hex characters".to_string(),
                actual: format!("{} characters", hex.len()),
            });
        }

        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(DomainError::ContentHashMismatch {
                expected: "hex characters only".to_string(),
                actual: hex,
            });
        }

        Ok(Self(hex.to_lowercase()))
    }

    /// Get hex string representation
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// Get first 2 characters for directory fan-out
    pub fn prefix(&self) -> &str {
        &self.0[0..2]
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ContentHash {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s.to_string())
    }
}
