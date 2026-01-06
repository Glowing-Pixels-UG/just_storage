use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// API key identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[schema(value_type = String)]
pub struct ApiKeyId(Uuid);

impl ApiKeyId {
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

impl Default for ApiKeyId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ApiKeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ApiKeyId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// API key value (the actual secret)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApiKeyValue(String);

impl ApiKeyValue {
    /// Generate a new random API key
    pub fn generate() -> Self {
        use rand::{distr::Alphanumeric, Rng};
        let key: String = rand::rng()
            .sample_iter(Alphanumeric)
            .take(64)
            .map(char::from)
            .collect();
        Self(key)
    }

    /// Create from existing string (for loading from DB)
    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    /// Get the key as a string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the key as owned string (for API responses, etc.)
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for ApiKeyValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// API key permissions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyPermissions {
    pub read: bool,
    pub write: bool,
    pub delete: bool,
    pub admin: bool,
}

impl Default for ApiKeyPermissions {
    fn default() -> Self {
        Self {
            read: true,
            write: true,
            delete: true,
            admin: false,
        }
    }
}

impl ApiKeyPermissions {
    pub fn full_access() -> Self {
        Self {
            read: true,
            write: true,
            delete: true,
            admin: true,
        }
    }

    pub fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            delete: false,
            admin: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for ApiKeyId
    mod api_key_id_tests {
        use super::*;

        #[test]
        fn test_api_key_id_new_creates_unique_ids() {
            let id1 = ApiKeyId::new();
            let id2 = ApiKeyId::new();

            assert_ne!(id1, id2, "New ApiKeyIds should be unique");
        }

        #[test]
        fn test_api_key_id_from_uuid_round_trip() {
            let uuid = Uuid::new_v4();
            let api_key_id = ApiKeyId::from_uuid(uuid);
            let retrieved_uuid = api_key_id.as_uuid();

            assert_eq!(uuid, *retrieved_uuid);
        }

        #[test]
        fn test_api_key_id_display() {
            let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
            let api_key_id = ApiKeyId::from_uuid(uuid);

            assert_eq!(api_key_id.to_string(), "550e8400-e29b-41d4-a716-446655440000");
        }

        #[test]
        fn test_api_key_id_from_str_valid() {
            let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
            let api_key_id: ApiKeyId = uuid_str.parse().unwrap();
            let expected_uuid = Uuid::parse_str(uuid_str).unwrap();

            assert_eq!(*api_key_id.as_uuid(), expected_uuid);
        }

        #[test]
        fn test_api_key_id_from_str_invalid() {
            let invalid_uuids = vec![
                "",
                "not-a-uuid",
                "550e8400-e29b-41d4-a716", // too short
            ];

        for invalid in invalid_uuids {
            assert!(invalid.parse::<ApiKeyId>().is_err(),
                "Should fail to parse invalid UUID: {}", invalid);
        }
        }

        #[test]
        fn test_api_key_id_default() {
            let default1 = ApiKeyId::default();
            let default2 = ApiKeyId::default();

            // Default should create new IDs, so they should be different
            assert_ne!(default1, default2);
        }

        #[test]
        fn test_api_key_id_serialization() {
            let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
            let api_key_id = ApiKeyId::from_uuid(uuid);

            // Test JSON serialization
            let json = serde_json::to_string(&api_key_id).unwrap();
            assert_eq!(json, "\"550e8400-e29b-41d4-a716-446655440000\"");

            // Test JSON deserialization
            let deserialized: ApiKeyId = serde_json::from_str(&json).unwrap();
            assert_eq!(api_key_id, deserialized);
        }
    }

    // Tests for ApiKeyValue
    mod api_key_value_tests {
        use super::*;

        #[test]
        fn test_api_key_value_generate_creates_64_char_key() {
            let key = ApiKeyValue::generate();

            assert_eq!(key.as_str().len(), 64, "Generated key should be 64 characters");
            assert!(key.as_str().chars().all(|c| c.is_ascii_alphanumeric()),
                "Generated key should contain only alphanumeric characters");
        }

        #[test]
        fn test_api_key_value_generate_creates_unique_keys() {
            let key1 = ApiKeyValue::generate();
            let key2 = ApiKeyValue::generate();

            assert_ne!(key1.as_str(), key2.as_str(), "Generated keys should be unique");
        }

