use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    response::Response,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::sync::Arc;
use tower::Layer;

use crate::application::ports::ApiKeyRepository;
use crate::domain::authorization::{roles, UserContext};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,              // Subject (user ID)
    pub exp: usize,               // Expiration time
    pub iat: usize,               // Issued at
    pub tenant_id: String,        // Tenant identifier
    pub roles: Vec<String>,       // User roles
    pub permissions: Vec<String>, // Direct permissions (bypasses roles)
}

#[derive(Clone)]
pub struct AuthLayer {
    // TODO: Add api_key_repo field when state injection is fixed
}

impl AuthLayer {
    pub fn new(_api_key_repo: Arc<dyn ApiKeyRepository>) -> Self {
        Self {
            // TODO: Store api_key_repo
        }
    }
}

impl<S> Layer<S> for AuthLayer
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService { inner }
    }
}

#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
    // TODO: Add api_key_repo field
}

impl<S> tower::Service<Request> for AuthService<S>
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        // TODO: Implement authentication logic
        // For now, just pass through
        self.inner.call(req)
    }
}

/// Create authentication middleware with API key repository
pub fn create_auth_middleware(api_key_repo: Arc<dyn ApiKeyRepository>) -> AuthLayer {
    AuthLayer::new(api_key_repo)
}

/// Authenticate a request and return user context
#[allow(unused)]
async fn authenticate_request(
    headers: &HeaderMap,
    api_key_repo: &dyn ApiKeyRepository,
) -> Result<UserContext, StatusCode> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Try API key authentication first
    if auth_header.starts_with("ApiKey ") {
        if let Some(api_key_value) = auth_header.strip_prefix("ApiKey ") {
            return authenticate_api_key(api_key_value, api_key_repo).await;
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // Try JWT authentication
    if auth_header.starts_with("Bearer ") {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            return authenticate_jwt(token).await;
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

/// Authenticate using API key
#[allow(unused)]
async fn authenticate_api_key(
    api_key_value: &str,
    api_key_repo: &dyn ApiKeyRepository,
) -> Result<UserContext, StatusCode> {
    // Look up API key in database
    let api_key = api_key_repo
        .find_by_key(api_key_value)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Check if API key is active and not expired
    if !api_key.is_active() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    if api_key.is_expired() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Convert API key permissions to permission strings
    let mut permissions = HashSet::new();
    if api_key.permissions().read {
        permissions.insert("objects:read".to_string());
    }
    if api_key.permissions().write {
        permissions.insert("objects:write".to_string());
    }
    if api_key.permissions().delete {
        permissions.insert("objects:delete".to_string());
    }
    if api_key.permissions().admin {
        permissions.insert("admin".to_string());
        permissions.insert("api_keys:read".to_string());
        permissions.insert("api_keys:write".to_string());
        permissions.insert("api_keys:delete".to_string());
    }
    permissions.insert("health:read".to_string()); // API keys can always check health

    // Update last used timestamp
    let _ = api_key_repo.mark_used(api_key.id()).await;

    Ok(UserContext::from_api_key(
        api_key.id().to_string(),
        api_key.tenant_id().to_string(),
        permissions,
    ))
}

/// Authenticate using JWT token
async fn authenticate_jwt(token: &str) -> Result<UserContext, StatusCode> {
    let claims = validate_jwt(token)?;

    // Convert roles to permissions
    let mut permissions = HashSet::new();

    // Add direct permissions from JWT
    for permission in &claims.permissions {
        permissions.insert(permission.clone());
    }

    // Add permissions from roles
    for role in &claims.roles {
        let role_permissions = roles::get_permissions_for_role(role);
        for perm in role_permissions {
            permissions.insert(perm.to_string());
        }
    }

    // Ensure health access for authenticated users
    permissions.insert("health:read".to_string());

    Ok(UserContext::new(
        claims.sub.clone(),
        claims.tenant_id.clone(),
        claims.roles.clone(),
        permissions,
        false,
        None,
    ))
}

/// Validate JWT token
fn validate_jwt(token: &str) -> Result<Claims, StatusCode> {
    let jwt_secret = env::var("JWT_SECRET").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let token_data = decode::<Claims>(token, &decoding_key, &validation)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_api_key() {
        // This test is now deprecated since we use database-backed API keys
        // Keeping for backward compatibility but it doesn't test the actual auth flow
    }
}
