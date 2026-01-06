use std::collections::HashSet;

/// Permission constants for the JustStorage system
pub mod permissions {
    // Object operations
    pub const OBJECTS_READ: &str = "objects:read";
    pub const OBJECTS_WRITE: &str = "objects:write";
    pub const OBJECTS_DELETE: &str = "objects:delete";

    // API key management
    pub const API_KEYS_READ: &str = "api_keys:read";
    pub const API_KEYS_WRITE: &str = "api_keys:write";
    pub const API_KEYS_DELETE: &str = "api_keys:delete";

    // Administrative operations
    pub const ADMIN: &str = "admin";
    pub const TENANT_ADMIN: &str = "tenant_admin";

    // System operations
    pub const HEALTH_READ: &str = "health:read";

    // All available permissions
    pub const ALL: &[&str] = &[
        OBJECTS_READ,
        OBJECTS_WRITE,
        OBJECTS_DELETE,
        API_KEYS_READ,
        API_KEYS_WRITE,
        API_KEYS_DELETE,
        ADMIN,
        TENANT_ADMIN,
        HEALTH_READ,
    ];
}

/// Role definitions with their associated permissions
pub mod roles {
    use super::permissions::*;

    pub const ADMIN: &[&str] = &[
        super::permissions::ADMIN,
        super::permissions::TENANT_ADMIN,
        OBJECTS_READ,
        OBJECTS_WRITE,
        OBJECTS_DELETE,
        API_KEYS_READ,
        API_KEYS_WRITE,
        API_KEYS_DELETE,
        HEALTH_READ,
    ];
    pub const TENANT_ADMIN: &[&str] = &[
        super::permissions::TENANT_ADMIN,
        OBJECTS_READ,
        OBJECTS_WRITE,
        OBJECTS_DELETE,
        API_KEYS_READ,
        API_KEYS_WRITE,
        API_KEYS_DELETE,
        HEALTH_READ,
    ];
    pub const USER: &[&str] = &[OBJECTS_READ, OBJECTS_WRITE, API_KEYS_READ, HEALTH_READ];
    pub const READ_ONLY: &[&str] = &[OBJECTS_READ, HEALTH_READ];
    pub const API_CLIENT: &[&str] = &[OBJECTS_READ, OBJECTS_WRITE, HEALTH_READ];

    /// Get permissions for a role
    pub fn get_permissions_for_role(role: &str) -> Vec<&'static str> {
        match role {
            "admin" => ADMIN.to_vec(),
            "tenant_admin" => TENANT_ADMIN.to_vec(),
            "user" => USER.to_vec(),
            "read_only" => READ_ONLY.to_vec(),
            "api_client" => API_CLIENT.to_vec(),
            _ => vec![], // Unknown role gets no permissions
        }
    }

    /// Check if a role is valid
    pub fn is_valid_role(role: &str) -> bool {
        matches!(
            role,
            "admin" | "tenant_admin" | "user" | "read_only" | "api_client"
        )
    }
}

/// User context extracted from authentication
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
    pub permissions: HashSet<String>,
    pub is_api_key: bool,
    pub api_key_id: Option<String>,
}

impl UserContext {
    /// Create a new user context
    pub fn new(
        user_id: String,
        tenant_id: String,
        roles: Vec<String>,
        permissions: HashSet<String>,
        is_api_key: bool,
        api_key_id: Option<String>,
    ) -> Self {
        Self {
            user_id,
            tenant_id,
            roles,
            permissions,
            is_api_key,
            api_key_id,
        }
    }

    /// Create context for API key authentication
    pub fn from_api_key(
        api_key_id: String,
        tenant_id: String,
        permissions: HashSet<String>,
    ) -> Self {
        Self {
            user_id: format!("api_key:{}", api_key_id),
            tenant_id,
            roles: vec!["api_client".to_string()],
            permissions,
            is_api_key: true,
            api_key_id: Some(api_key_id),
        }
    }

    /// Check if user has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(permission)
    }

    /// Check if user has all specified permissions
    pub fn has_permissions(&self, required_permissions: &[&str]) -> bool {
        required_permissions.iter().all(|p| self.has_permission(p))
    }

    /// Check if user has any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[&str]) -> bool {
        permissions.iter().any(|p| self.has_permission(p))
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }

    /// Check if user is an admin (global or tenant)
    pub fn is_admin(&self) -> bool {
        self.has_role("admin")
            || self.has_role("tenant_admin")
            || self.has_permission(permissions::ADMIN)
    }

    /// Check if user is a tenant admin
    pub fn is_tenant_admin(&self) -> bool {
        self.has_role("tenant_admin") || self.has_permission(permissions::TENANT_ADMIN)
    }

    /// Check if user can read objects
    pub fn can_read_objects(&self) -> bool {
        self.has_permission(permissions::OBJECTS_READ)
    }

    /// Check if user can write objects
    pub fn can_write_objects(&self) -> bool {
        self.has_permission(permissions::OBJECTS_WRITE)
    }

    /// Check if user can delete objects
    pub fn can_delete_objects(&self) -> bool {
        self.has_permission(permissions::OBJECTS_DELETE)
    }

    /// Check if user can manage API keys
    pub fn can_manage_api_keys(&self) -> bool {
        self.has_permission(permissions::API_KEYS_READ)
            || self.has_permission(permissions::API_KEYS_WRITE)
            || self.has_permission(permissions::API_KEYS_DELETE)
    }
}

