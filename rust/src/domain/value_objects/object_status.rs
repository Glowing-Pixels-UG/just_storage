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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_object_status_can_transition_to_valid() {
        assert!(ObjectStatus::Writing.can_transition_to(ObjectStatus::Committed));
        assert!(ObjectStatus::Committed.can_transition_to(ObjectStatus::Deleting));
        assert!(ObjectStatus::Deleting.can_transition_to(ObjectStatus::Deleted));
    }

    #[test]
    fn test_object_status_can_transition_to_invalid() {
        assert!(!ObjectStatus::Writing.can_transition_to(ObjectStatus::Deleting));
        assert!(!ObjectStatus::Committed.can_transition_to(ObjectStatus::Writing));
        assert!(!ObjectStatus::Deleting.can_transition_to(ObjectStatus::Committed));
        assert!(!ObjectStatus::Deleted.can_transition_to(ObjectStatus::Writing));
    }

    #[test]
    fn test_object_status_display() {
        assert_eq!(format!("{}", ObjectStatus::Writing), "WRITING");
        assert_eq!(format!("{}", ObjectStatus::Committed), "COMMITTED");
        assert_eq!(format!("{}", ObjectStatus::Deleting), "DELETING");
        assert_eq!(format!("{}", ObjectStatus::Deleted), "DELETED");
    }

    #[test]
    fn test_object_status_from_str_valid() {
        assert_eq!(
            ObjectStatus::from_str("WRITING").unwrap(),
            ObjectStatus::Writing
        );
        assert_eq!(
            ObjectStatus::from_str("COMMITTED").unwrap(),
            ObjectStatus::Committed
        );
        assert_eq!(
            ObjectStatus::from_str("DELETING").unwrap(),
            ObjectStatus::Deleting
        );
        assert_eq!(
            ObjectStatus::from_str("DELETED").unwrap(),
            ObjectStatus::Deleted
        );
    }

    #[test]
    fn test_object_status_from_str_invalid() {
        assert!(ObjectStatus::from_str("INVALID").is_err());
    }
}
