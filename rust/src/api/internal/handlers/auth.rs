use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect, Response},
};
use openidconnect::core::{CoreClient, CoreResponseType};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, AuthenticationFlow,
};
use serde::Deserialize;
use tower_sessions::Session;
use tracing::{error, info};

use crate::api::router::AppState;
use crate::domain::authorization::{roles, UserContext};
use std::collections::HashSet;

#[derive(Deserialize)]
pub struct AuthCallbackParams {
    code: String,
    state: String,
}

/// Start OIDC login flow
pub async fn oidc_login(
    State(state): State<AppState>,
    session: Session,
) -> impl IntoResponse {
    let metadata = match &state.oidc_metadata {
        Some(m) => m,
        None => return Redirect::to("/dashboard/login").into_response(),
    };

    let client_id = ClientId::new(state.config.oidc_client_id.clone().unwrap_or_default());
    let client_secret = state.config.oidc_client_secret.clone().map(ClientSecret::new);
    
    let client = CoreClient::from_provider_metadata(
        metadata.clone(),
        client_id,
        client_secret,
    );

    let mut client = client;
    if let Some(redirect_url) = &state.config.oidc_redirect_url {
        client = client.set_redirect_uri(RedirectUrl::new(redirect_url.clone()).expect("Invalid redirect URL"));
    }

    // Generate PKCE challenge
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate state and nonce
    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    // Store verifier, state and nonce in session
    let _: () = session.insert("oidc_pkce_verifier", pkce_verifier).await.unwrap_or_else(|e| error!("Session error: {}", e));
    let _: () = session.insert("oidc_state", csrf_token.secret().clone()).await.unwrap_or_else(|e| error!("Session error: {}", e));
    let _: () = session.insert("oidc_nonce", nonce.secret().clone()).await.unwrap_or_else(|e| error!("Session error: {}", e));

    Redirect::to(auth_url.as_str()).into_response()
}

/// Manual async HTTP client for OIDC code exchange
async fn oidc_http_client(
    request: openidconnect::HttpRequest,
) -> Result<openidconnect::HttpResponse, oauth2::HttpClientError<reqwest::Error>> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| oauth2::HttpClientError::Reqwest(Box::new(e)))?;

    let method = request.method().clone();
    let url = request.uri().to_string();

    let mut request_builder = client.request(method, url);

    for (name, value) in request.headers() {
        request_builder = request_builder.header(name.as_str(), value.as_bytes());
    }

    let response = request_builder
        .body(request.body().clone())
        .send()
        .await
        .map_err(|e| oauth2::HttpClientError::Reqwest(Box::new(e)))?;

    let status_code = response.status();
    let mut http_response = openidconnect::http::Response::builder()
        .status(status_code);
    
    for (name, value) in response.headers() {
        http_response = http_response.header(name, value);
    }
    
    let body = response
        .bytes()
        .await
        .map_err(|e| oauth2::HttpClientError::Reqwest(Box::new(e)))?
        .to_vec();

    http_response.body(body).map_err(|e| oauth2::HttpClientError::Other(e.to_string()))
}

