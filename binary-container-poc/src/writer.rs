// SPDX-License-Identifier: MIT
//! Binary container writer for creating BDC files

use crate::compression_strategy::{CompressionConfig, CompressionEngine};
use crate::format::{BdcHeader, ComponentType, BDC_HEADER_SIZE};
use std::io::Write;

/// Errors that can occur during writing
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Component already added: {0}")]
    ComponentAlreadyExists(String),

    #[error("Invalid component data: {0}")]
    InvalidData(String),
}

/// Builder for creating binary containers
pub struct ContainerWriter {
    header: BdcHeader,
    components: [Option<Vec<u8>>; 4], // Fixed array for known components
    compression_engine: CompressionEngine,
    asset_mime_type: Option<String>, // MIME type for asset (if known)
}

impl ContainerWriter {
    /// Create a new writer with smart compression detection
    ///
    /// Automatically detects file types and compresses only when beneficial.
    /// This is the recommended approach for production use.
    pub fn new() -> Self {
        let header = BdcHeader::new(); // Start with no compression, let smart detection decide
        Self {
            header,
            components: [None, None, None, None], // [Metadata, Asset, Text, Embeddings]
            compression_engine: CompressionEngine::new(CompressionConfig::smart()),
            asset_mime_type: None,
        }
    }

    /// Create a new writer with custom compression config
    pub fn with_config(config: CompressionConfig) -> Self {
        let header = BdcHeader::new(); // Start with no compression, let config decide
        Self {
            header,
            components: [None, None, None, None],
            compression_engine: CompressionEngine::new(config),
            asset_mime_type: None,
        }
    }

    /// Set the MIME type for the asset (for better compression decisions)
    ///
    /// Example: `writer.set_asset_mime_type("application/pdf")`
    pub fn set_asset_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.asset_mime_type = Some(mime_type.into());
        self
    }

    /// Set compression flags
    pub fn with_compression_flags(mut self, flags: u32) -> Self {
        self.header.flags = flags;
        self
    }

    /// Add metadata component
    pub fn add_metadata(&mut self, data: Vec<u8>) -> Result<(), WriteError> {
        self.add_component(ComponentType::Metadata, data)
    }

    /// Add asset component
    pub fn add_asset(&mut self, data: Vec<u8>) -> Result<(), WriteError> {
        let mime_type = self.asset_mime_type.clone(); // Clone to avoid borrow checker issues
        let mime_type_ref = mime_type.as_deref();
        self.add_component_with_mime(ComponentType::Asset, data, mime_type_ref)
    }

    /// Add asset component with explicit MIME type
    pub fn add_asset_with_mime(
        &mut self,
        data: Vec<u8>,
        mime_type: Option<&str>,
    ) -> Result<(), WriteError> {
        self.add_component_with_mime(ComponentType::Asset, data, mime_type)
    }

    /// Add text component
    pub fn add_text(&mut self, data: Vec<u8>) -> Result<(), WriteError> {
        self.add_component(ComponentType::Text, data)
    }

    /// Add embeddings component
    pub fn add_embeddings(&mut self, data: Vec<u8>) -> Result<(), WriteError> {
        self.add_component(ComponentType::Embeddings, data)
    }

    /// Add a component with optional compression
    #[inline]
    fn add_component(
        &mut self,
        component_type: ComponentType,
        data: Vec<u8>,
    ) -> Result<(), WriteError> {
        self.add_component_with_mime(component_type, data, None)
    }

    /// Add a component with optional compression and MIME type
    #[inline]
    fn add_component_with_mime(
        &mut self,
        component_type: ComponentType,
        data: Vec<u8>,
        mime_type: Option<&str>,
    ) -> Result<(), WriteError> {
        let index = component_type as usize;
        if self.components[index].is_some() {
            // Avoid allocation in error path - use static string
            return Err(WriteError::ComponentAlreadyExists(
                match component_type {
                    ComponentType::Metadata => "metadata",
                    ComponentType::Asset => "asset",
                    ComponentType::Text => "text",
                    ComponentType::Embeddings => "embeddings",
                }
                .to_string(),
            ));
        }

        // Smart compression decision
        let should_compress = if self.compression_engine.is_smart() {
            self.compression_engine
                .should_compress(component_type, &data, mime_type)
        } else {
            // Legacy: use header flags
            self.header.should_compress(component_type)
        };

        // Update header flags to reflect actual compression decision
        if should_compress {
            let flag = match component_type {
                ComponentType::Metadata => crate::format::flags::COMPRESS_METADATA,
                ComponentType::Asset => crate::format::flags::COMPRESS_ASSET,
                ComponentType::Text => crate::format::flags::COMPRESS_TEXT,
                ComponentType::Embeddings => crate::format::flags::COMPRESS_EMBEDDINGS,
            };
            self.header.flags |= flag;
        }

        let compressed_data = if should_compress {
            self.compress_data(&data)?
        } else {
            data
        };

        self.header
            .set_component_size(component_type, compressed_data.len() as u32);
        self.components[index] = Some(compressed_data);

        Ok(())
    }

    /// Compress data using zlib with optimized settings
    ///
    /// Uses zlib-rs backend (fastest) with fast compression level for optimal write speed.
    #[cfg(feature = "compression")]
    #[inline]
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>, WriteError> {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;

        // Use fastest compression level (1) for maximum write speed
        // Trade-off: slightly larger files but much faster writes
        let compression = Compression::fast();

        // Optimized capacity estimation based on typical zlib compression ratios:
        // - Text/JSON: 30-50% of original
        // - Binary: 60-80% of original (often already compressed)
        // Use conservative estimate to minimize reallocations
        let estimated_size = data.len().saturating_mul(6) / 10; // 60% of original
        let min_capacity = 256; // Smaller minimum for better memory usage

        let mut encoder = ZlibEncoder::new(
            Vec::with_capacity(estimated_size.max(min_capacity)),
            compression,
        );

        // write_all uses optimized bulk write operations
        encoder
            .write_all(data)
            .map_err(|e| WriteError::Compression(format!("Write failed: {}", e)))?;

        encoder
            .finish()
            .map_err(|e| WriteError::Compression(format!("Finish failed: {}", e)))
    }

    /// Compress data (no-op when compression is disabled)
    #[cfg(not(feature = "compression"))]
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>, WriteError> {
        Ok(data.to_vec())
    }

    /// Finalize the container and return the binary data
    ///
    /// Optimized for minimal allocations and maximum write speed.
    #[inline]
    pub fn finalize(self) -> Result<Vec<u8>, WriteError> {
        // Validate all components are present (single pass, avoid allocations in error path)
        let component_refs: [&Vec<u8>; 4] = [
            self.components[0]
                .as_ref()
                .ok_or_else(|| WriteError::InvalidData("Missing metadata".into()))?,
            self.components[1]
                .as_ref()
                .ok_or_else(|| WriteError::InvalidData("Missing asset".into()))?,
            self.components[2]
                .as_ref()
                .ok_or_else(|| WriteError::InvalidData("Missing text".into()))?,
            self.components[3]
                .as_ref()
                .ok_or_else(|| WriteError::InvalidData("Missing embeddings".into()))?,
        ];

        // Calculate total size (manual sum to avoid iterator overhead)
        // Use checked_add for safety, but in practice this should never overflow
        let header_size = BDC_HEADER_SIZE;
        let data_size = component_refs[0]
            .len()
            .checked_add(component_refs[1].len())
            .and_then(|s| s.checked_add(component_refs[2].len()))
            .and_then(|s| s.checked_add(component_refs[3].len()))
            .ok_or_else(|| WriteError::InvalidData("Total size overflow".into()))?;

        let total_size = header_size
            .checked_add(data_size)
            .ok_or_else(|| WriteError::InvalidData("Total size overflow".into()))?;

        // Pre-allocate exact size to avoid any reallocations
        // Vec::with_capacity is highly optimized and will allocate exactly what we need
        let mut buffer = Vec::with_capacity(total_size);

        // Write header (32 bytes) - optimized method
        self.header.write_to_buffer(&mut buffer);

        // Write components in order using extend_from_slice
        // This is the fastest way to append slices - uses memcpy internally
        buffer.extend_from_slice(component_refs[0]);
        buffer.extend_from_slice(component_refs[1]);
        buffer.extend_from_slice(component_refs[2]);
        buffer.extend_from_slice(component_refs[3]);

        debug_assert_eq!(buffer.len(), total_size);
        Ok(buffer)
    }
}

