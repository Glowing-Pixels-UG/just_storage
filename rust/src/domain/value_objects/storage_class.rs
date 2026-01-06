use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Storage tier for performance/cost trade-offs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum StorageClass {
    /// NVMe-backed fast storage
    #[default]
    Hot,
    /// HDD-backed slower storage
    Cold,
}

impl std::fmt::Display for StorageClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageClass::Hot => write!(f, "hot"),
            StorageClass::Cold => write!(f, "cold"),
        }
    }
}

impl std::str::FromStr for StorageClass {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hot" => Ok(StorageClass::Hot),
            "cold" => Ok(StorageClass::Cold),
            _ => Err(format!("Invalid storage class: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_storage_class_display() {
        assert_eq!(format!("{}", StorageClass::Hot), "hot");
        assert_eq!(format!("{}", StorageClass::Cold), "cold");
    }

    #[test]
    fn test_storage_class_from_str_valid() {
        assert_eq!(StorageClass::from_str("hot").unwrap(), StorageClass::Hot);
        assert_eq!(StorageClass::from_str("cold").unwrap(), StorageClass::Cold);
        assert_eq!(StorageClass::from_str("HOT").unwrap(), StorageClass::Hot);
        assert_eq!(StorageClass::from_str("COLD").unwrap(), StorageClass::Cold);
    }

    #[test]
    fn test_storage_class_from_str_invalid() {
        assert!(StorageClass::from_str("invalid").is_err());
    }

    #[test]
    fn test_storage_class_default() {
        assert_eq!(StorageClass::default(), StorageClass::Hot);
    }
}