/// Handle OIDC callback
pub async fn oidc_callback(
    State(state): State<AppState>,
    session: Session,
    params: Query<AuthCallbackParams>,
) -> Response {
    let metadata = match &state.oidc_metadata {
        Some(m) => m,
        None => return Redirect::to("/dashboard/login").into_response(),
    };

    let client_id = ClientId::new(state.config.oidc_client_id.clone().unwrap_or_default());
    let client_secret = state.config.oidc_client_secret.clone().map(ClientSecret::new);
    
    let client = CoreClient::from_provider_metadata(
        metadata.clone(),
        client_id,
        client_secret,
    );

    let mut client = client;
    if let Some(redirect_url) = &state.config.oidc_redirect_url {
        client = client.set_redirect_uri(RedirectUrl::new(redirect_url.clone()).expect("Invalid redirect URL"));
    }

    // 2. Validate state
    let stored_state: String = match session.get("oidc_state").await {
        Ok(Some(s)) => s,
        _ => {
            error!("OIDC state not found in session");
            return Redirect::to("/dashboard/login").into_response();
        }
    };
    if params.state != stored_state {
        error!("OIDC state mismatch");
        
        // Log authentication failure
        let log_entry = crate::api::middleware::audit::AuditLogEntry {
            timestamp: time::OffsetDateTime::now_utc(),
            event_type: crate::api::middleware::audit::AuditEventType::AuthenticationFailure,
            user_id: None,
            tenant_id: None,
            api_key_id: None,
            ip_address: None,
            user_agent: None,
            method: "GET".to_string(),
            path: "/auth/callback".to_string(),
            query: None,
            status_code: Some(401),
            response_time_ms: None,
            error_message: Some("OIDC state mismatch".to_string()),
            additional_data: None,
        };
        let _ = state.audit_repo.store(log_entry).await;

        return Redirect::to("/dashboard/login").into_response();
    }

    // 3. Get PKCE verifier and nonce
    let pkce_verifier: PkceCodeVerifier = match session.get("oidc_pkce_verifier").await {
        Ok(Some(v)) => v,
        _ => {
            error!("OIDC PKCE verifier not found in session");
            return Redirect::to("/dashboard/login").into_response();
        }
    };
    let stored_nonce: String = match session.get("oidc_nonce").await {
        Ok(Some(n)) => n,
        _ => {
            error!("OIDC nonce not found in session");
            return Redirect::to("/dashboard/login").into_response();
        }
    };

    // 4. Exchange code for tokens
    let token_request = match client
        .exchange_code(AuthorizationCode::new(params.code.clone())) {
            Ok(req) => req.set_pkce_verifier(pkce_verifier),
            Err(e) => {
                error!("Failed to create OIDC token request: {}", e);
                return Redirect::to("/dashboard/login").into_response();
            }
        };
    
    let token_response: openidconnect::core::CoreTokenResponse = match token_request
        .request_async(&oidc_http_client)
        .await
    {
        Ok(res) => res,
        Err(e) => {
            error!("Failed to exchange OIDC code: {}", e);
            return Redirect::to("/dashboard/login").into_response();
        }
    };

    // 5. Validate ID token
    let id_token = match token_response.id_token() {
        Some(t) => t,
        None => {
            error!("OIDC IdP did not return an ID token");
            return Redirect::to("/dashboard/login").into_response();
        }
    };

    let claims = match id_token.claims(&client.id_token_verifier(), &Nonce::new(stored_nonce)) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to validate OIDC ID token: {}", e);
            return Redirect::to("/dashboard/login").into_response();
        }
    };

    // 6. Create UserContext and rotate session
    let user_id = claims.subject().to_string();
    
    // Extract additional claims manually from the ID token's JSON representation
    let id_token_json: serde_json::Value = match serde_json::to_value(id_token) {
        Ok(v) => v,
        Err(_) => serde_json::Value::Null,
    };
    
    // In openidconnect-rs, IdToken serializes to a JSON object with a "claims" field
    let claims_json = &id_token_json;
    
    let tenant_id = claims_json["tenant_id"].as_str().map(|s| s.to_string()).unwrap_or_else(|| "default".to_string());
    let user_roles = claims_json["roles"].as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["user".to_string()]);
    
    let mut permissions = HashSet::new();
    if let Some(perms_arr) = claims_json["permissions"].as_array() {
        for v in perms_arr {
            if let Some(p) = v.as_str() { permissions.insert(p.to_string()); }
        }
    }
    
    for role in &user_roles {
        let role_perms = roles::get_permissions_for_role(role);
        for p in role_perms { permissions.insert(p.to_string()); }
    }

    // Log successful login to AuditRepository
    let log_entry = crate::api::middleware::audit::AuditLogEntry {
        timestamp: time::OffsetDateTime::now_utc(),
        event_type: crate::api::middleware::audit::AuditEventType::AuthenticationSuccess,
        user_id: Some(user_id.clone()),
        tenant_id: Some(tenant_id.clone()),
        api_key_id: None,
        ip_address: None, 
        user_agent: None,
        method: "GET".to_string(),
        path: "/auth/callback".to_string(),
        query: None,
        status_code: Some(200),
        response_time_ms: None,
        error_message: None,
        additional_data: Some(serde_json::json!({ 
            "provider": claims.issuer().to_string(),
            "roles": user_roles,
            "tenant_id": tenant_id
        })),
    };
    
    if let Err(e) = state.audit_repo.store(log_entry).await {
        error!("Failed to store audit log for OIDC login: {}", e);
    }

    let user_ctx = UserContext::new(
        user_id.clone(),
        tenant_id,
        user_roles,
        permissions,
        false,
        None,
    );

    let _: () = session.insert("user_context", user_ctx).await.unwrap_or_else(|e| error!("Session error: {}", e));
    
    // Rotate session ID (fixation defense)
    if let Err(e) = session.cycle_id().await {
        error!("Failed to rotate session ID: {}", e);
    }

    // 7. Cleanup OIDC session data
    let _ = session.remove::<String>("oidc_state").await;
    let _ = session.remove::<String>("oidc_nonce").await;
    let _ = session.remove::<PkceCodeVerifier>("oidc_pkce_verifier").await;

    info!("User {} logged in via OIDC", user_id);
    Redirect::to("/dashboard/health").into_response()
}

/// Logout and clear session
pub async fn oidc_logout(session: Session) -> impl IntoResponse {
    let _ = session.clear().await;
    Redirect::to("/dashboard/login")
}
