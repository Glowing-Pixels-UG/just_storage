// SPDX-License-Identifier: MIT
//! Binary container core types and structures

use crate::format::{BdcHeader, ComponentType};
use document_bundler::BundleMetadata;

/// A binary document container with zero-copy access
#[derive(Debug)]
pub struct BinaryContainer {
    /// Container header (parsed once)
    pub header: BdcHeader,

    /// Container data (owned or borrowed)
    pub data: Vec<u8>,
}

impl BinaryContainer {
    /// Create a new empty container
    pub fn new() -> Self {
        Self {
            header: BdcHeader::new(),
            data: Vec::new(),
        }
    }

    /// Create from owned data (takes ownership)
    pub fn from_vec(data: Vec<u8>) -> Result<Self, String> {
        if data.len() < crate::format::BDC_HEADER_SIZE {
            return Err("Container too small".to_string());
        }

        // Parse header from the beginning
        let header_data = &data[..crate::format::BDC_HEADER_SIZE];
        let header = BdcHeader::from_bytes(header_data)?;
        header.validate()?;

        Ok(Self { header, data })
    }

    /// Create from borrowed data (copies - necessary for owned container)
    ///
    /// For zero-copy access, use `ContainerReader::from_slice` instead
    #[inline]
    pub fn from_slice(data: &[u8]) -> Result<Self, String> {
        Self::from_vec(data.to_vec())
    }

    /// Get the total size of the container
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Get component data by type (zero-copy)
    #[inline]
    pub fn get_component(&self, component_type: ComponentType) -> Option<&[u8]> {
        let offset = self.header.component_offset(component_type) as usize;
        let size = self.header.component_size(component_type) as usize;
        let end = offset.checked_add(size)?;

        // Bounds check (compiler optimizes this well)
        if end > self.data.len() {
            return None;
        }

        Some(&self.data[offset..end])
    }

    /// Get component data without bounds checking (unsafe, but faster)
    ///
    /// # Safety
    /// Caller must ensure offset + size <= self.data.len()
    #[inline]
    pub unsafe fn get_component_unchecked(&self, component_type: ComponentType) -> &[u8] {
        let offset = self.header.component_offset(component_type) as usize;
        let size = self.header.component_size(component_type) as usize;
        self.data.get_unchecked(offset..offset + size)
    }

    /// Check if the container is valid (only checks bounds, header already validated)
    pub fn validate(&self) -> Result<(), String> {
        // Header already validated during construction
        // Just check bounds
        for &component_type in ComponentType::all() {
            let offset = self.header.component_offset(component_type) as usize;
            let size = self.header.component_size(component_type) as usize;

            if offset + size > self.data.len() {
                return Err(format!(
                    "Component {} extends beyond container bounds",
                    component_type.name()
                ));
            }
        }

        Ok(())
    }

    /// Get header reference
    pub fn header(&self) -> &BdcHeader {
        &self.header
    }
}

impl Default for BinaryContainer {
    fn default() -> Self {
        Self::new()
    }
}

/// Uniform access interface similar to JSON lines
///
/// This allows accessing container components like JSON fields:
/// - `container.metadata()` instead of `container.get(ComponentType::Metadata)`
/// - `container.asset()` instead of `container.get(ComponentType::Asset)`
/// - etc.
impl BinaryContainer {
    /// Get metadata component (zero-copy)
    pub fn metadata(&self) -> Option<&[u8]> {
        self.get_component(ComponentType::Metadata)
    }

    /// Get asset component (zero-copy)
    pub fn asset(&self) -> Option<&[u8]> {
        self.get_component(ComponentType::Asset)
    }

    /// Get text component (zero-copy)
    pub fn text(&self) -> Option<&[u8]> {
        self.get_component(ComponentType::Text)
    }

    /// Get embeddings component (zero-copy)
    pub fn embeddings(&self) -> Option<&[u8]> {
        self.get_component(ComponentType::Embeddings)
    }
}

/// Create containers from document bundler types
impl BinaryContainer {
    /// Create from document bundler metadata
    pub fn from_bundle_metadata(_metadata: &BundleMetadata) -> Result<Self, serde_json::Error> {
        // This is just for interface compatibility - would need a writer
        Ok(Self::new())
    }

    /// Create from file path (reads entire file)
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, String> {
        let data = std::fs::read(&path).map_err(|e| format!("Failed to read file: {}", e))?;
        Self::from_vec(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_new() {
        let container = BinaryContainer::new();
        assert_eq!(container.size(), 0);
        assert!(container.metadata().is_none());
    }

    #[test]
    fn test_container_validate_empty() {
        let container = BinaryContainer::new();
        assert!(container.validate().is_err());
    }

    #[test]
    fn test_uniform_access_methods() {
        let container = BinaryContainer::new();

        // These should all return None for empty container
        assert!(container.metadata().is_none());
        assert!(container.asset().is_none());
        assert!(container.text().is_none());
        assert!(container.embeddings().is_none());
    }
}
