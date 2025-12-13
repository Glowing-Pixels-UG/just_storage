// SPDX-License-Identifier: MIT
//! Binary Document Container (BDC) format specification
//!
//! Defines the binary format for high-performance document bundling.

use std::io::{Read, Write};

/// BDC format magic bytes
pub const BDC_MAGIC: &[u8; 8] = &[66, 68, 67, 1, 0, 0, 0, 0]; // "BDC\x01\x00\x00\x00"

/// BDC format version
pub const BDC_VERSION: u32 = 1;

/// Header size in bytes
pub const BDC_HEADER_SIZE: usize = 32;

/// Compression flags
pub mod flags {
    /// No compression
    pub const NONE: u32 = 0x00000000;

    /// Compress metadata
    pub const COMPRESS_METADATA: u32 = 0x00000001;

    /// Compress asset
    pub const COMPRESS_ASSET: u32 = 0x00000002;

    /// Compress text
    pub const COMPRESS_TEXT: u32 = 0x00000004;

    /// Compress embeddings
    pub const COMPRESS_EMBEDDINGS: u32 = 0x00000008;

    /// Compress all components
    pub const COMPRESS_ALL: u32 =
        COMPRESS_METADATA | COMPRESS_ASSET | COMPRESS_TEXT | COMPRESS_EMBEDDINGS;
}

/// BDC file header (32 bytes) - optimized for performance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)] // Remove packed for better alignment/performance
pub struct BdcHeader {
    /// Magic bytes: "BDC\x01\x00\x00\x00"
    pub magic: [u8; 8],

    /// Format version (currently 1)
    pub version: u32,

    /// Compression flags
    pub flags: u32,

    /// Component sizes (compressed)
    pub metadata_size: u32,
    pub asset_size: u32,
    pub text_size: u32,
    pub embeddings_size: u32,
}

impl BdcHeader {
    /// Create a new header with default values
    ///
    /// Default compression: Compress all components for best size/read performance.
    /// Use `with_compression_flags()` to customize for faster writes.
    pub fn new() -> Self {
        Self {
            magic: *BDC_MAGIC,
            version: BDC_VERSION,
            flags: flags::COMPRESS_ALL,
            metadata_size: 0,
            asset_size: 0,
            text_size: 0,
            embeddings_size: 0,
        }
    }

