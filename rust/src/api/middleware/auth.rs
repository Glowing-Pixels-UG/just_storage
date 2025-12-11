use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,       // Subject (user/tenant ID)
    exp: usize,        // Expiration time
    iat: usize,        // Issued at
    tenant_id: String, // Tenant identifier
}

/// Authentication middleware supporting JWT and API keys
pub async fn auth_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip auth for health endpoint
    if request.uri().path() == "/health" {
        return Ok(next.run(request).await);
    }

    // Check if auth is disabled (for development)
    if env::var("DISABLE_AUTH").unwrap_or_default() == "true" {
        return Ok(next.run(request).await);
    }

    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Try API key authentication first
    if auth_header.starts_with("ApiKey ") {
        if let Some(api_key) = auth_header.strip_prefix("ApiKey ") {
            validate_api_key(api_key)?;
            return Ok(next.run(request).await);
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // Try JWT authentication
    if auth_header.starts_with("Bearer ") {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            let claims = validate_jwt(token)?;

            // Add tenant_id to request extensions for handlers to use
            request.extensions_mut().insert(claims.tenant_id);

            return Ok(next.run(request).await);
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

/// Validate API key against configured keys
fn validate_api_key(api_key: &str) -> Result<(), StatusCode> {
    let keys_string = env::var("API_KEYS").unwrap_or_default();
    let valid_keys: Vec<&str> = keys_string.split(',').map(|s| s.trim()).collect();

    if valid_keys.is_empty() {
        // No API keys configured, reject
        return Err(StatusCode::UNAUTHORIZED);
    }

    if valid_keys.contains(&api_key) {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
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
        env::set_var("API_KEYS", "key1,key2,key3");

        assert!(validate_api_key("key1").is_ok());
        assert!(validate_api_key("key2").is_ok());
        assert!(validate_api_key("invalid").is_err());

        env::remove_var("API_KEYS");
    }
}
