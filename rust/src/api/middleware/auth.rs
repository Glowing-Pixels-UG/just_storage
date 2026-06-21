use axum::{extract::Request, http::header::AUTHORIZATION, response::Response};
use futures_util::future::BoxFuture;
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tower::Layer;
use tower_sessions::Session;

use super::oidc_config::OidcConfig;
use crate::application::ports::ApiKeyRepository;
use crate::domain::authorization::{roles, CustomClaims, UserContext};

/// Claims structure for OIDC JWT tokens
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,                    // Subject (user ID)
    pub iss: Option<String>,            // Issuer
    pub aud: Option<serde_json::Value>, // Audience (can be string or array)
    pub exp: Option<usize>,             // Expiration time
    pub iat: Option<usize>,             // Issued at
    #[serde(flatten)]
    pub custom: CustomClaims, // Custom roles, tenant_id, etc.
}

#[derive(Clone)]
pub struct AuthLayer {
    api_key_repo: Arc<dyn ApiKeyRepository>,
    auth_config: crate::api::middleware::auth_config::AuthMiddlewareConfig,
    oidc_config: OidcConfig,
    jwks_cache: Arc<moka::future::Cache<String, DecodingKey>>,
    usage_tracker: Arc<dashmap::DashMap<String, std::time::Instant>>,
}

impl AuthLayer {
    pub fn new(
        api_key_repo: Arc<dyn ApiKeyRepository>,
        auth_config: crate::api::middleware::auth_config::AuthMiddlewareConfig,
        oidc_config: OidcConfig,
        jwks_cache: Arc<moka::future::Cache<String, DecodingKey>>,
    ) -> Self {
        Self {
            api_key_repo,
            auth_config,
            oidc_config,
            jwks_cache,
            usage_tracker: Arc::new(dashmap::DashMap::new()),
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
        AuthService {
            inner,
            api_key_repo: Arc::clone(&self.api_key_repo),
            auth_config: self.auth_config.clone(),
            oidc_config: self.oidc_config.clone(),
            jwks_cache: Arc::clone(&self.jwks_cache),
            usage_tracker: Arc::clone(&self.usage_tracker),
        }
    }
}

#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
    api_key_repo: Arc<dyn ApiKeyRepository>,
    auth_config: crate::api::middleware::auth_config::AuthMiddlewareConfig,
    oidc_config: OidcConfig,
    jwks_cache: Arc<moka::future::Cache<String, DecodingKey>>,
    usage_tracker: Arc<dashmap::DashMap<String, std::time::Instant>>,
}

