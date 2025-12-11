//! # JustStorage - Content-Addressable Object Storage
//!
//! A focused, reliable object storage service with strong consistency guarantees,
//! built on Clean Architecture principles.
//!
//! ## Architecture Layers
//!
//! - **Domain**: Core business logic (entities, value objects, domain errors)
//! - **Application**: Use cases and ports (interfaces)
//! - **Infrastructure**: Adapters for storage and persistence
//! - **API**: HTTP handlers and middleware
//!
//! ## Key Features
//!
//! - Content-addressable storage with automatic deduplication
//! - Two-phase writes for crash safety
//! - Background garbage collection
//! - JWT and API key authentication
//! - Hot/cold storage classes
//!
//! ## Example Usage
//!
//! ```no_run
//! use just_storage::{Config, use_cases::UploadObjectUseCase};
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Setup repositories and storage (see integration tests for full example)
//! # Ok(())
//! # }
//! ```

pub mod api;
pub mod application;
pub mod config;
pub mod domain;
pub mod infrastructure;

// Re-export key types explicitly to avoid ambiguity
pub use api::errors as api_errors;
pub use application::{dto, ports, use_cases};
pub use config::Config;
pub use domain::errors as domain_errors;
pub use domain::{entities, value_objects};
