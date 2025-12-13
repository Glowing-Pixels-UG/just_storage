// SPDX-License-Identifier: MIT
//! Binary container reader for reading BDC files

use crate::container::BinaryContainer;
use crate::format::{BdcHeader, ComponentType};

/// Errors that can occur during reading
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Decompression error: {0}")]
    Decompression(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Component not found: {0}")]
    ComponentNotFound(String),
}

/// Reader for binary containers
pub struct ContainerReader {
    container: BinaryContainer,
}

impl ContainerReader {
    /// Create a reader from borrowed data
    ///
    /// This is the primary constructor for reading BDC files.
    pub fn from_slice(data: &[u8]) -> Result<Self, ReadError> {
        let container = BinaryContainer::from_slice(data).map_err(ReadError::InvalidFormat)?;
        Ok(Self { container })
    }

    /// Create a reader from owned data
    ///
    /// Convenience constructor when you have owned Vec<u8> data.
    pub fn from_vec(data: Vec<u8>) -> Result<Self, ReadError> {
        Self::from_slice(&data)
    }

    /// Get the container header
    pub fn header(&self) -> &BdcHeader {
        self.container.header()
    }

    /// Get raw component data
    pub fn get_component_raw(&self, component_type: ComponentType) -> Result<&[u8], ReadError> {
        self.container
            .get_component(component_type)
            .ok_or_else(|| ReadError::ComponentNotFound(component_type.name().to_string()))
    }

    /// Get decompressed component data
    #[inline]
    pub fn get_component(&self, component_type: ComponentType) -> Result<Vec<u8>, ReadError> {
        let raw_data = self.get_component_raw(component_type)?;

        if self.container.header().should_compress(component_type) {
            self.decompress_data(raw_data)
        } else {
            // For uncompressed data, we still need to allocate for ownership
            // Use to_vec() which is optimized by the compiler
            Ok(raw_data.to_vec())
        }
    }

    /// Get component data as Cow for zero-copy when possible
    pub fn get_component_cow(
        &self,
        component_type: ComponentType,
    ) -> Result<std::borrow::Cow<'_, [u8]>, ReadError> {
        let raw_data = self.get_component_raw(component_type)?;

        if self.container.header.should_compress(component_type) {
            // Must decompress, so allocate
            self.decompress_data(raw_data).map(std::borrow::Cow::Owned)
        } else {
            // No compression, can return borrowed data (true zero-copy)
            Ok(std::borrow::Cow::Borrowed(raw_data))
        }
    }

    /// Decompress data using zlib with optimized capacity estimation
    #[cfg(feature = "compression")]
    #[inline]
    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>, ReadError> {
        use flate2::read::ZlibDecoder;
        use std::io::Read;

        // Optimized capacity estimation: zlib typically achieves 2-4x compression
        // Use integer math to avoid floating point overhead
        let estimated_size = data.len().saturating_mul(3).max(1024);
        let mut decompressed = Vec::with_capacity(estimated_size);

        let mut decoder = ZlibDecoder::new(data);
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| ReadError::Decompression(e.to_string()))?;

        // Only shrink if we significantly over-allocated (avoid frequent reallocs)
        if decompressed.capacity() > decompressed.len().saturating_mul(2) {
            decompressed.shrink_to_fit();
        }

        Ok(decompressed)
    }

    /// Decompress data (no-op when compression is disabled)
    #[cfg(not(feature = "compression"))]
    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>, ReadError> {
        Ok(data.to_vec())
    }

    /// Uniform access methods (like JSON field access)
    pub fn metadata(&self) -> Result<Vec<u8>, ReadError> {
        self.get_component(ComponentType::Metadata)
    }

    pub fn asset(&self) -> Result<Vec<u8>, ReadError> {
        self.get_component(ComponentType::Asset)
    }

    pub fn text(&self) -> Result<Vec<u8>, ReadError> {
        self.get_component(ComponentType::Text)
    }

    pub fn embeddings(&self) -> Result<Vec<u8>, ReadError> {
        self.get_component(ComponentType::Embeddings)
    }

    /// Get metadata as JSON string
    pub fn metadata_json(&self) -> Result<String, ReadError> {
        let data = self.metadata()?;
        String::from_utf8(data)
            .map_err(|e| ReadError::InvalidFormat(format!("Invalid UTF-8 in metadata: {}", e)))
    }

    /// Get text as string
    pub fn text_string(&self) -> Result<String, ReadError> {
        let data = self.text()?;
        String::from_utf8(data)
            .map_err(|e| ReadError::InvalidFormat(format!("Invalid UTF-8 in text: {}", e)))
    }

    /// Stream component data (for large files)
    pub fn stream_component<F>(
        &self,
        component_type: ComponentType,
        mut callback: F,
    ) -> Result<(), ReadError>
    where
        F: FnMut(&[u8]) -> Result<(), ReadError>,
    {
        let data = self.get_component_raw(component_type)?;
        callback(data)
    }

    /// Get container statistics
    pub fn stats(&self) -> ContainerStats {
        let header = self.container.header();
        ContainerStats {
            total_size: self.container.size(),
            header_size: crate::format::BDC_HEADER_SIZE,
            metadata_size: header.metadata_size as usize,
            asset_size: header.asset_size as usize,
            text_size: header.text_size as usize,
            embeddings_size: header.embeddings_size as usize,
            compression_flags: header.flags,
        }
    }
}

