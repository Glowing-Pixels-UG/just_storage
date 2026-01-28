# Development Plan: Unified OIDC Authentication (Production-Ready 2026)

## 1. Overview
Implement a unified, production-grade OpenID Connect (OIDC) system for both the **Dashboard (BFF)** and **API (Resource Server)**. This plan adheres to **OAuth 2.1 / FAPI 2.0 Security Profiles** and incorporates advanced production hardening for high-availability Rust services.

## 2. Architecture & Identity Model

### Dual-Purpose Authentication
- **Dashboard (BFF)**: Rust backend handles OIDC flow -> stores tokens in server-side session -> issues opaque, rotated session cookie.
- **API (Resource Server)**: Backend validates incoming Bearer JWTs against the OIDC provider's JWKS (JSON Web Key Set).

### Identity & Multi-Tenancy Mapping
- **Stable Identifier**: All users uniquely identified by `(iss, sub)`.
- **Tenant Mapping**: Map the OIDC `iss` (Issuer) or a specific claim (e.g., `tenant_id` from Entra ID/Okta) to the internal `tenant_id` in the database.
- **UserContext**: Shared identity structure populated by either session or bearer token validation.

## 3. Recommended Crate Stack (Verified 2026)

| Crate | Version | Role |
|-------|---------|------|
| `openidconnect` | `^4.0.0` | Core OIDC/OAuth2 protocols |
| `axum-oidc` | `0.6.0` | Dashboard OIDC layers and extractors |
| `jwt-authorizer` | `^0.14` | High-level JWT validation with JWKS refresh |
| `tower-sessions` | `0.14.0` | Session management with rotation |
| `moka` | `^0.12` | High-performance JWKS and session caching |

## 4. Implementation Steps

### Phase 1: Foundation & Session Hardening
1. **Dependency Update**: Target the specific versions above.
2. **Database Migration**: Create `sessions` table.
3. **Session Configuration**:
   - **Rotation**: `session.cycle_id()` on every login.
   - **Encryption at Rest**: Encrypt sensitive session data before storing in Postgres.
4. **HTMX Redirect Handling**:
   - Implement middleware to detect `HX-Request`.
   - Instead of a 302 for unauthenticated requests, return a 200 with `HX-Redirect` or `HX-Location` to the login page to avoid CORS issues with the IdP.

### Phase 2: Dashboard OIDC (BFF)
1. **SSRF Lockdown**: Disable redirect-following in the OIDC HTTP client for discovery/JWKS.
2. **Auth Routes**:
   - Handle IdP error responses (`error`, `error_description`) without leaking system info.
   - Enforce exact redirect URI matching.
3. **CSRF Protection**: Use synchronizer tokens or strict `Origin` + `SameSite=Lax` for POST/PUT/DELETE.

### Phase 3: API OIDC (Resource Server)
1. **JWKS Engine**: 
   - Use `jwt-authorizer` or `moka` to cache public keys.
   - Implement **Background Refresh**: Fetch keys before expiry to avoid "thundering herd" latency spikes.
2. **Strict Validation**:
   - Reject `none` or `HS256`. Mandate `RS256` or `ES256`.
   - Allow 1-2 minutes for clock drift.
   - Strictly check `aud` (Audience) to ensure the token was meant for this API.

## 5. Production Hardening Checklist
- [x] **Rate Limiting**: Aggressive limits on `/auth/login` and `/auth/callback`.
- [x] **Audit Logging**: Log `authentication_attempt` with result codes to the `AuditRepository`.
- [x] **Session Store Cleanup**: Background worker to prune expired session rows.
- [x] **Secret Management**: Inject all OIDC secrets and encryption keys via env (never committed).
- [x] **DPoP Support**: Evaluate if the IdP supports DPoP for stronger sender-constraint.

## 6. Migration Strategy
1. **Parallel Authentication**: Maintain legacy API Keys and `admin_token` during transition.
2. **Kill Switch**: Feature-flag OIDC to allow quick rollback.
3. **Gradual Deprecation**: Turn off legacy methods only after OIDC stability is verified.
