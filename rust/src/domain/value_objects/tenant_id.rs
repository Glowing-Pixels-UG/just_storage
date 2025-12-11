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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use uuid::Uuid;

    #[test]
    fn test_tenant_id_new() {
        let uuid = Uuid::new_v4();
        let tenant_id = TenantId::new(uuid);
        assert_eq!(tenant_id.as_uuid(), &uuid);
    }

    #[test]
    fn test_tenant_id_from_string_valid() {
        let uuid = Uuid::new_v4();
        let tenant_id = TenantId::from_string(&uuid.to_string()).unwrap();
        assert_eq!(tenant_id.as_uuid(), &uuid);
    }

    #[test]
    fn test_tenant_id_from_string_invalid() {
        let err = TenantId::from_string("invalid-uuid").unwrap_err();
        assert!(matches!(err, DomainError::InvalidTenantId(_)));
    }

    #[test]
    fn test_tenant_id_display() {
        let uuid = Uuid::new_v4();
        let tenant_id = TenantId::new(uuid);
        assert_eq!(format!("{}", tenant_id), uuid.to_string());
    }

    #[test]
    fn test_tenant_id_from_str_valid() {
        let uuid = Uuid::new_v4();
        let tenant_id = TenantId::from_str(&uuid.to_string()).unwrap();
        assert_eq!(tenant_id.as_uuid(), &uuid);
    }
}
