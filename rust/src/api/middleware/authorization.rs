use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::collections::HashSet;

use crate::domain::authorization::{permissions, UserContext};

/// Authorization error response
#[derive(Serialize)]
struct AuthorizationErrorResponse {
    error: String,
    code: String,
    details: Option<String>,
}

/// Authorization middleware that checks if the user has required permissions
pub fn require_permissions(required_permissions: Vec<&'static str>) -> PermissionMiddleware {
    PermissionMiddleware {
        required_permissions: required_permissions.into_iter().collect(),
        require_all: true,
    }
}

/// Authorization middleware that checks if the user has any of the required permissions
pub fn require_any_permission(permissions: Vec<&'static str>) -> PermissionMiddleware {
    PermissionMiddleware {
        required_permissions: permissions.into_iter().collect(),
        require_all: false,
    }
}

/// Authorization middleware that checks if the user has a specific role
pub fn require_role(role: &'static str) -> RoleMiddleware {
    RoleMiddleware { role }
}

/// Authorization middleware that checks if the user owns the resource
pub fn require_resource_owner() -> ResourceOwnerMiddleware {
    ResourceOwnerMiddleware
}

/// Permission-based authorization middleware
pub struct PermissionMiddleware {
    required_permissions: HashSet<&'static str>,
    require_all: bool,
}

impl PermissionMiddleware {
    pub async fn layer(self, request: Request, next: Next) -> Response {
        // Extract user context from request extensions
        let user_context = match request.extensions().get::<UserContext>() {
            Some(context) => context,
            None => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(AuthorizationErrorResponse {
                        error: "Authentication required".to_string(),
                        code: "AUTHENTICATION_REQUIRED".to_string(),
                        details: Some("No user context found in request".to_string()),
                    }),
                )
                    .into_response();
            }
        };

        // Check permissions
        let has_permissions = if self.require_all {
            user_context.has_permissions(
                &self
                    .required_permissions
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        } else {
            user_context.has_any_permission(
                &self
                    .required_permissions
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>(),
            )
        };

        if !has_permissions {
            let required_perms: Vec<String> = self
                .required_permissions
                .iter()
                .map(|s| s.to_string())
                .collect();
            return (
                StatusCode::FORBIDDEN,
                Json(AuthorizationErrorResponse {
                    error: "Access forbidden".to_string(),
                    code: "ACCESS_FORBIDDEN".to_string(),
                    details: Some(format!("Required permissions: {:?}", required_perms)),
                }),
            )
                .into_response();
        }

        next.run(request).await
    }
}

/// Role-based authorization middleware
pub struct RoleMiddleware {
    role: &'static str,
}

impl RoleMiddleware {
    pub async fn layer(self, request: Request, next: Next) -> Response {
        // Extract user context from request extensions
        let user_context = match request.extensions().get::<UserContext>() {
            Some(context) => context,
            None => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(AuthorizationErrorResponse {
                        error: "Authentication required".to_string(),
                        code: "AUTHENTICATION_REQUIRED".to_string(),
                        details: Some("No user context found in request".to_string()),
                    }),
                )
                    .into_response();
            }
        };

        // Check role
        if !user_context.has_role(self.role) {
            return (
                StatusCode::FORBIDDEN,
                Json(AuthorizationErrorResponse {
                    error: "Access forbidden".to_string(),
                    code: "ACCESS_FORBIDDEN".to_string(),
                    details: Some(format!("Required role: {}", self.role)),
                }),
            )
                .into_response();
        }

        next.run(request).await
    }
}

/// Resource ownership validation middleware
pub struct ResourceOwnerMiddleware;

impl ResourceOwnerMiddleware {
    pub async fn layer(request: Request, next: Next) -> Response {
        // Extract user context from request extensions
        let _user_context = match request.extensions().get::<UserContext>() {
            Some(context) => context,
            None => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(AuthorizationErrorResponse {
                        error: "Authentication required".to_string(),
                        code: "AUTHENTICATION_REQUIRED".to_string(),
                        details: Some("No user context found in request".to_string()),
                    }),
                )
                    .into_response();
            }
        };

        // For now, we rely on handlers to extract tenant_id from path/query params
        // and validate ownership. This middleware sets up the user context.
        // Future enhancement: extract resource tenant_id from path and validate here.

        next.run(request).await
    }
}

/// Convenience functions for common authorization patterns
/// Require health check access (minimal permissions)
pub fn require_health_access() -> PermissionMiddleware {
    require_permissions(vec![permissions::HEALTH_READ])
}

/// Require object read access
pub async fn require_object_read(request: Request, next: Next) -> Response {
    require_permissions(vec![permissions::OBJECTS_READ])
        .layer(request, next)
        .await
}

/// Require object write access
pub async fn require_object_write(request: Request, next: Next) -> Response {
    require_permissions(vec![permissions::OBJECTS_WRITE])
        .layer(request, next)
        .await
}

/// Require object delete access
pub async fn require_object_delete(request: Request, next: Next) -> Response {
    require_permissions(vec![permissions::OBJECTS_DELETE])
        .layer(request, next)
        .await
}

/// Require API key management access
pub async fn require_api_key_management(request: Request, next: Next) -> Response {
    require_any_permission(vec![
        permissions::API_KEYS_READ,
        permissions::API_KEYS_WRITE,
        permissions::API_KEYS_DELETE,
    ])
    .layer(request, next)
    .await
}

/// Require admin access
pub fn require_admin() -> PermissionMiddleware {
    require_permissions(vec![permissions::ADMIN])
}

/// Require tenant admin access
pub fn require_tenant_admin() -> PermissionMiddleware {
    require_any_permission(vec![permissions::ADMIN, permissions::TENANT_ADMIN])
}

/// Extract tenant_id from request path or query parameters
pub fn extract_tenant_id_from_request(request: &Request) -> Option<String> {
    // Try to extract from path parameters (for routes like /v1/objects/by-key/{namespace}/{tenant_id}/{key})
    if let Some(tenant_id) = extract_from_path(request, 4) {
        return Some(tenant_id);
    }

    // Try to extract from query parameters
    if let Some(query) = request.uri().query() {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                if key == "tenant_id" {
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}

/// Extract parameter from path by index (0-based)
fn extract_from_path(request: &Request, index: usize) -> Option<String> {
    let path = request.uri().path();
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    segments.get(index).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use std::collections::HashSet;

    #[test]
    fn test_extract_tenant_id_from_path() {
        let request = Request::builder()
            .uri("/v1/objects/by-key/models/tenant123/file.txt")
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            extract_tenant_id_from_request(&request),
            Some("tenant123".to_string())
        );
    }

    #[test]
    fn test_extract_tenant_id_from_query() {
        let request = Request::builder()
            .uri("/v1/objects?namespace=models&tenant_id=tenant456")
            .body(Body::empty())
            .unwrap();

        assert_eq!(
            extract_tenant_id_from_request(&request),
            Some("tenant456".to_string())
        );
    }

    #[test]
    fn test_user_context_has_permissions() {
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

        assert!(context.has_permissions(&[permissions::OBJECTS_READ]));
        assert!(context.has_permissions(&[permissions::OBJECTS_READ, permissions::OBJECTS_WRITE]));
        assert!(!context.has_permissions(&[permissions::ADMIN]));
        assert!(context.has_any_permission(&[permissions::OBJECTS_READ, permissions::ADMIN]));
        assert!(!context.has_any_permission(&[permissions::ADMIN, permissions::TENANT_ADMIN]));
    }
}
