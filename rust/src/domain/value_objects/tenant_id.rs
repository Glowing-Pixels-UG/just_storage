use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::errors::DomainError;

/// Validated tenant identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(Uuid);

impl TenantId {
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn from_string(s: &str) -> Result<Self, DomainError> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|e| DomainError::InvalidTenantId(e.to_string()))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl std::fmt::Display for TenantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for TenantId {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_string(s)
    }
}