/// Authorization result
#[derive(Debug)]
pub enum AuthorizationResult {
    Allowed,
    Forbidden(String),    // Reason for denial
    Unauthorized(String), // Authentication required
}

impl AuthorizationResult {
    pub fn allowed() -> Self {
        Self::Allowed
    }

    pub fn forbidden(reason: impl Into<String>) -> Self {
        Self::Forbidden(reason.into())
    }

    pub fn unauthorized(reason: impl Into<String>) -> Self {
        Self::Unauthorized(reason.into())
    }

    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

/// Authorization error types
#[derive(Debug, thiserror::Error)]
pub enum AuthorizationError {
    #[error("Authentication required: {0}")]
    AuthenticationRequired(String),

    #[error("Access forbidden: {0}")]
    AccessForbidden(String),

    #[error("Invalid user context")]
    InvalidUserContext,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_context_permissions() {
        let mut permissions = HashSet::new();
        permissions.insert(permissions::OBJECTS_READ.to_string());
        permissions.insert(permissions::OBJECTS_WRITE.to_string());

        let context = UserContext::new(
            "user123".to_string(),
            "tenant456".to_string(),
            vec!["user".to_string()],
            permissions,
            false,
            None,
        );

        assert!(context.has_permission(permissions::OBJECTS_READ));
        assert!(context.has_permission(permissions::OBJECTS_WRITE));
        assert!(!context.has_permission(permissions::ADMIN));
        assert!(context.can_read_objects());
        assert!(context.can_write_objects());
        assert!(!context.can_delete_objects());
    }

    #[test]
    fn test_role_permissions() {
        let admin_perms = roles::get_permissions_for_role("admin");
        assert!(admin_perms.contains(&permissions::ADMIN));
        assert!(admin_perms.contains(&permissions::OBJECTS_READ));

        let read_only_perms = roles::get_permissions_for_role("read_only");
        assert!(read_only_perms.contains(&permissions::OBJECTS_READ));
        assert!(!read_only_perms.contains(&permissions::OBJECTS_WRITE));
    }

    #[test]
    fn test_api_key_context() {
        let mut permissions = HashSet::new();
        permissions.insert(permissions::OBJECTS_READ.to_string());

        let context =
            UserContext::from_api_key("key123".to_string(), "tenant456".to_string(), permissions);

        assert!(context.is_api_key);
        assert_eq!(context.api_key_id, Some("key123".to_string()));
        assert!(context.has_role("api_client"));
        assert!(context.can_read_objects());
        assert!(!context.can_write_objects());
    }

    #[test]
    fn test_admin_permissions() {
        let admin_perms = roles::get_permissions_for_role("admin");
        assert!(admin_perms.contains(&permissions::ADMIN));
        assert!(admin_perms.contains(&permissions::TENANT_ADMIN));
        assert!(admin_perms.contains(&permissions::OBJECTS_READ));
        assert!(admin_perms.contains(&permissions::OBJECTS_WRITE));
        assert!(admin_perms.contains(&permissions::OBJECTS_DELETE));
        assert!(admin_perms.contains(&permissions::API_KEYS_READ));
        assert!(admin_perms.contains(&permissions::API_KEYS_WRITE));
        assert!(admin_perms.contains(&permissions::API_KEYS_DELETE));
        assert!(admin_perms.contains(&permissions::HEALTH_READ));
    }

    #[test]
    fn test_tenant_admin_permissions() {
        let tenant_admin_perms = roles::get_permissions_for_role("tenant_admin");
        assert!(tenant_admin_perms.contains(&permissions::TENANT_ADMIN));
        assert!(tenant_admin_perms.contains(&permissions::OBJECTS_READ));
        assert!(tenant_admin_perms.contains(&permissions::OBJECTS_WRITE));
        assert!(tenant_admin_perms.contains(&permissions::OBJECTS_DELETE));
        assert!(tenant_admin_perms.contains(&permissions::API_KEYS_READ));
        assert!(tenant_admin_perms.contains(&permissions::API_KEYS_WRITE));
        assert!(tenant_admin_perms.contains(&permissions::API_KEYS_DELETE));
        assert!(tenant_admin_perms.contains(&permissions::HEALTH_READ));
        // Should not have global admin
        assert!(!tenant_admin_perms.contains(&permissions::ADMIN));
    }

    #[test]
    fn test_user_permissions() {
        let user_perms = roles::get_permissions_for_role("user");
        assert!(user_perms.contains(&permissions::OBJECTS_READ));
        assert!(user_perms.contains(&permissions::OBJECTS_WRITE));
        assert!(user_perms.contains(&permissions::API_KEYS_READ));
        assert!(user_perms.contains(&permissions::HEALTH_READ));
        // Should not have delete permissions
        assert!(!user_perms.contains(&permissions::OBJECTS_DELETE));
        assert!(!user_perms.contains(&permissions::API_KEYS_WRITE));
        assert!(!user_perms.contains(&permissions::ADMIN));
    }