impl<S> tower::Service<Request> for AuthService<S>
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut inner = self.inner.clone();
        let api_key_repo = Arc::clone(&self.api_key_repo);
        let auth_config = self.auth_config.clone();
        let oidc_config = self.oidc_config.clone();
        let jwks_cache = Arc::clone(&self.jwks_cache);
        let usage_tracker = Arc::clone(&self.usage_tracker);

        Box::pin(async move {
            let (mut parts, body) = req.into_parts();

            if !auth_config.enabled {
                let permissions: HashSet<String> = roles::ADMIN
                    .iter()
                    .map(|permission| (*permission).to_string())
                    .collect();
                let user_ctx = UserContext::new(
                    "disabled-auth:admin".to_string(),
                    "default".to_string(),
                    vec!["admin".to_string()],
                    permissions,
                    false,
                    None,
                );
                parts.extensions.insert(user_ctx);
                let req = Request::from_parts(parts, body);
                return inner.call(req).await;
            }

            // 1. Try Session-based authentication (Dashboard/BFF)
            if let Some(session) = parts.extensions.get::<Session>() {
                if let Ok(Some(user_ctx)) = session.get::<UserContext>("user_context").await {
                    parts.extensions.insert(user_ctx);
                    let req = Request::from_parts(parts, body);
                    return inner.call(req).await;
                }
            }

            // 2. Try Authorization header
            if let Some(auth_header) = parts
                .headers
                .get(AUTHORIZATION)
                .and_then(|h| h.to_str().ok())
            {
                if let Some(token) = auth_header.strip_prefix("Bearer ") {
                    // 2a. Try Master Token (Simple Deployment Mode)
                    if auth_config.legacy_auth_enabled {
                        if let Some(expected) = &auth_config.admin_token {
                            if token == expected {
                                let permissions: HashSet<String> =
                                    roles::get_permissions_for_role("admin")
                                        .into_iter()
                                        .map(|s| s.to_string())
                                        .collect();
                                let user_ctx = UserContext::new(
                                    "admin:master".to_string(),
                                    "default".to_string(),
                                    vec!["admin".to_string()],
                                    permissions,
                                    false,
                                    None,
                                );
                                parts.extensions.insert(user_ctx);
                                let req = Request::from_parts(parts, body);
                                return inner.call(req).await;
                            }
                        }
                    }

                    // 2b. Try API Key from Database
                    if auth_config.legacy_auth_enabled {
                        use crate::domain::value_objects::ApiKeyValue;
                        let token_hash = ApiKeyValue::hash(token);
                        if let Ok(Some(api_key)) = api_key_repo.find_by_key(token_hash.as_str()).await {
                            if api_key.is_active() && !api_key.is_expired() {
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
                                permissions.insert("health:read".to_string());

                                let user_ctx = UserContext::from_api_key(
                                    api_key.id().to_string(),
                                    api_key.tenant_id().to_string(),
                                    permissions,
                                );

                                let key_id = api_key.id().to_string();
                                let mut should_update = true;
                                if let Some(mut last_used) = usage_tracker.get_mut(&key_id) {
                                    if last_used.elapsed() < std::time::Duration::from_secs(60) {
                                        should_update = false;
                                    } else {
                                        *last_used = std::time::Instant::now();
                                    }
                                } else {
                                    usage_tracker.insert(key_id, std::time::Instant::now());
                                }

                                if should_update {
                                    let _ = api_key_repo.mark_used(api_key.id()).await;
                                }

                                parts.extensions.insert(user_ctx);
                                let req = Request::from_parts(parts, body);
                                return inner.call(req).await;
                            }
                        }
                    }

                    // 2c. Try OIDC JWT Validation
                    if oidc_config.enabled && oidc_config.issuer_url.is_some() {
                        if let Ok(header) = decode_header(token) {
                            if let Some(kid) = header.kid {
                                if let Some(decoding_key) = jwks_cache.get(&kid).await {
                                    // Mandate OIDC-compliant algorithms (Reject none, HS256, etc.)
                                    use jsonwebtoken::Algorithm;
                                    match header.alg {
                                        Algorithm::RS256
                                        | Algorithm::RS384
                                        | Algorithm::RS512
                                        | Algorithm::PS256
                                        | Algorithm::PS384
                                        | Algorithm::PS512 => {}
                                        _ => {
                                            tracing::warn!(
                                                "OIDC token uses non-compliant algorithm: {:?}",
                                                header.alg
                                            );
                                            let req = Request::from_parts(parts, body);
                                            return inner.call(req).await;
                                        }
                                    }

                                    let mut validation = Validation::new(header.alg);
                                    if let Some(iss) = &oidc_config.issuer_url {
                                        validation.set_issuer(&[iss]);
                                    }
                                    if let Some(aud) = &oidc_config.audience {
                                        validation.set_audience(&[aud]);
                                    }

                                    match decode::<Claims>(token, &decoding_key, &validation) {
                                        Ok(token_data) => {
                                            let claims = token_data.claims;
                                            let mut permissions = HashSet::new();

                                            if let Some(perms) = &claims.custom.permissions {
                                                for p in perms {
                                                    permissions.insert(p.clone());
                                                }
                                            }

                                            if let Some(user_roles) = &claims.custom.roles {
                                                for role in user_roles {
                                                    let role_perms =
                                                        roles::get_permissions_for_role(role);
                                                    for p in role_perms {
                                                        permissions.insert(p.to_string());
                                                    }
                                                }
                                            }

                                            let tenant_id = claims
                                                .custom
                                                .tenant_id
                                                .clone()
                                                .unwrap_or_else(|| "default".to_string());
                                            let user_ctx = UserContext::new(
                                                claims.sub.clone(),
                                                tenant_id,
                                                claims.custom.roles.clone().unwrap_or_default(),
                                                permissions,
                                                false,
                                                None,
                                            );

                                            parts.extensions.insert(user_ctx);
                                            let req = Request::from_parts(parts, body);
                                            return inner.call(req).await;
                                        }
                                        Err(e) => {
                                            tracing::debug!("OIDC token validation failed: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Fallback to unauthenticated
            let req = Request::from_parts(parts, body);
            inner.call(req).await
        })
    }
}

/// Create authentication middleware
pub fn create_auth_middleware(
    api_key_repo: Arc<dyn ApiKeyRepository>,
    auth_config: crate::api::middleware::auth_config::AuthMiddlewareConfig,
    oidc_config: OidcConfig,
    jwks_cache: Arc<moka::future::Cache<String, DecodingKey>>,
) -> AuthLayer {
    AuthLayer::new(api_key_repo, auth_config, oidc_config, jwks_cache)
}
