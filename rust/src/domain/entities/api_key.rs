use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::value_objects::{ApiKeyId, ApiKeyPermissions, ApiKeyValue};

/// Data structure for reconstructing API keys from database
#[derive(Debug, Clone)]
pub struct ApiKeyDbData {
    pub id: ApiKeyId,
    pub api_key: ApiKeyValue,
    pub tenant_id: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: ApiKeyPermissions,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// API key entity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiKey {
    id: ApiKeyId,
    api_key: ApiKeyValue,
    tenant_id: String,
    name: String,
    description: Option<String>,
    permissions: ApiKeyPermissions,
    is_active: bool,
    expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
}

impl ApiKey {
    pub fn new(
        tenant_id: String,
        name: String,
        description: Option<String>,
        permissions: ApiKeyPermissions,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: ApiKeyId::new(),
            api_key: ApiKeyValue::generate(),
            tenant_id,
            name,
            description,
            permissions,
            is_active: true,
            expires_at,
            created_at: now,
            updated_at: now,
            last_used_at: None,
        }
    }

    /// Reconstruct from database data (used by repository)
    pub fn from_db(db_data: ApiKeyDbData) -> Self {
        Self {
            id: db_data.id,
            api_key: db_data.api_key,
            tenant_id: db_data.tenant_id,
            name: db_data.name,
            description: db_data.description,
            permissions: db_data.permissions,
            is_active: db_data.is_active,
            expires_at: db_data.expires_at,
            created_at: db_data.created_at,
            updated_at: db_data.updated_at,
            last_used_at: db_data.last_used_at,
        }
    }

    // Getters
    pub fn id(&self) -> &ApiKeyId {
        &self.id
    }

    pub fn api_key(&self) -> &ApiKeyValue {
        &self.api_key
    }

    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn permissions(&self) -> &ApiKeyPermissions {
        &self.permissions
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn expires_at(&self) -> Option<&DateTime<Utc>> {
        self.expires_at.as_ref()
    }

    pub fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    pub fn updated_at(&self) -> &DateTime<Utc> {
        &self.updated_at
    }

    pub fn last_used_at(&self) -> Option<&DateTime<Utc>> {
        self.last_used_at.as_ref()
    }

    // Setters
    pub fn set_name(&mut self, name: String) {
        self.name = name;
        self.updated_at = Utc::now();
    }

    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.updated_at = Utc::now();
    }

    pub fn set_permissions(&mut self, permissions: ApiKeyPermissions) {
        self.permissions = permissions;
        self.updated_at = Utc::now();
    }

    pub fn set_active(&mut self, is_active: bool) {
        self.is_active = is_active;
        self.updated_at = Utc::now();
    }

    pub fn set_expires_at(&mut self, expires_at: Option<DateTime<Utc>>) {
        self.expires_at = expires_at;
        self.updated_at = Utc::now();
    }

    pub fn mark_used(&mut self) {
        self.last_used_at = Some(Utc::now());
    }

    // Business logic
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expires| Utc::now() > expires)
    }

    pub fn can_read(&self) -> bool {
        self.is_active && !self.is_expired() && self.permissions.read
    }

    pub fn can_write(&self) -> bool {
        self.is_active && !self.is_expired() && self.permissions.write
    }

    pub fn can_delete(&self) -> bool {
        self.is_active && !self.is_expired() && self.permissions.delete
    }

    pub fn is_admin(&self) -> bool {
        self.is_active && !self.is_expired() && self.permissions.admin
    }
}
