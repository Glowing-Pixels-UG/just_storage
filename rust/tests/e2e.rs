//! E2E (end-to-end) API tests
//!
//! Tests the full API stack including routing, middleware, and handlers.

// Import common test utilities for child modules
mod common;

#[path = "e2e/api.rs"]
mod api;
#[path = "e2e/security.rs"]
mod security;
