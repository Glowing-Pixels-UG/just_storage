use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Unique identifier for an object
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[schema(value_type = String)]
pub struct ObjectId(Uuid);

impl ObjectId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ObjectId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_object_id_new_creates_unique_ids() {
        let id1 = ObjectId::new();
        let id2 = ObjectId::new();

        assert_ne!(id1, id2, "New ObjectIds should be unique");
    }

    #[test]
    fn test_object_id_from_uuid_round_trip() {
        let uuid = Uuid::new_v4();
        let object_id = ObjectId::from_uuid(uuid);
        let retrieved_uuid = object_id.as_uuid();

        assert_eq!(uuid, *retrieved_uuid);
    }

    #[test]
    fn test_object_id_display() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let object_id = ObjectId::from_uuid(uuid);

        assert_eq!(object_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_object_id_from_str_valid() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
            let object_id: ObjectId = uuid_str.parse().unwrap();
        let expected_uuid = Uuid::parse_str(uuid_str).unwrap();

        assert_eq!(*object_id.as_uuid(), expected_uuid);
    }

    #[test]
    fn test_object_id_from_str_invalid() {
        let invalid_uuids = vec![
            "",
            "not-a-uuid",
            "550e8400-e29b-41d4-a716", // too short
            "550e8400-e29b-41d4-a716-446655440000-extra", // too long
            "550e8400-e29b-41d4-a716-44665544000g", // invalid character
        ];

        for invalid in invalid_uuids {
            assert!(invalid.parse::<ObjectId>().is_err(),
                "Should fail to parse invalid UUID: {}", invalid);
        }
    }

    #[test]
    fn test_object_id_default() {
        let default1 = ObjectId::default();
        let default2 = ObjectId::default();

        // Default should create new IDs, so they should be different
        assert_ne!(default1, default2);
    }

    #[test]
    fn test_object_id_equality() {
        let uuid = Uuid::new_v4();
        let id1 = ObjectId::from_uuid(uuid);
        let id2 = ObjectId::from_uuid(uuid);
        let id3 = ObjectId::new();

        assert_eq!(id1, id2, "Same UUID should create equal ObjectIds");
        assert_ne!(id1, id3, "Different UUIDs should create unequal ObjectIds");
    }

    #[test]
    fn test_object_id_hash() {
        let uuid = Uuid::new_v4();
        let id1 = ObjectId::from_uuid(uuid);
        let id2 = ObjectId::from_uuid(uuid);

        let mut set = HashSet::new();
        set.insert(id1);

        assert!(set.contains(&id2), "Equal ObjectIds should have same hash");
    }

    #[test]
    fn test_object_id_clone() {
        let id1 = ObjectId::new();
        let id2 = id1.clone();

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_object_id_copy() {
        let id1 = ObjectId::new();
        let id2 = id1; // Copy

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_object_id_serialization() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let object_id = ObjectId::from_uuid(uuid);

        // Test JSON serialization
        let json = serde_json::to_string(&object_id).unwrap();
        assert_eq!(json, "\"550e8400-e29b-41d4-a716-446655440000\"");

        // Test JSON deserialization
        let deserialized: ObjectId = serde_json::from_str(&json).unwrap();
        assert_eq!(object_id, deserialized);
    }

    #[test]
    fn test_object_id_uniqueness_at_scale() {
        let mut ids = HashSet::new();

        // Generate many IDs and ensure they're all unique
        for _ in 0..1000 {
            let id = ObjectId::new();
            assert!(!ids.contains(&id), "Generated ObjectId should be unique");
            ids.insert(id);
        }

        assert_eq!(ids.len(), 1000, "All generated IDs should be unique");
    }
}