        #[test]
        fn test_api_key_value_from_string() {
            let test_string = "test-api-key-123".to_string();
            let key = ApiKeyValue::from_string(test_string.clone());

            assert_eq!(key.as_str(), test_string);
        }

        #[test]
        fn test_api_key_value_into_string() {
            let test_string = "test-api-key-456".to_string();
            let key = ApiKeyValue::from_string(test_string.clone());
            let extracted = key.into_string();

            assert_eq!(extracted, test_string);
        }

        #[test]
        fn test_api_key_value_display() {
            let test_string = "display-test-key".to_string();
            let key = ApiKeyValue::from_string(test_string.clone());

            assert_eq!(key.to_string(), test_string);
        }

        #[test]
        fn test_api_key_value_equality() {
            let key1 = ApiKeyValue::from_string("same-key".to_string());
            let key2 = ApiKeyValue::from_string("same-key".to_string());
            let key3 = ApiKeyValue::from_string("different-key".to_string());

            assert_eq!(key1, key2, "Equal keys should be equal");
            assert_ne!(key1, key3, "Different keys should not be equal");
        }

        #[test]
        fn test_api_key_value_clone() {
            let key1 = ApiKeyValue::from_string("clone-test".to_string());
            let key2 = key1.clone();

            assert_eq!(key1, key2);
        }

        #[test]
        fn test_api_key_value_serialization() {
            let test_key = "serialization-test-key";
            let key = ApiKeyValue::from_string(test_key.to_string());

            // Test JSON serialization
            let json = serde_json::to_string(&key).unwrap();
            assert_eq!(json, format!("\"{}\"", test_key));

            // Test JSON deserialization
            let deserialized: ApiKeyValue = serde_json::from_str(&json).unwrap();
            assert_eq!(key, deserialized);
        }
    }

    // Tests for ApiKeyPermissions
    mod api_key_permissions_tests {
        use super::*;

        #[test]
        fn test_api_key_permissions_default() {
            let perms = ApiKeyPermissions::default();

            assert!(perms.read, "Default permissions should allow read");
            assert!(perms.write, "Default permissions should allow write");
            assert!(perms.delete, "Default permissions should allow delete");
            assert!(!perms.admin, "Default permissions should not allow admin");
        }

        #[test]
        fn test_api_key_permissions_full_access() {
            let perms = ApiKeyPermissions::full_access();

            assert!(perms.read, "Full access should allow read");
            assert!(perms.write, "Full access should allow write");
            assert!(perms.delete, "Full access should allow delete");
            assert!(perms.admin, "Full access should allow admin");
        }

        #[test]
        fn test_api_key_permissions_read_only() {
            let perms = ApiKeyPermissions::read_only();

            assert!(perms.read, "Read-only should allow read");
            assert!(!perms.write, "Read-only should not allow write");
            assert!(!perms.delete, "Read-only should not allow delete");
            assert!(!perms.admin, "Read-only should not allow admin");
        }

        #[test]
        fn test_api_key_permissions_equality() {
            let perms1 = ApiKeyPermissions {
                read: true,
                write: false,
                delete: true,
                admin: false,
            };

            let perms2 = ApiKeyPermissions {
                read: true,
                write: false,
                delete: true,
                admin: false,
            };

            let perms3 = ApiKeyPermissions {
                read: false,
                write: true,
                delete: true,
                admin: false,
            };

            assert_eq!(perms1, perms2, "Equal permissions should be equal");
            assert_ne!(perms1, perms3, "Different permissions should not be equal");
        }

        #[test]
        fn test_api_key_permissions_clone() {
            let perms1 = ApiKeyPermissions::full_access();
            let perms2 = perms1.clone();

            assert_eq!(perms1, perms2);
        }

        #[test]
        fn test_api_key_permissions_serialization() {
            let perms = ApiKeyPermissions {
                read: true,
                write: false,
                delete: true,
                admin: false,
            };

            // Test JSON serialization
            let json = serde_json::to_string(&perms).unwrap();
            let expected = r#"{"read":true,"write":false,"delete":true,"admin":false}"#;
            assert_eq!(json, expected);

            // Test JSON deserialization
            let deserialized: ApiKeyPermissions = serde_json::from_str(&json).unwrap();
            assert_eq!(perms, deserialized);
        }
    }
}
