use base64::Engine;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::traits::PublicKeyParts;
use serde::{Deserialize, Serialize};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[allow(dead_code)]
pub struct MockOidcServer {
    pub server: MockServer,
    pub issuer_url: String,
    pub private_key_der: Vec<u8>,
    pub public_key_jwk: serde_json::Value,
    pub kid: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize)]
struct TestClaims {
    sub: String,
    iss: String,
    aud: String,
    exp: usize,
    iat: usize,
    tenant_id: String,
    roles: Vec<String>,
}

impl MockOidcServer {
    #[allow(dead_code)]
    pub async fn new() -> Self {
        let server = MockServer::start().await;
        let issuer_url = server.uri();
        let kid = "test-kid".to_string();

        // Generate a real RSA key pair for signing tokens in tests.
        // rsa 0.9 is built against rand_core 0.6; use its re-exported OsRng so
        // the RNG trait bounds line up (rand 0.9's `rng()` exposes rand_core 0.10,
        // which does not satisfy `RsaPrivateKey::new`).
        let mut rng = rsa::rand_core::OsRng;
        let rsa = rsa::RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate key");

        // Try PKCS#1 DER for jsonwebtoken v10 compatibility
        let private_key_der = rsa.to_pkcs1_der().unwrap().as_bytes().to_vec();

        let public_key = rsa::RsaPublicKey::from(&rsa);
        let n =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
        let e =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());

        let public_key_jwk = json!({
            "kty": "RSA",
            "use": "sig",
            "alg": "RS256",
            "kid": kid,
            "n": n,
            "e": e,
        });

        // 1. Mock OIDC Discovery
        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "issuer": issuer_url,
                "authorization_endpoint": format!("{}/authorize", issuer_url),
                "token_endpoint": format!("{}/token", issuer_url),
                "jwks_uri": format!("{}/jwks", issuer_url),
                "userinfo_endpoint": format!("{}/userinfo", issuer_url),
                "response_types_supported": ["code", "id_token"],
                "subject_types_supported": ["public"],
                "id_token_signing_alg_values_supported": ["RS256"]
            })))
            .mount(&server)
            .await;

        // 2. Mock JWKS
        let jwk_clone = public_key_jwk.clone();
        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "keys": [jwk_clone]
            })))
            .mount(&server)
            .await;

        Self {
            server,
            issuer_url,
            private_key_der,
            public_key_jwk,
            kid,
        }
    }

    #[allow(dead_code)]
    pub fn generate_token(&self, sub: &str, tenant_id: &str, roles: Vec<String>) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let claims = TestClaims {
            sub: sub.to_string(),
            iss: self.issuer_url.clone(),
            aud: "test-client".to_string(),
            exp: now + 3600,
            iat: now,
            tenant_id: tenant_id.to_string(),
            roles,
        };

        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.kid.clone());

        let encoding_key = EncodingKey::from_rsa_der(&self.private_key_der);
        jsonwebtoken::encode(&header, &claims, &encoding_key).unwrap()
    }
}
