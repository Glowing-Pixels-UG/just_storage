//! Common validation utilities for use cases
//!
//! This module provides reusable validation functions to reduce
//! duplication across use case implementations.

use crate::application::errors::ObjectUseCaseError;
use crate::domain::value_objects::{Namespace, TenantId};

/// Validate namespace and tenant_id for object operations
///
/// Returns the validated values or an ObjectUseCaseError
pub fn validate_namespace_and_tenant(
    namespace: &str,
    tenant_id: &str,
) -> Result<(Namespace, TenantId), ObjectUseCaseError> {
    let namespace = Namespace::new(namespace.to_string())
        .map_err(|e| ObjectUseCaseError::InvalidRequest(e.to_string()))?;

    let tenant_id = TenantId::from_string(tenant_id)
        .map_err(|e| ObjectUseCaseError::InvalidRequest(e.to_string()))?;

    Ok((namespace, tenant_id))
}

/// Validate namespace and tenant_id for text search operations
///
/// Returns the validated values or a TextSearchUseCaseError
pub fn validate_namespace_and_tenant_for_text_search(
    namespace: &str,
    tenant_id: &str,
) -> Result<(Namespace, TenantId), crate::application::errors::TextSearchUseCaseError> {
    let namespace = Namespace::new(namespace.to_string()).map_err(|e| {
        crate::application::errors::TextSearchUseCaseError::InvalidRequest(e.to_string())
    })?;

    let tenant_id = TenantId::from_string(tenant_id).map_err(|e| {
        crate::application::errors::TextSearchUseCaseError::InvalidRequest(e.to_string())
    })?;

    Ok((namespace, tenant_id))
}

/// Validate that a search query is not empty
pub fn validate_search_query(
    query: &str,
) -> Result<(), crate::application::errors::TextSearchUseCaseError> {
    if query.trim().is_empty() {
        return Err(
            crate::application::errors::TextSearchUseCaseError::InvalidRequest(
                "Search query cannot be empty".to_string(),
            ),
        );
    }
    Ok(())
}
