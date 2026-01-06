use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{
    errors::DomainError,
    value_objects::{
        ContentHash, Namespace, ObjectId, ObjectMetadata, ObjectStatus, StorageClass, TenantId,
    },
};

/// Object aggregate root - represents a stored object with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    id: ObjectId,
    namespace: Namespace,
    tenant_id: TenantId,
    key: Option<String>,
    status: ObjectStatus,
    storage_class: StorageClass,
    content_hash: Option<ContentHash>,
    size_bytes: Option<u64>,
    content_type: Option<String>,
    metadata: ObjectMetadata,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Object {
    /// Create new object in WRITING state
    pub fn new(
        namespace: Namespace,
        tenant_id: TenantId,
        key: Option<String>,
        storage_class: StorageClass,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: ObjectId::new(),
            namespace,
            tenant_id,
            key,
            status: ObjectStatus::Writing,
            storage_class,
            content_hash: None,
            size_bytes: None,
            content_type: None,
            metadata: ObjectMetadata::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Reconstruct from storage (e.g., database)
    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: ObjectId,
        namespace: Namespace,
        tenant_id: TenantId,
        key: Option<String>,
        status: ObjectStatus,
        storage_class: StorageClass,
        content_hash: Option<ContentHash>,
        size_bytes: Option<u64>,
        content_type: Option<String>,
        metadata: ObjectMetadata,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            namespace,
            tenant_id,
            key,
            status,
            storage_class,
            content_hash,
            size_bytes,
            content_type,
            metadata,
            created_at,
            updated_at,
        }
    }

    /// Commit object after successful upload
    pub fn commit(
        &mut self,
        content_hash: ContentHash,
        size_bytes: u64,
    ) -> Result<(), DomainError> {
        if self.status != ObjectStatus::Writing {
            return Err(DomainError::InvalidStateTransition {
                from: self.status,
                to: ObjectStatus::Committed,
            });
        }

        self.status = ObjectStatus::Committed;
        self.content_hash = Some(content_hash);
        self.size_bytes = Some(size_bytes);
        self.updated_at = Utc::now();

        Ok(())
    }

    /// Mark object for deletion
    pub fn mark_for_deletion(&mut self) -> Result<(), DomainError> {
        if self.status != ObjectStatus::Committed {
            return Err(DomainError::CannotDeleteNonCommitted);
        }

        self.status = ObjectStatus::Deleting;
        self.updated_at = Utc::now();

        Ok(())
    }

    /// Mark as fully deleted (tombstone)
    pub fn mark_deleted(&mut self) -> Result<(), DomainError> {
        if self.status != ObjectStatus::Deleting {
            return Err(DomainError::InvalidStateTransition {
                from: self.status,
                to: ObjectStatus::Deleted,
            });
        }

        self.status = ObjectStatus::Deleted;
        self.updated_at = Utc::now();

        Ok(())
    }

    // Getters
    pub fn id(&self) -> &ObjectId {
        &self.id
    }

    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    pub fn status(&self) -> ObjectStatus {
        self.status
    }

    pub fn storage_class(&self) -> StorageClass {
        self.storage_class
    }

    pub fn content_hash(&self) -> Option<&ContentHash> {
        self.content_hash.as_ref()
    }

    pub fn size_bytes(&self) -> Option<u64> {
        self.size_bytes
    }

    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    pub fn metadata(&self) -> &ObjectMetadata {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut ObjectMetadata {
        &mut self.metadata
    }

    pub fn set_content_type(&mut self, content_type: String) {
        self.content_type = Some(content_type);
        self.updated_at = Utc::now();
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Check if object is in terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, ObjectStatus::Deleted)
    }

    /// Check if object can be read
    pub fn is_readable(&self) -> bool {
        self.status == ObjectStatus::Committed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::{Namespace, TenantId, ObjectKind};
    use std::str::FromStr;
    use uuid::Uuid;

    fn create_test_object() -> Object {
        let tenant_id = TenantId::new(Uuid::new_v4());
        let namespace = Namespace::from_str("test-namespace").unwrap();
        Object::new(
            namespace,
            tenant_id,
            Some("test-key".to_string()),
            StorageClass::Hot,
        )
    }

    #[test]
    fn test_object_new() {
        let object = create_test_object();
        assert_eq!(object.status(), ObjectStatus::Writing);
        assert!(object.content_hash().is_none());
        assert!(object.size_bytes().is_none());
    }

    #[test]
    fn test_object_commit_valid() {
        let mut object = create_test_object();
        let content_hash = ContentHash::from_str(&"a".repeat(64)).unwrap();
        let size_bytes = 123;

        object.commit(content_hash.clone(), size_bytes).unwrap();

        assert_eq!(object.status(), ObjectStatus::Committed);
        assert_eq!(object.content_hash(), Some(&content_hash));
        assert_eq!(object.size_bytes(), Some(size_bytes));
    }

    #[test]
    fn test_object_commit_invalid_state() {
        let mut object = create_test_object();
        let content_hash = ContentHash::from_str(&"a".repeat(64)).unwrap();
        object.commit(content_hash.clone(), 123).unwrap();

        let err = object.commit(content_hash, 123).unwrap_err();
        assert!(matches!(err, DomainError::InvalidStateTransition { .. }));
    }

    #[test]
    fn test_object_mark_for_deletion_valid() {
        let mut object = create_test_object();
        let content_hash = ContentHash::from_str(&"a".repeat(64)).unwrap();
        object.commit(content_hash, 123).unwrap();

        object.mark_for_deletion().unwrap();
        assert_eq!(object.status(), ObjectStatus::Deleting);
    }

    #[test]
    fn test_object_mark_for_deletion_invalid_state() {
        let mut object = create_test_object();
        let err = object.mark_for_deletion().unwrap_err();
        assert!(matches!(err, DomainError::CannotDeleteNonCommitted));
    }

    #[test]
    fn test_object_mark_deleted_valid() {
        let mut object = create_test_object();
        let content_hash = ContentHash::from_str(&"a".repeat(64)).unwrap();
        object.commit(content_hash, 123).unwrap();
        object.mark_for_deletion().unwrap();

        object.mark_deleted().unwrap();
        assert_eq!(object.status(), ObjectStatus::Deleted);
    }

    #[test]
    fn test_object_mark_deleted_invalid_state() {
        let mut object = create_test_object();
        let err = object.mark_deleted().unwrap_err();
        assert!(matches!(err, DomainError::InvalidStateTransition { .. }));
    }

    #[test]
    fn test_object_is_readable() {
        let mut object = create_test_object();
        assert!(!object.is_readable());

        let content_hash = ContentHash::from_str(&"a".repeat(64)).unwrap();
        object.commit(content_hash, 123).unwrap();
        assert!(object.is_readable());

        object.mark_for_deletion().unwrap();
        assert!(!object.is_readable());
    }

    #[test]
    fn test_object_is_terminal() {
        let mut object = create_test_object();
        assert!(!object.is_terminal());

        let content_hash = ContentHash::from_str(&"a".repeat(64)).unwrap();
        object.commit(content_hash, 123).unwrap();
        assert!(!object.is_terminal());

        object.mark_for_deletion().unwrap();
        assert!(!object.is_terminal());

        object.mark_deleted().unwrap();
        assert!(object.is_terminal());
    }

    #[test]
    fn test_set_content_type() {
        let mut object = create_test_object();
        let original_updated_at = object.updated_at();

        // Manually set the updated_at to a known value in the past
        object.updated_at = original_updated_at - chrono::Duration::seconds(1);
        let past_updated_at = object.updated_at();

        let content_type = "application/json".to_string();
        object.set_content_type(content_type.clone());

        assert_eq!(object.content_type(), Some(content_type.as_str()));
        assert!(object.updated_at() > past_updated_at);
    }

    #[test]
    fn test_object_creation_edge_cases() {
        let tenant_id = TenantId::new(Uuid::new_v4());
        let namespace = Namespace::from_str("test").unwrap();

        // Test with None key
        let object_no_key = Object::new(
            namespace.clone(),
            tenant_id.clone(),
            None,
            StorageClass::Hot,
        );
        assert!(object_no_key.key().is_none());

        // Test with empty string key (should be allowed)
        let object_empty_key = Object::new(
            namespace.clone(),
            tenant_id.clone(),
            Some("".to_string()),
            StorageClass::Cold,
        );
        assert_eq!(object_empty_key.key(), Some(""));

        // Test with long key
        let long_key = "a".repeat(1000);
        let object_long_key = Object::new(
            namespace.clone(),
            tenant_id.clone(),
            Some(long_key.clone()),
            StorageClass::Hot,
        );
        assert_eq!(object_long_key.key(), Some(&long_key));
    }

    #[test]
    fn test_object_metadata_operations() {
        let mut object = create_test_object();

        // Test setting content type multiple times
        object.set_content_type("text/plain".to_string());
        assert_eq!(object.content_type(), Some("text/plain"));

        object.set_content_type("application/json".to_string());
        assert_eq!(object.content_type(), Some("application/json"));

        // Test metadata access
        let metadata = object.metadata();
        assert_eq!(metadata.kind, ObjectKind::Upload);

        // Test mutable metadata access
        let metadata_mut = object.metadata_mut();
        assert_eq!(metadata_mut.kind, ObjectKind::Upload);
    }

    #[test]
    fn test_object_status_transitions_comprehensive() {
        let mut object = create_test_object();

        // Initial state
        assert_eq!(object.status(), ObjectStatus::Writing);

        // Commit object
        let content_hash = ContentHash::from_str(&"a".repeat(64)).unwrap();
        object.commit(content_hash, 1024).unwrap();
        assert_eq!(object.status(), ObjectStatus::Committed);

        // Test that committed object can't be committed again
        let result = object.commit(content_hash, 2048);
        assert!(matches!(result, Err(DomainError::InvalidStateTransition { .. })));
    }

    #[test]
    fn test_object_timestamps() {
        let object = create_test_object();
        let created_at = object.created_at();
        let updated_at = object.updated_at();

        // Updated at should be >= created at
        assert!(updated_at >= created_at);

        // Timestamps should be recent (within last minute)
        let now = Utc::now();
        let one_minute_ago = now - chrono::Duration::minutes(1);
        assert!(created_at > one_minute_ago);
        assert!(updated_at > one_minute_ago);
    }

    #[test]
    fn test_object_equality_and_hashing() {
        let object1 = create_test_object();
        let object2 = create_test_object();

        // Different objects should not be equal
        assert_ne!(object1, object2);
        assert_ne!(object1.id(), object2.id());

        // Same ID should make objects equal for hashing purposes
        // (In practice, objects with same ID should be considered equal)
        assert_ne!(object1.id(), object2.id());
    }

    #[test]
    fn test_object_clone() {
        let object1 = create_test_object();
        let object2 = object1.clone();

        assert_eq!(object1, object2);
        assert_eq!(object1.id(), object2.id());
        assert_eq!(object1.status(), object2.status());
        assert_eq!(object1.namespace(), object2.namespace());
        assert_eq!(object1.tenant_id(), object2.tenant_id());
        assert_eq!(object1.key(), object2.key());
    }

    #[test]
    fn test_object_with_committed_state() {
        let mut object = create_test_object();
        let content_hash = ContentHash::from_str(&"b".repeat(64)).unwrap();
        let size = 2048u64;

        object.commit(content_hash.clone(), size).unwrap();

        // Verify all properties after commit
        assert_eq!(object.status(), ObjectStatus::Committed);
        assert_eq!(object.content_hash(), Some(&content_hash));
        assert_eq!(object.size_bytes(), Some(size));
        assert!(object.updated_at() >= object.created_at());
    }

    #[test]
    fn test_object_invalid_commit_parameters() {
        let mut object = create_test_object();

        // Test with zero size
        let content_hash = ContentHash::from_str(&"c".repeat(64)).unwrap();
        object.commit(content_hash, 0).unwrap(); // Zero size should be allowed

        // Test with very large size
        let mut object2 = create_test_object();
        let large_size = u64::MAX;
        object2.commit(content_hash, large_size).unwrap();
        assert_eq!(object2.size_bytes(), Some(large_size));
    }
}
