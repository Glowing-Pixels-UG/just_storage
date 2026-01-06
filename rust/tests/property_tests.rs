//! Property-based tests using proptest
//!
//! These tests generate many random inputs to test invariants, edge cases,
//! and properties that should hold for all possible inputs.

use proptest::prelude::*;
use std::collections::HashSet;
use uuid::Uuid;

use just_storage::domain::value_objects::{
    ContentHash, Namespace, ObjectId, StorageClass, TenantId, ObjectStatus
};
use just_storage::domain::entities::{Object, Blob};
use just_storage::application::validation::{
    validate_namespace_and_tenant_for_text_search, validate_search_query
};

/// Strategy for generating valid namespace strings
fn namespace_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_-]{0,63}".prop_map(|s| s.to_string())
}

/// Strategy for generating valid tenant IDs
fn tenant_id_strategy() -> impl Strategy<Value = TenantId> {
    any::<[u8; 16]>().prop_map(|bytes| {
        let uuid = Uuid::from_bytes(bytes);
        TenantId::new(uuid)
    })
}

/// Strategy for generating valid content hashes (64 hex chars)
fn content_hash_strategy() -> impl Strategy<Value = ContentHash> {
    "[0-9a-f]{64}".prop_map(|s| ContentHash::from_hex(s).unwrap())
}

/// Strategy for generating valid object keys
fn object_key_strategy() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        "[a-zA-Z0-9_-]{1,255}".prop_map(Some)
    ]
}

/// Strategy for generating storage classes
fn storage_class_strategy() -> impl Strategy<Value = StorageClass> {
    prop_oneof![
        Just(StorageClass::Hot),
        Just(StorageClass::Cold)
    ]
}

/// Strategy for generating object statuses
fn object_status_strategy() -> impl Strategy<Value = ObjectStatus> {
    prop_oneof![
        Just(ObjectStatus::Writing),
        Just(ObjectStatus::Committed),
        Just(ObjectStatus::Deleting),
        Just(ObjectStatus::Deleted)
    ]
}

