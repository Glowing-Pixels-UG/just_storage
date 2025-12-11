use serde::{Deserialize, Serialize};

/// Object lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ObjectStatus {
    /// Upload in progress (reserved)
    Writing,
    /// Upload complete and committed
    Committed,
    /// Marked for deletion
    Deleting,
    /// Deleted (tombstone)
    Deleted,
}

impl ObjectStatus {
    /// Check if transition is valid
    pub fn can_transition_to(&self, target: ObjectStatus) -> bool {
        matches!(
            (self, target),
            (ObjectStatus::Writing, ObjectStatus::Committed)
                | (ObjectStatus::Committed, ObjectStatus::Deleting)
                | (ObjectStatus::Deleting, ObjectStatus::Deleted)
        )
    }
}

impl std::fmt::Display for ObjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectStatus::Writing => write!(f, "WRITING"),
            ObjectStatus::Committed => write!(f, "COMMITTED"),
            ObjectStatus::Deleting => write!(f, "DELETING"),
            ObjectStatus::Deleted => write!(f, "DELETED"),
        }
    }
}

impl std::str::FromStr for ObjectStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "WRITING" => Ok(ObjectStatus::Writing),
            "COMMITTED" => Ok(ObjectStatus::Committed),
            "DELETING" => Ok(ObjectStatus::Deleting),
            "DELETED" => Ok(ObjectStatus::Deleted),
            _ => Err(format!("Invalid object status: {}", s)),
        }
    }
}