    #[test]
    fn test_read_only_permissions() {
        let read_only_perms = roles::get_permissions_for_role("read_only");
        assert!(read_only_perms.contains(&permissions::OBJECTS_READ));
        assert!(read_only_perms.contains(&permissions::HEALTH_READ));
        // Should not have write permissions
        assert!(!read_only_perms.contains(&permissions::OBJECTS_WRITE));
        assert!(!read_only_perms.contains(&permissions::OBJECTS_DELETE));
        assert!(!read_only_perms.contains(&permissions::API_KEYS_READ));
    }

    #[test]
    fn test_api_client_permissions() {
        let api_client_perms = roles::get_permissions_for_role("api_client");
        assert!(api_client_perms.contains(&permissions::OBJECTS_READ));
        assert!(api_client_perms.contains(&permissions::OBJECTS_WRITE));
        assert!(api_client_perms.contains(&permissions::HEALTH_READ));
        // Should not have delete or admin permissions
        assert!(!api_client_perms.contains(&permissions::OBJECTS_DELETE));
        assert!(!api_client_perms.contains(&permissions::ADMIN));
    }

    #[test]
    fn test_unknown_role_permissions() {
        let unknown_perms = roles::get_permissions_for_role("unknown_role");
        assert!(unknown_perms.is_empty());
    }

    #[test]
    fn test_role_validation() {
        assert!(roles::is_valid_role("admin"));
        assert!(roles::is_valid_role("tenant_admin"));
        assert!(roles::is_valid_role("user"));
        assert!(roles::is_valid_role("read_only"));
        assert!(roles::is_valid_role("api_client"));
        assert!(!roles::is_valid_role("unknown"));
        assert!(!roles::is_valid_role(""));
    }

    #[test]
    fn test_user_context_admin_checks() {
        // Test global admin
        let mut permissions = HashSet::new();
        permissions.insert(permissions::ADMIN.to_string());
        let admin_context = UserContext::new(
            "admin".to_string(),
            "tenant1".to_string(),
            vec!["admin".to_string()],
            permissions,
            false,
            None,
        );
        assert!(admin_context.is_admin());
        assert!(admin_context.is_tenant_admin());

        // Test tenant admin
        let mut permissions = HashSet::new();
        permissions.insert(permissions::TENANT_ADMIN.to_string());
        let tenant_admin_context = UserContext::new(
            "tenant_admin".to_string(),
            "tenant1".to_string(),
            vec!["tenant_admin".to_string()],
            permissions,
            false,
            None,
        );
        assert!(tenant_admin_context.is_admin());
        assert!(tenant_admin_context.is_tenant_admin());

        // Test regular user
        let permissions = HashSet::new();
        let user_context = UserContext::new(
            "user".to_string(),
            "tenant1".to_string(),
            vec!["user".to_string()],
            permissions,
            false,
            None,
        );
        assert!(!user_context.is_admin());
        assert!(!user_context.is_tenant_admin());
    }

    #[test]
    fn test_user_context_capability_checks() {
        let mut permissions = HashSet::new();
        permissions.insert(permissions::OBJECTS_READ.to_string());
        permissions.insert(permissions::OBJECTS_WRITE.to_string());

        let context = UserContext::new(
            "user123".to_string(),
            "tenant456".to_string(),
            vec!["user".to_string()],
            permissions,
            false,
            None,
        );

        assert!(context.can_read_objects());
        assert!(context.can_write_objects());
        assert!(!context.can_delete_objects());
        assert!(!context.can_manage_api_keys());
    }

    #[test]
    fn test_permission_constants() {
        // Test that all permission constants are defined
        assert_eq!(permissions::OBJECTS_READ, "objects:read");
        assert_eq!(permissions::OBJECTS_WRITE, "objects:write");
        assert_eq!(permissions::OBJECTS_DELETE, "objects:delete");
        assert_eq!(permissions::API_KEYS_READ, "api_keys:read");
        assert_eq!(permissions::API_KEYS_WRITE, "api_keys:write");
        assert_eq!(permissions::API_KEYS_DELETE, "api_keys:delete");
        assert_eq!(permissions::ADMIN, "admin");
        assert_eq!(permissions::TENANT_ADMIN, "tenant_admin");
        assert_eq!(permissions::HEALTH_READ, "health:read");

        // Test that ALL contains all permissions
        assert!(permissions::ALL.contains(&permissions::OBJECTS_READ));
        assert!(permissions::ALL.contains(&permissions::ADMIN));
        assert_eq!(permissions::ALL.len(), 9); // Should have 9 permissions
    }

    #[test]
    fn test_authorization_result() {
        let allowed = AuthorizationResult::allowed();
        assert!(allowed.is_allowed());

        let forbidden = AuthorizationResult::forbidden("Access denied");
        assert!(!forbidden.is_allowed());

        let unauthorized = AuthorizationResult::unauthorized("Not authenticated");
        assert!(!unauthorized.is_allowed());
    }
}