impl Default for ContainerWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContainerReader;

    #[test]
    fn test_writer_new() {
        let writer = ContainerWriter::new();
        assert!(writer.components.iter().all(|c| c.is_none()));
    }

    #[test]
    fn test_add_components() {
        let mut writer = ContainerWriter::new();

        writer.add_metadata(b"metadata".to_vec()).unwrap();
        writer.add_asset(b"asset".to_vec()).unwrap();
        writer.add_text(b"text".to_vec()).unwrap();
        writer.add_embeddings(b"embeddings".to_vec()).unwrap();

        assert!(writer.components.iter().all(|c| c.is_some()));
    }

    #[test]
    fn test_add_duplicate_component() {
        let mut writer = ContainerWriter::new();
        writer.add_metadata(b"data1".to_vec()).unwrap();

        let result = writer.add_metadata(b"data2".to_vec());
        assert!(result.is_err());
    }

    #[test]
    fn test_finalize_missing_components() {
        let writer = ContainerWriter::new();
        let result = writer.finalize();
        assert!(result.is_err());
    }

    #[test]
    fn test_finalize_complete() {
        let mut writer = ContainerWriter::new();
        writer.add_metadata(b"metadata".to_vec()).unwrap();
        writer.add_asset(b"asset".to_vec()).unwrap();
        writer.add_text(b"text".to_vec()).unwrap();
        writer.add_embeddings(b"embeddings".to_vec()).unwrap();

        let result = writer.finalize();
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(data.len() > 32); // Header + data

        // Verify we can read it back
        let reader = ContainerReader::from_slice(&data);
        assert!(reader.is_ok());
    }
}
