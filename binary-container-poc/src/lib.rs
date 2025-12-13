// SPDX-License-Identifier: MIT
//! # Binary Document Container POC
//!
//! A high-performance binary container format optimized for document bundling.
//! Designed as an experiment to compare against ZIP-based containers.
//!
//! ## Format Overview
//!
//! The BDC (Binary Document Container) format is a custom binary format designed
//! for maximum performance in document bundling scenarios. Unlike ZIP which uses
//! a central directory and requires parsing, BDC uses a fixed header with direct
//! offsets for O(1) access to any component.
//!
//! ## Key Features
//!
//! - **O(1) Random Access**: Fixed header provides direct offsets to all components
//! - **Memory Mappable**: Can map file to memory for zero-copy access
//! - **Minimal Overhead**: 32-byte header vs ZIP's variable directory overhead
//! - **Streaming Friendly**: Sequential access without directory parsing
//! - **Uniform Access**: JSON-like access to components
//! - **Optional Compression**: Per-component compression control
//!
//! ## Format Specification
//!
//! ```text
//! Binary Document Container (BDC) Format v1.0
//! ===========================================
//!
//! Header (32 bytes, little-endian):
//! - Magic: "BDC\x01\x00\x00\x00" (8 bytes)
//! - Version: 1 (4 bytes)
//! - Flags: compression flags (4 bytes)
//! - Metadata Size: compressed size (4 bytes)
//! - Asset Size: compressed size (4 bytes)
//! - Text Size: compressed size (4 bytes)
//! - Embeddings Size: compressed size (4 bytes)
//!
//! Data sections (variable size):
//! - Metadata: compressed JSON
//! - Asset: compressed binary
//! - Text: compressed UTF-8
//! - Embeddings: compressed binary
//! ```
//!
//! ## Performance Comparison
//!
//! This POC includes benchmarks comparing BDC against ZIP-based containers.
//! Expected improvements:
//!
//! - **Write Performance**: 20-50% faster (no central directory)
//! - **Read Performance**: 50-200% faster (direct offsets)
//! - **Memory Usage**: 30-50% less (no directory structures)
//! - **File Size**: Similar to ZIP (same compression algorithms)
//!
//! ## Usage
//!
//! ```rust
//! use binary_container_poc::{ContainerWriter, ContainerReader};
//!
//! // Create container with smart compression
//! let mut writer = ContainerWriter::new();
//! writer.add_metadata(b"{\"title\":\"Document\"}".to_vec()).unwrap();
//! writer.add_asset(b"document content".to_vec()).unwrap();
//! writer.add_text(b"extracted text".to_vec()).unwrap();
//! writer.add_embeddings(vec![1, 2, 3, 4]).unwrap();
//!
//! // Write to file
//! let data = writer.finalize().unwrap();
//! std::fs::write("document.bdc", &data).unwrap();
//!
//! // Read from file
//! let reader = ContainerReader::from_vec(data).unwrap();
//! let metadata = reader.metadata().unwrap();
//! let asset = reader.asset().unwrap();
//! ```
//!
//! ## Comparison with ZIP
//!
//! | Aspect | BDC | ZIP |
//! |--------|-----|-----|
//! | **Random Access** | O(1) | O(log n) |
//! | **Header Size** | 32 bytes | Variable (central dir) |
//! | **Memory Mapping** | ✅ | ❌ |
//! | **Streaming** | ✅ | ⚠️ |
//! | **Tool Support** | Custom | Universal |
//! | **Standards** | Custom | ISO 21320 |
//!
//! BDC is optimized for the specific document bundling use case where:
//! - Components are known in advance (metadata, asset, text, embeddings)
//! - Random access to components is frequent
//! - Memory mapping is beneficial
//! - Standards compliance is less important than performance

pub mod compression_strategy;
pub mod container;
pub mod format;
pub mod reader;
pub mod writer;

// Re-export main types
pub use compression_strategy::{
    CompressionConfig, CompressionEngine, CompressionRecommendation, CompressionStrategy,
    FileTypeCategory,
};
pub use container::BinaryContainer;
pub use format::ComponentType;
pub use format::{BDC_HEADER_SIZE, BDC_MAGIC};
pub use reader::{ContainerReader, ReadError};
pub use writer::{ContainerWriter, WriteError};

// Re-export from document bundler for comparison
pub use document_bundler;
