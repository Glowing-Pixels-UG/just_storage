use serde::{Deserialize, Serialize};

/// Storage tier for performance/cost trade-offs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
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
