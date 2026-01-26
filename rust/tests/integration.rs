//! Integration tests for use cases with real database and storage
//!
//! Tests use case logic with actual PostgreSQL (via testcontainers) and filesystem storage.

// Import common test utilities
mod common;

#[path = "integration/use_cases/multi_object_operations.rs"]
mod multi_object_operations;
#[path = "integration/use_cases/namespace_validation.rs"]
mod namespace_validation;
#[path = "integration/use_cases/object_lifecycle.rs"]
mod object_lifecycle;
#[path = "integration/use_cases/storage_class_behavior.rs"]
mod storage_class_behavior;