    /// Create header from raw bytes (zero-copy)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != BDC_HEADER_SIZE {
            return Err(format!(
                "Header must be {} bytes, got {}",
                BDC_HEADER_SIZE,
                bytes.len()
            ));
        }

        // Read fields directly from bytes (little-endian)
        let magic = bytes[0..8].try_into().unwrap();
        let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let flags = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        let metadata_size = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        let asset_size = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
        let text_size = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
        let embeddings_size = u32::from_le_bytes(bytes[28..32].try_into().unwrap());

        Ok(Self {
            magic,
            version,
            flags,
            metadata_size,
            asset_size,
            text_size,
            embeddings_size,
        })
    }

    /// Validate the header (fast path)
    pub fn validate(&self) -> Result<(), String> {
        // Fast checks first (avoid expensive formatting)
        if self.magic != *BDC_MAGIC {
            return Err("Invalid magic bytes".to_string());
        }

        if self.version != BDC_VERSION {
            return Err(format!(
                "Unsupported version: expected {}, got {}",
                BDC_VERSION, self.version
            ));
        }

        Ok(())
    }

    /// Validate header with detailed error messages
    pub fn validate_detailed(&self) -> Result<(), String> {
        if self.magic != *BDC_MAGIC {
            return Err(format!(
                "Invalid magic bytes: expected {:?}, got {:?}",
                *BDC_MAGIC, self.magic
            ));
        }

        if self.version != BDC_VERSION {
            return Err(format!(
                "Unsupported version: expected {}, got {}",
                BDC_VERSION, self.version
            ));
        }

        Ok(())
    }

    /// Check if a component should be compressed
    #[inline]
    pub fn should_compress(&self, component: ComponentType) -> bool {
        let flag = match component {
            ComponentType::Metadata => flags::COMPRESS_METADATA,
            ComponentType::Asset => flags::COMPRESS_ASSET,
            ComponentType::Text => flags::COMPRESS_TEXT,
            ComponentType::Embeddings => flags::COMPRESS_EMBEDDINGS,
        };
        (self.flags & flag) != 0
    }

    /// Get the size of a component
    #[inline]
    pub fn component_size(&self, component: ComponentType) -> u32 {
        match component {
            ComponentType::Metadata => self.metadata_size,
            ComponentType::Asset => self.asset_size,
            ComponentType::Text => self.text_size,
            ComponentType::Embeddings => self.embeddings_size,
        }
    }

    /// Set the size of a component
    pub fn set_component_size(&mut self, component: ComponentType, size: u32) {
        match component {
            ComponentType::Metadata => self.metadata_size = size,
            ComponentType::Asset => self.asset_size = size,
            ComponentType::Text => self.text_size = size,
            ComponentType::Embeddings => self.embeddings_size = size,
        }
    }

    /// Calculate the offset of a component from the start of the file
    #[inline]
    pub fn component_offset(&self, component: ComponentType) -> u64 {
        let base = BDC_HEADER_SIZE as u64;
        match component {
            ComponentType::Metadata => base,
            ComponentType::Asset => base + self.metadata_size as u64,
            ComponentType::Text => base + self.metadata_size as u64 + self.asset_size as u64,
            ComponentType::Embeddings => {
                base + self.metadata_size as u64 + self.asset_size as u64 + self.text_size as u64
            }
        }
    }

    /// Read header from a reader
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self, std::io::Error> {
        let mut header = [0u8; BDC_HEADER_SIZE];
        reader.read_exact(&mut header)?;
        Self::from_bytes(&header)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Write header directly to buffer (zero-copy, optimized)
    #[inline]
    pub fn write_to_buffer(&self, buffer: &mut Vec<u8>) {
        // Reserve space upfront to avoid reallocation
        buffer.reserve(BDC_HEADER_SIZE);

        // Write all fields in one go (compiler optimizes this well)
        buffer.extend_from_slice(&self.magic);
        buffer.extend_from_slice(&self.version.to_le_bytes());
        buffer.extend_from_slice(&self.flags.to_le_bytes());
        buffer.extend_from_slice(&self.metadata_size.to_le_bytes());
        buffer.extend_from_slice(&self.asset_size.to_le_bytes());
        buffer.extend_from_slice(&self.text_size.to_le_bytes());
        buffer.extend_from_slice(&self.embeddings_size.to_le_bytes());
    }

    /// Write header to a writer
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        writer.write_all(&self.magic)?;
        writer.write_all(&self.version.to_le_bytes())?;
        writer.write_all(&self.flags.to_le_bytes())?;
        writer.write_all(&self.metadata_size.to_le_bytes())?;
        writer.write_all(&self.asset_size.to_le_bytes())?;
        writer.write_all(&self.text_size.to_le_bytes())?;
        writer.write_all(&self.embeddings_size.to_le_bytes())?;

        Ok(())
    }

    /// Convert to bytes for writing (legacy method)
    pub fn to_bytes(&self) -> [u8; BDC_HEADER_SIZE] {
        let mut bytes = [0u8; BDC_HEADER_SIZE];

        bytes[0..8].copy_from_slice(&self.magic);
        bytes[8..12].copy_from_slice(&self.version.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.flags.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.metadata_size.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.asset_size.to_le_bytes());
        bytes[24..28].copy_from_slice(&self.text_size.to_le_bytes());
        bytes[28..32].copy_from_slice(&self.embeddings_size.to_le_bytes());

        bytes
    }
}

