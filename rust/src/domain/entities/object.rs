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
