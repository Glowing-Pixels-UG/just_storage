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
    use crate::domain::value_objects::{Namespace, TenantId};
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
}
