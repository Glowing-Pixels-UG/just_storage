use serde::{Deserialize, Serialize};

use crate::domain::errors::DomainError;

/// Validated namespace identifier (e.g., "models", "datasets")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Namespace(String);

impl Namespace {
    const MAX_LENGTH: usize = 64;

    pub fn new(value: String) -> Result<Self, DomainError> {
        if value.is_empty() {
            return Err(DomainError::InvalidNamespace(
                "Namespace cannot be empty".to_string(),
            ));
        }

        if value.len() > Self::MAX_LENGTH {
            return Err(DomainError::InvalidNamespace(format!(
                "Namespace too long: {} > {}",
                value.len(),
                Self::MAX_LENGTH
            )));
        }

        // Must be alphanumeric with underscores/hyphens
        if !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(DomainError::InvalidNamespace(
                "Namespace must be alphanumeric with underscores/hyphens".to_string(),
            ));
        }

        Ok(Self(value.to_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Namespace {
    fn default() -> Self {
        // "default" is a safe, accepted namespace value
        Self("default".to_string())
    }
}

impl std::fmt::Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for Namespace {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}