impl Default for BdcHeader {
    fn default() -> Self {
        Self::new()
    }
}

/// Component types in the container
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentType {
    /// Document metadata (JSON)
    Metadata,

    /// Original document asset (PDF, image, etc.)
    Asset,

    /// Extracted text content
    Text,

    /// Vector embeddings
    Embeddings,
}

impl ComponentType {
    /// Get all component types in order
    pub fn all() -> &'static [ComponentType] {
        &[
            ComponentType::Metadata,
            ComponentType::Asset,
            ComponentType::Text,
            ComponentType::Embeddings,
        ]
    }

    /// Get the name of the component
    pub fn name(&self) -> &'static str {
        match self {
            ComponentType::Metadata => "metadata",
            ComponentType::Asset => "asset",
            ComponentType::Text => "text",
            ComponentType::Embeddings => "embeddings",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_new() {
        let header = BdcHeader::new();
        // Test that header is valid and has expected defaults
        assert!(header.validate().is_ok());
        assert_eq!(header.component_size(ComponentType::Metadata), 0);
        assert_eq!(header.component_size(ComponentType::Asset), 0);
        assert_eq!(header.should_compress(ComponentType::Metadata), true);
    }

    #[test]
    fn test_header_validate_valid() {
        let header = BdcHeader::new();
        assert!(header.validate().is_ok());
    }

    #[test]
    fn test_header_validate_invalid_magic() {
        let mut header = BdcHeader::new();
        header.magic = [0; 8];
        assert!(header.validate().is_err());
    }

    #[test]
    fn test_header_validate_invalid_version() {
        let mut header = BdcHeader::new();
        header.version = 999;
        assert!(header.validate().is_err());
    }

    #[test]
    fn test_should_compress() {
        let mut header = BdcHeader::new();
        header.flags = flags::COMPRESS_METADATA | flags::COMPRESS_TEXT;

        assert!(header.should_compress(ComponentType::Metadata));
        assert!(!header.should_compress(ComponentType::Asset));
        assert!(header.should_compress(ComponentType::Text));
        assert!(!header.should_compress(ComponentType::Embeddings));
    }

    #[test]
    fn test_component_offset() {
        let mut header = BdcHeader::new();
        header.metadata_size = 100;
        header.asset_size = 200;
        header.text_size = 300;
        header.embeddings_size = 400;

        assert_eq!(
            header.component_offset(ComponentType::Metadata),
            BDC_HEADER_SIZE as u64
        );
        assert_eq!(
            header.component_offset(ComponentType::Asset),
            BDC_HEADER_SIZE as u64 + 100
        );
        assert_eq!(
            header.component_offset(ComponentType::Text),
            BDC_HEADER_SIZE as u64 + 300
        );
        assert_eq!(
            header.component_offset(ComponentType::Embeddings),
            BDC_HEADER_SIZE as u64 + 600
        );
    }

    #[test]
    fn test_component_size_and_set() {
        let mut header = BdcHeader::new();

        header.set_component_size(ComponentType::Metadata, 123);
        assert_eq!(header.component_size(ComponentType::Metadata), 123);

        header.set_component_size(ComponentType::Asset, 456);
        assert_eq!(header.component_size(ComponentType::Asset), 456);
    }

    #[test]
    fn test_component_type_name() {
        assert_eq!(ComponentType::Metadata.name(), "metadata");
        assert_eq!(ComponentType::Asset.name(), "asset");
        assert_eq!(ComponentType::Text.name(), "text");
        assert_eq!(ComponentType::Embeddings.name(), "embeddings");
    }

    #[test]
    fn test_component_type_all() {
        let all = ComponentType::all();
        assert_eq!(all.len(), 4);
        assert_eq!(all[0], ComponentType::Metadata);
        assert_eq!(all[1], ComponentType::Asset);
        assert_eq!(all[2], ComponentType::Text);
        assert_eq!(all[3], ComponentType::Embeddings);
    }
}
