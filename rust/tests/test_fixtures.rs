//! Shared test fixtures and utilities for all test types
//!
//! This module provides common test setup patterns to reduce duplication
//! and make tests more maintainable.



// Re-export the shared TestEnvironment from `tests/common`
// This eliminates the duplicate implementation and ensures all tests
// use a single source of truth for environment setup.
pub use crate::common::TestEnvironment;

// Re-export shared DB & storage helpers from `tests/common::database`
pub use crate::common::database::{cleanup_test_data, setup_test_database, setup_test_storage};

// Re-export domain factories from shared `tests/common/fixtures`
pub use crate::common::fixtures::{create_test_object, create_test_blob_from_hex, create_test_blob, create_custom_object};

#[path = "common/mod.rs"]
mod common;

pub use crate::common::assertions;
pub use common::http;
pub use common::mocks;