/// Container statistics
#[derive(Debug, Clone)]
pub struct ContainerStats {
    pub total_size: usize,
    pub header_size: usize,
    pub metadata_size: usize,
    pub asset_size: usize,
    pub text_size: usize,
    pub embeddings_size: usize,
    pub compression_flags: u32,
}

impl ContainerStats {
    /// Calculate compression ratio (compressed / uncompressed)
    pub fn compression_ratio(&self) -> f64 {
        // This is a simplified calculation - in reality we'd need uncompressed sizes
        // For now, just return the overhead ratio
        let data_size =
            self.metadata_size + self.asset_size + self.text_size + self.embeddings_size;
        let overhead_ratio = self.header_size as f64 / (self.header_size + data_size) as f64;
        1.0 - overhead_ratio
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::ContainerWriter;

    fn create_test_data() -> Vec<u8> {
        let mut writer = ContainerWriter::new();
        writer.add_metadata(b"metadata".to_vec()).unwrap();
        writer.add_asset(b"asset".to_vec()).unwrap();
        writer.add_text(b"text".to_vec()).unwrap();
        writer.add_embeddings(b"embeddings".to_vec()).unwrap();

        writer.finalize().unwrap()
    }

    #[test]
    fn test_reader_from_vec() {
        let data = create_test_data();
        let reader = ContainerReader::from_vec(data);
        assert!(reader.is_ok());
    }

    #[test]
    fn test_reader_from_slice() {
        let data = create_test_data();
        let reader = ContainerReader::from_slice(&data);
        assert!(reader.is_ok());
    }

    #[test]
    fn test_get_component_raw() {
        let data = create_test_data();
        let reader = ContainerReader::from_slice(&data).unwrap();

        let metadata = reader.get_component_raw(ComponentType::Metadata).unwrap();
        assert!(!metadata.is_empty());
    }

    #[test]
    fn test_uniform_access() {
        let data = create_test_data();
        let reader = ContainerReader::from_slice(&data).unwrap();

        let metadata = reader.metadata().unwrap();
        let asset = reader.asset().unwrap();
        let text = reader.text().unwrap();
        let embeddings = reader.embeddings().unwrap();

        assert!(!metadata.is_empty());
        assert!(!asset.is_empty());
        assert!(!text.is_empty());
        assert!(!embeddings.is_empty());
    }

    #[test]
    fn test_stats() {
        let data = create_test_data();
        let reader = ContainerReader::from_slice(&data).unwrap();

        let stats = reader.stats();
        assert_eq!(stats.header_size, crate::format::BDC_HEADER_SIZE);
        assert!(stats.total_size > stats.header_size);
        assert!(stats.compression_ratio() > 0.0);
    }

    #[test]
    fn test_invalid_data() {
        let data = vec![0; 16]; // Too small
        let reader = ContainerReader::from_slice(&data);
        assert!(reader.is_err());
    }
}
