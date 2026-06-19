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

| Crate | Version | Role | Status |
|-------|---------|------|--------|
| `openidconnect` | `^4.0.0` | Core OIDC/OAuth2 protocols | ✅ Updated |
| `oauth2` | `^5.0.0` | Core OAuth2 protocols | ✅ Updated |
| `jsonwebtoken` | `^10.0` | High-level JWT validation with JWKS | ✅ Updated |
| `tower-sessions` | `0.14.0` | Session management with rotation | ✅ Updated |
| `moka` | `^0.12` | High-performance JWKS and session caching | ✅ Updated |

## 4. Implementation Steps

### Phase 1: Foundation & Session Hardening
1.  [x] **Dependency Update**: Target the specific versions above.
2.  [x] **Database Migration**: Create `sessions` table (See `0009_add_sessions_table.sql`).
3.  [x] **Session Configuration**:
    - [x] **Rotation**: `session.cycle_id()` integrated into auth middleware logic.
    - [x] **Encryption at Rest**: Encrypt sensitive session data before storing in Postgres (Implemented in `EncryptedPostgresStore`).
4.  [x] **HTMX Redirect Handling**:
    - [x] Implement middleware to detect `HX-Request`.
    - [x] Instead of a 302 for unauthenticated requests, return a 200 with `HX-Redirect` or `HX-Location` to the login page to avoid CORS issues with the IdP.
5.  [x] **Chrono to Time Migration**: Fully migrated all entities and repositories to `time::OffsetDateTime`.

### Phase 2: Dashboard OIDC (BFF)
1.  [x] **SSRF Lockdown**: Disable redirect-following in the OIDC HTTP client for discovery/JWKS (Implemented in `ApplicationBuilder`).
2.  [x] **Auth Routes**:
    - [x] Handle IdP error responses (`error`, `error_description`) without leaking system info (Implemented in `oidc_callback`).
    - [x] Enforce exact redirect URI matching (Configurable via `Config`).
3.  [x] **CSRF Protection**: Use synchronizer tokens for POST/PUT/DELETE (Implemented in `csrf_middleware` + HTMX header).
4.  [x] **Auth Integration Tests**: Implement integration tests with a mock OIDC IdP (Completed in `tests/e2e/security/oidc_tests.rs`).

### Phase 3: API OIDC (Resource Server)
1.  [x] **JWKS Engine**: 
    -   [x] Use `moka` to cache public keys (Implemented in `ApplicationBuilder`).
    -   [x] Implement **Background Refresh**: Fetches keys during startup and discovery.
2.  [x] **Strict Validation**:
    -   [x] Reject `none` or `HS256`. Mandate OIDC-compliant algorithms (Implemented in `AuthService`).
    -   [x] Strictly check `aud` (Audience) and `iss` (Issuer) to ensure the token was meant for this API.
3.  [x] **Unified Auth Middleware**: Refactor `AuthService` to sequentially try:
    -   Session (Dashboard context)
    -   OIDC Bearer Token (Resource Server context via JWKS)
    -   Legacy API Key (Database-backed)
    -   Simple Master Token (Env-backed)

## 5. Production Hardening Checklist
- [x] **Rate Limiting**: Aggressive limits on `/auth/login` and `/auth/callback` (Configured in factory).
- [x] **Audit Logging**: Log `authentication_attempt` with result codes to the `AuditRepository` (Implemented in `oidc_callback`).
- [x] **Session Store Cleanup**: Background worker to prune expired session rows (Implemented in `create_internal_router`).
- [x] **Secret Management**: Inject all OIDC secrets and encryption keys via env (Configured in `Config`).
- [x] **Time Handling**: Migrated to `time` crate for standard-compliant timestamp handling.
- [x] **Database Security**: Dedicated `tower_sessions` schema for session persistence.
- [x] **Modern Stack (2026)**: Updated all core dependencies to latest stable versions (`axum 0.8.8`, `tokio 1.49`, `tower-http 0.6.8`, `rand 0.8`, `jsonwebtoken 10`).
- [x] **Hygiene**: Consolidated `dev-dependencies` and replaced deprecated `serde_yaml`.

## 6. Migration & Legacy Support Strategy
1.  [x] **Parallel Authentication**: Maintain legacy API Keys and `admin_token` during transition.
2.  [x] **Simple Deployment Mode**: If `OIDC_ISSUER_URL` is missing, the system gracefully falls back to legacy API Key/Token authentication.
3.  [x] **OIDC Integration Tests**: Full E2E coverage for login flows and API token validation.
4.  [x] **Kill Switch**: Feature-flag OIDC to allow quick rollback (Implemented via `OIDC_ENABLED`).
5.  [x] **Gradual Deprecation**: Support disabling legacy methods via `LEGACY_AUTH_ENABLED` config flag.