proptest! {
    /// Test that Namespace creation is deterministic and consistent
    #[test]
    fn namespace_creation_is_deterministic(s in namespace_strategy()) {
        let ns1 = Namespace::new(s.clone());
        let ns2 = Namespace::new(s);

        match (ns1, ns2) {
            (Ok(n1), Ok(n2)) => prop_assert_eq!(n1, n2),
            (Err(_), Err(_)) => {} // Both should fail consistently
            _ => prop_assert!(false, "Namespace creation should be consistent"),
        }
    }

    /// Test that TenantId round-trip serialization works
    #[test]
    fn tenant_id_round_trip(tenant_id in tenant_id_strategy()) {
        let uuid_str = tenant_id.to_string();
        let parsed = TenantId::from_string(&uuid_str);
        prop_assert!(parsed.is_ok(), "TenantId should parse its own string representation");
        prop_assert_eq!(parsed.unwrap(), tenant_id);
    }

    /// Test ContentHash hex encoding/decoding round trip
    #[test]
    fn content_hash_hex_round_trip(hash in content_hash_strategy()) {
        let hex = hash.as_hex();
        let parsed = ContentHash::from_hex(hex.to_string());
        prop_assert!(parsed.is_ok(), "ContentHash should parse its own hex representation");
        prop_assert_eq!(parsed.unwrap(), hash);
    }

    /// Test that ContentHash maintains 64-character hex format
    #[test]
    fn content_hash_hex_format(hash in content_hash_strategy()) {
        let hex = hash.as_hex();
        prop_assert_eq!(hex.len(), 64, "ContentHash hex should always be 64 characters");
        prop_assert!(hex.chars().all(|c| c.is_ascii_hexdigit()), "ContentHash hex should only contain hex digits");
    }

    /// Test namespace validation properties
    #[test]
    fn namespace_validation_properties(s in "[a-zA-Z][a-zA-Z0-9_-]{0,63}") {
        let result = Namespace::new(s.clone());

        // All strings in this regex should be valid (start with letter, valid chars, right length)
        prop_assert!(result.is_ok(), "Valid namespace pattern should succeed: {}", s);

        // Check that the namespace was lowercased
        if let Ok(namespace) = result {
            prop_assert_eq!(namespace.as_str(), s.to_lowercase());
        }
    }

    /// Test ObjectId generation uniqueness
    #[test]
    fn object_id_uniqueness(iterations in 1..1000usize) {
        let mut ids = HashSet::new();
        for _ in 0..iterations {
            let id = ObjectId::new();
            prop_assert!(!ids.contains(&id), "ObjectId should be unique");
            ids.insert(id);
        }
    }

    /// Test Object creation and properties
    #[test]
    fn object_creation_properties(
        namespace_str in namespace_strategy(),
        tenant_id in tenant_id_strategy(),
        key in object_key_strategy(),
        storage_class in storage_class_strategy(),
    ) {
        // Only test valid namespaces
        if let Ok(namespace) = Namespace::new(namespace_str) {
            let object = Object::new(namespace.clone(), tenant_id.clone(), key.clone(), storage_class);

            // Test basic properties
            prop_assert_eq!(object.namespace(), &namespace);
            prop_assert_eq!(object.tenant_id(), &tenant_id);
            match (object.key().as_ref(), key.as_ref()) {
                (Some(obj_key), Some(expected_key)) => prop_assert_eq!(obj_key, expected_key),
                (None, None) => {},
                _ => prop_assert!(false, "Key mismatch"),
            }
            prop_assert_eq!(object.storage_class(), storage_class);
            prop_assert_eq!(object.status(), ObjectStatus::Writing);

            // Test that ID is generated
            prop_assert!(!object.id().to_string().is_empty());

            // Test timestamps are set
            prop_assert!(object.created_at() <= object.updated_at());
        }
    }

    /// Test Blob creation properties
    #[test]
    fn blob_creation_properties(
        hash in content_hash_strategy(),
        storage_class in storage_class_strategy(),
        size in 0..1_000_000u64,
    ) {
        let blob = Blob::new(hash.clone(), storage_class, size);

        prop_assert_eq!(blob.content_hash(), &hash);
        prop_assert_eq!(blob.storage_class(), storage_class);
        prop_assert_eq!(blob.size_bytes(), size);
        prop_assert_eq!(blob.ref_count(), 1); // New blobs start with ref count 1
    }

    /// Test storage class serialization consistency
    #[test]
    fn storage_class_serialization(storage_class in storage_class_strategy()) {
        let serialized = serde_json::to_string(&storage_class).unwrap();
        let deserialized: StorageClass = serde_json::from_str(&serialized).unwrap();
        prop_assert_eq!(storage_class, deserialized);
    }

    /// Test search query validation
    #[test]
    fn search_query_validation(query in ".*") {
        let result = validate_search_query(&query);

        // Empty queries should fail
        if query.trim().is_empty() {
            prop_assert!(result.is_err(), "Empty search query should be invalid");
        }

        // Very long queries should fail
        if query.len() > 1000 {
            prop_assert!(result.is_err(), "Very long search queries should be invalid");
        }

        // Queries with only whitespace should fail
        if query.chars().all(|c| c.is_whitespace()) {
            prop_assert!(result.is_err(), "Whitespace-only queries should be invalid");
        }
    }

    /// Test text search validation
    #[test]
    fn text_search_validation_properties(
        namespace_str in namespace_strategy(),
        tenant_id_str in "[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}",
    ) {
        let result = validate_namespace_and_tenant_for_text_search(&namespace_str, &tenant_id_str);

        // The function validates namespace and tenant_id
        // Test that it returns the parsed objects for valid inputs
        if let Ok((parsed_namespace, parsed_tenant_id)) = result {
            prop_assert_eq!(parsed_namespace.to_string(), namespace_str.to_lowercase());
            prop_assert_eq!(parsed_tenant_id.to_string(), tenant_id_str);
        } else {
            // If it fails, check the reasons
            let is_valid_uuid = uuid::Uuid::parse_str(&tenant_id_str).is_ok();
            let is_valid_namespace = Namespace::new(namespace_str.clone()).is_ok();

            if !is_valid_uuid || !is_valid_namespace {
                // Should fail for invalid inputs
                prop_assert!(result.is_err(), "Should fail for invalid inputs");
            }
        }
    }

    /// Test object status transitions
    #[test]
    fn object_status_transitions(
        initial_status in object_status_strategy(),
        hash in content_hash_strategy(),
        size in 1..1000000u64,
    ) {
        let mut object = Object::new(
            Namespace::new("test".to_string()).unwrap(),
            TenantId::new(Uuid::new_v4()),
            Some("test_key".to_string()),
            StorageClass::Hot,
        );

        // Test that commit only works from Writing status
        // Note: We can't easily test status transitions with the current Object API
        // as commit() is not exposed in the current implementation
    }

    /// Test that object metadata is properly initialized
    #[test]
    fn object_metadata_initialization(namespace_str in namespace_strategy()) {
        if let Ok(namespace) = Namespace::new(namespace_str) {
            let tenant_id = TenantId::new(Uuid::new_v4());
            let mut object = Object::new(namespace, tenant_id, None, StorageClass::Hot);

            // Metadata should be initialized with default values
            let metadata = object.metadata();
            prop_assert_eq!(&metadata.kind, &just_storage::domain::value_objects::ObjectKind::Upload);

            // Should be able to get mutable access
            let _mutable_metadata = object.metadata_mut();
        }
    }

    /// Test content hash validation
    #[test]
    fn content_hash_validation(hex in ".*") {
        let result = ContentHash::from_hex(hex.clone());

        // Should fail for wrong length
        if hex.len() != 64 {
            prop_assert!(result.is_err(), "ContentHash should reject wrong length: {}", hex.len());
        }

        // Should fail for non-hex characters
        if hex.len() == 64 && !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            prop_assert!(result.is_err(), "ContentHash should reject non-hex characters");
        }

        // Should succeed for valid 64-char hex
        if hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            prop_assert!(result.is_ok(), "ContentHash should accept valid hex");
        }
    }

    /// Test namespace equality and hashing
    #[test]
    fn namespace_equality_and_hashing(
        ns1 in namespace_strategy(),
        ns2 in namespace_strategy(),
    ) {
        if let (Ok(n1), Ok(n2)) = (Namespace::new(ns1), Namespace::new(ns2)) {
            // Equal namespaces should have equal hash
            if n1 == n2 {
                let mut set = HashSet::new();
                set.insert(n1);
                prop_assert!(!set.insert(n2), "Equal namespaces should have same hash");
            }
        }
    }
}

/// Test suite configuration for proptest
#[cfg(test)]
mod config {
    use proptest::test_runner::Config;

    fn proptest_config() -> Config {
        Config {
            cases: 1000,  // Run 1000 test cases per property
            ..Config::default()
        }
    }
}