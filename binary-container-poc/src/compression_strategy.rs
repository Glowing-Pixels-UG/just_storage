// SPDX-License-Identifier: MIT
//! Smart compression strategy based on file type detection
//!
//! This module provides intelligent compression decisions based on:
//! - File type detection (magic numbers, MIME types)
//! - Compression characteristics (already compressed, compressible, etc.)
//! - Performance considerations

use crate::format::ComponentType;

/// File type categories for compression decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileTypeCategory {
    /// Already compressed formats (PDF, JPEG, MP3, ZIP, etc.)
    AlreadyCompressed,
    /// Highly compressible formats (text, JSON, XML, etc.)
    HighlyCompressible,
    /// Moderately compressible formats (raw images, some binaries)
    ModeratelyCompressible,
    /// Poorly compressible or random data
    PoorlyCompressible,
    /// Unknown file type
    Unknown,
}

/// Compression recommendation for a file type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionRecommendation {
    /// Skip compression (already compressed or expands)
    Skip,
    /// Compress with fast algorithm (LZ4, zstd-1)
    Fast,
    /// Compress with balanced algorithm (zstd-3)
    Balanced,
    /// Compress with best ratio algorithm (zstd-6, zstd-9)
    BestRatio,
}

/// Compression configuration
#[derive(Debug, Clone, Copy)]
pub struct CompressionConfig {
    /// Whether to use smart detection
    pub smart_detection: bool,
}

impl CompressionConfig {
    /// Create a new smart compression config (recommended)
    pub fn smart() -> Self {
        Self {
            smart_detection: true,
        }
    }

    /// Create a legacy config that always compresses based on component type
    pub fn legacy() -> Self {
        Self {
            smart_detection: false,
        }
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self::smart()
    }
}

/// Compression engine that handles compression decisions
#[derive(Debug)]
pub struct CompressionEngine {
    config: CompressionConfig,
}

impl CompressionEngine {
    /// Create a new compression engine with the given config
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    /// Check if smart detection is enabled
    pub fn is_smart(&self) -> bool {
        self.config.smart_detection
    }

    /// Determine if a component should be compressed
    pub fn should_compress(
        &self,
        component_type: ComponentType,
        data: &[u8],
        mime_type: Option<&str>,
    ) -> bool {
        if !self.config.smart_detection {
            // Legacy behavior: compress based on component type only
            return match component_type {
                ComponentType::Metadata => true,
                ComponentType::Text => true,
                ComponentType::Asset => false, // Assets often already compressed
                ComponentType::Embeddings => false, // Embeddings may be random
            };
        }

        // Smart detection based on file type and component
        match component_type {
            ComponentType::Metadata => {
                // Metadata is always JSON/text - always compress
                true
            }
            ComponentType::Text => {
                // Text is always compressible
                true
            }
            ComponentType::Asset => {
                // Assets: check file type
                self.should_compress_asset(data, mime_type)
            }
            ComponentType::Embeddings => {
                // Embeddings: check if sparse/compressible
                self.should_compress_embeddings(data)
            }
        }
    }

    /// Determine if an asset should be compressed
    fn should_compress_asset(&self, data: &[u8], mime_type: Option<&str>) -> bool {
        // Use file type detection
        let file_category = self.detect_file_category(data, mime_type);

        match file_category {
            FileTypeCategory::AlreadyCompressed => false,
            FileTypeCategory::HighlyCompressible => true,
            FileTypeCategory::ModeratelyCompressible => true,
            FileTypeCategory::PoorlyCompressible => false,
            FileTypeCategory::Unknown => {
                // For unknown types, check if it looks compressible
                self.looks_compressible(data)
            }
        }
    }

    /// Determine if embeddings should be compressed
    fn should_compress_embeddings(&self, data: &[u8]) -> bool {
        // Check if embeddings are sparse (many zeros)
        // Sparse embeddings compress very well
        if data.len() < 1024 {
            // Small embeddings - always compress
            return true;
        }

        // Sample first 1KB to check sparsity
        let sample_size = data.len().min(1024);
        let zeros = data[..sample_size].iter().filter(|&&b| b == 0).count();
        let sparsity_ratio = zeros as f64 / sample_size as f64;

        // If >80% zeros, compress (sparse embeddings)
        if sparsity_ratio > 0.8 {
            return true;
        }

        // Check entropy (simple heuristic)
        // High entropy = random data = don't compress
        let entropy = self.calculate_entropy(&data[..sample_size]);

        // Low entropy = compressible
        entropy < 7.0
    }

    /// Detect file category from data and optional MIME type
    fn detect_file_category(&self, data: &[u8], mime_type: Option<&str>) -> FileTypeCategory {
        // First, try MIME type if provided
        if let Some(mime) = mime_type {
            if let Some(category) = self.category_from_mime(mime) {
                return category;
            }
        }

        // Then, try magic number detection
        #[cfg(feature = "file-type-detection")]
        {
            if let Some(category) = self.detect_from_magic_numbers(data) {
                return category;
            }
        }

        // Fallback: analyze data characteristics
        if self.looks_compressible(data) {
            FileTypeCategory::HighlyCompressible
        } else {
            FileTypeCategory::Unknown
        }
    }

    /// Get category from MIME type
    ///
    /// Comprehensive MIME type detection based on IANA media types and common file formats.
    /// Categorizes files by their compression characteristics.
    fn category_from_mime(&self, mime: &str) -> Option<FileTypeCategory> {
        let mime_lower = mime.to_lowercase();

        // ============================================================================
        // ALREADY COMPRESSED FORMATS (Skip compression)
        // ============================================================================

        // Compressed image formats
        if mime_lower == "image/jpeg"
            || mime_lower == "image/jpg"
            || mime_lower.starts_with("image/jpeg")
            || mime_lower.starts_with("image/jpg")
            || mime_lower == "image/png"
            || mime_lower.starts_with("image/png")
            || mime_lower == "image/gif"
            || mime_lower.starts_with("image/gif")
            || mime_lower == "image/webp"
            || mime_lower.starts_with("image/webp")
            || mime_lower == "image/heif"
            || mime_lower.starts_with("image/heif")
            || mime_lower == "image/heic"
            || mime_lower.starts_with("image/heic")
            || mime_lower == "image/avif"
            || mime_lower.starts_with("image/avif")
            || mime_lower == "image/jxl"
            || mime_lower.starts_with("image/jxl")
            || mime_lower == "image/jp2"
            || mime_lower.starts_with("image/jp2")
            || mime_lower == "image/jpx"
            || mime_lower.starts_with("image/jpx")
            || mime_lower == "image/vnd.adobe.photoshop"
            || mime_lower == "image/psd"
        {
            return Some(FileTypeCategory::AlreadyCompressed);
        }

        // Compressed archives
        if mime_lower == "application/zip"
            || mime_lower.starts_with("application/zip")
            || mime_lower == "application/x-zip-compressed"
            || mime_lower == "application/gzip"
            || mime_lower == "application/x-gzip"
            || mime_lower.starts_with("application/x-gzip")
            || mime_lower == "application/x-bzip2"
            || mime_lower.starts_with("application/x-bzip2")
            || mime_lower == "application/x-7z-compressed"
            || mime_lower.starts_with("application/x-7z-compressed")
            || mime_lower == "application/x-xz"
            || mime_lower.starts_with("application/x-xz")
            || mime_lower == "application/zstd"
            || mime_lower.starts_with("application/zstd")
            || mime_lower == "application/x-lz4"
            || mime_lower.starts_with("application/x-lz4")
            || mime_lower == "application/x-lzip"
            || mime_lower.starts_with("application/x-lzip")
            || mime_lower == "application/vnd.rar"
            || mime_lower == "application/x-rar-compressed"
            || mime_lower.starts_with("application/x-rar")
            || mime_lower == "application/x-tar"
            || mime_lower.starts_with("application/x-tar")
            || mime_lower == "application/x-compress"
            || mime_lower.starts_with("application/x-compress")
            || mime_lower == "application/x-bzip"
            || mime_lower == "application/x-bzip3"
        {
            return Some(FileTypeCategory::AlreadyCompressed);
        }

        // Office Open XML formats (ZIP-based, already compressed)
        if mime_lower == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            || mime_lower == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            || mime_lower
                == "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            || mime_lower.starts_with("application/vnd.openxmlformats")
            || mime_lower == "application/vnd.ms-excel.sheet.macroenabled.12"
            || mime_lower == "application/vnd.ms-powerpoint.presentation.macroenabled.12"
            || mime_lower == "application/vnd.ms-word.document.macroenabled.12"
        {
            return Some(FileTypeCategory::AlreadyCompressed);
        }

        // PDF (usually compressed internally)
        if mime_lower == "application/pdf" || mime_lower.starts_with("application/pdf") {
            return Some(FileTypeCategory::AlreadyCompressed);
        }

        // EPUB (ZIP-based)
        if mime_lower == "application/epub+zip" || mime_lower.starts_with("application/epub") {
            return Some(FileTypeCategory::AlreadyCompressed);
        }

        // Compressed audio formats
        if mime_lower == "audio/mpeg"
            || mime_lower.starts_with("audio/mpeg")
            || mime_lower == "audio/mp3"
            || mime_lower == "audio/mp4"
            || mime_lower.starts_with("audio/mp4")
            || mime_lower == "audio/x-m4a"
            || mime_lower == "audio/aac"
            || mime_lower.starts_with("audio/aac")
            || mime_lower == "audio/ogg"
            || mime_lower.starts_with("audio/ogg")
            || mime_lower == "audio/vorbis"
            || mime_lower == "audio/opus"
            || mime_lower.starts_with("audio/opus")
            || mime_lower == "audio/x-flac"
            || mime_lower == "audio/flac"
            || mime_lower.starts_with("audio/flac")
            || mime_lower == "audio/amr"
            || mime_lower.starts_with("audio/amr")
            || mime_lower == "audio/amr-wb"
            || mime_lower.starts_with("audio/amr-wb")
            || mime_lower == "audio/x-ape"
            || mime_lower == "audio/x-dsf"
        {
            return Some(FileTypeCategory::AlreadyCompressed);
        }

        // Compressed video formats
        if mime_lower == "video/mp4"
            || mime_lower.starts_with("video/mp4")
            || mime_lower == "video/x-m4v"
            || mime_lower.starts_with("video/x-m4v")
            || mime_lower == "video/quicktime"
            || mime_lower.starts_with("video/quicktime")
            || mime_lower == "video/x-matroska"
            || mime_lower.starts_with("video/x-matroska")
            || mime_lower == "video/mkv"
            || mime_lower == "video/webm"
            || mime_lower.starts_with("video/webm")
            || mime_lower == "video/x-msvideo"
            || mime_lower == "video/avi"
            || mime_lower == "video/x-ms-wmv"
            || mime_lower.starts_with("video/x-ms-wmv")
            || mime_lower == "video/mpeg"
            || mime_lower.starts_with("video/mpeg")
            || mime_lower == "video/mpg"
            || mime_lower == "video/x-flv"
            || mime_lower.starts_with("video/x-flv")
            || mime_lower == "video/x-ms-asf"
            || mime_lower.starts_with("video/x-ms-asf")
            || mime_lower == "video/3gpp"
            || mime_lower.starts_with("video/3gpp")
            || mime_lower == "video/3gpp2"
            || mime_lower.starts_with("video/3gpp2")
        {
            return Some(FileTypeCategory::AlreadyCompressed);
        }

        // Font files (often compressed)
        if mime_lower == "application/font-woff"
            || mime_lower == "application/font-woff2"
            || mime_lower == "font/woff"
            || mime_lower == "font/woff2"
            || mime_lower == "application/font-sfnt"
            || mime_lower.starts_with("application/font-sfnt")
            || mime_lower == "application/x-font-ttf"
            || mime_lower.starts_with("application/x-font-ttf")
            || mime_lower == "application/x-font-otf"
            || mime_lower.starts_with("application/x-font-otf")
        {
            return Some(FileTypeCategory::AlreadyCompressed);
        }

        // ============================================================================
        // HIGHLY COMPRESSIBLE FORMATS (Always compress)
        // ============================================================================

        // All text-based formats
        if mime_lower.starts_with("text/")
            || mime_lower == "text/plain"
            || mime_lower == "text/html"
            || mime_lower == "text/xml"
            || mime_lower == "text/css"
            || mime_lower == "text/javascript"
            || mime_lower == "text/csv"
            || mime_lower == "text/markdown"
            || mime_lower == "text/yaml"
            || mime_lower == "text/x-yaml"
            || mime_lower == "text/x-script"
            || mime_lower == "text/x-log"
        {
            return Some(FileTypeCategory::HighlyCompressible);
        }

        // Structured data formats (text-based)
        if mime_lower == "application/json"
            || mime_lower == "application/xml"
            || mime_lower == "text/xml"
            || mime_lower == "application/xml-dtd"
            || mime_lower == "application/javascript"
            || mime_lower == "application/ecmascript"
            || mime_lower == "text/javascript"
            || mime_lower == "text/ecmascript"
            || mime_lower == "application/x-yaml"
            || mime_lower == "application/yaml"
            || mime_lower == "text/yaml"
            || mime_lower == "text/x-yaml"
            || mime_lower == "application/toml"
            || mime_lower == "text/toml"
            || mime_lower == "application/x-toml"
        {
            return Some(FileTypeCategory::HighlyCompressible);
        }

        // Source code files
        if mime_lower == "text/x-c"
            || mime_lower == "text/x-c++"
            || mime_lower == "text/x-c++src"
            || mime_lower == "text/x-csrc"
            || mime_lower == "text/x-java"
            || mime_lower == "text/x-java-source"
            || mime_lower == "text/x-python"
            || mime_lower == "text/x-rust"
            || mime_lower == "text/x-go"
            || mime_lower == "text/x-ruby"
            || mime_lower == "text/x-php"
            || mime_lower == "text/x-shellscript"
            || mime_lower == "application/x-sh"
            || mime_lower == "application/x-bash"
            || mime_lower == "text/x-perl"
            || mime_lower == "text/x-lua"
        {
            return Some(FileTypeCategory::HighlyCompressible);
        }

        // Markup and configuration
        if mime_lower == "application/xhtml+xml"
            || mime_lower == "application/sgml"
            || mime_lower == "text/sgml"
            || mime_lower == "application/x-ini"
            || mime_lower == "text/x-ini"
            || mime_lower == "application/x-config"
            || mime_lower == "text/x-properties"
        {
            return Some(FileTypeCategory::HighlyCompressible);
        }

        // Data interchange formats
        if mime_lower == "application/csv"
            || mime_lower == "text/csv"
            || mime_lower == "application/x-csv"
            || mime_lower == "text/tab-separated-values"
            || mime_lower == "text/tsv"
        {
            return Some(FileTypeCategory::HighlyCompressible);
        }

        // ============================================================================
        // MODERATELY COMPRESSIBLE FORMATS (Compress with fast algorithm)
        // ============================================================================

        // Uncompressed image formats
        if mime_lower == "image/bmp"
            || mime_lower == "image/x-ms-bmp"
            || mime_lower == "image/x-bmp"
            || mime_lower == "image/tiff"
            || mime_lower.starts_with("image/tiff")
            || mime_lower == "image/tif"
            || mime_lower == "image/x-tiff"
            || mime_lower.starts_with("image/x-tiff")
            || mime_lower == "image/x-portable-bitmap"
            || mime_lower == "image/x-portable-pixmap"
            || mime_lower == "image/x-portable-graymap"
            || mime_lower == "image/x-portable-anymap"
            || mime_lower == "image/x-pcx"
            || mime_lower == "image/x-pict"
            || mime_lower == "image/x-tga"
            || mime_lower == "image/x-targa"
            || mime_lower == "image/x-icon"
            || mime_lower == "image/vnd.microsoft.icon"
        {
            return Some(FileTypeCategory::ModeratelyCompressible);
        }

        // Uncompressed audio formats
        if mime_lower == "audio/wav"
            || mime_lower == "audio/x-wav"
            || mime_lower == "audio/wave"
            || mime_lower == "audio/vnd.wave"
            || mime_lower == "audio/x-pn-wav"
            || mime_lower == "audio/x-aiff"
            || mime_lower == "audio/aiff"
        {
            return Some(FileTypeCategory::ModeratelyCompressible);
        }

        // Office documents (older formats, may contain uncompressed data)
        if mime_lower == "application/msword"
            || mime_lower == "application/vnd.ms-word"
            || mime_lower == "application/vnd.ms-excel"
            || mime_lower == "application/vnd.ms-powerpoint"
            || mime_lower == "application/vnd.oasis.opendocument.text"
            || mime_lower == "application/vnd.oasis.opendocument.spreadsheet"
            || mime_lower == "application/vnd.oasis.opendocument.presentation"
            || mime_lower == "application/vnd.oasis.opendocument.graphics"
            || mime_lower == "application/rtf"
            || mime_lower == "text/rtf"
            || mime_lower == "application/x-rtf"
        {
            return Some(FileTypeCategory::ModeratelyCompressible);
        }

        // Database files (may contain compressible data)
        if mime_lower == "application/x-sqlite3"
            || mime_lower == "application/vnd.sqlite3"
            || mime_lower == "application/x-sql"
            || mime_lower == "application/x-access"
            || mime_lower == "application/x-msaccess"
        {
            return Some(FileTypeCategory::ModeratelyCompressible);
        }

        // ============================================================================
        // POORLY COMPRESSIBLE FORMATS (Skip compression)
        // ============================================================================

        // Encrypted files (appear random)
        if mime_lower == "application/pgp-encrypted"
            || mime_lower == "application/pgp-keys"
            || mime_lower == "application/pgp-signature"
            || mime_lower.starts_with("application/x-encrypted")
        {
            return Some(FileTypeCategory::PoorlyCompressible);
        }

        // Executable files (often compressed or encrypted)
        if mime_lower == "application/x-executable"
            || mime_lower == "application/x-elf"
            || mime_lower == "application/x-mach-binary"
            || mime_lower == "application/vnd.microsoft.portable-executable"
            || mime_lower == "application/x-msdownload"
            || mime_lower == "application/x-dosexec"
        {
            return Some(FileTypeCategory::PoorlyCompressible);
        }

        // Virtual machine disk images (often sparse but large)
        if mime_lower == "application/x-vhd"
            || mime_lower == "application/x-vmdk"
            || mime_lower == "application/x-vdi"
            || mime_lower == "application/x-qcow2"
        {
            return Some(FileTypeCategory::PoorlyCompressible);
        }

        // Binary data formats (will be analyzed by data characteristics)
        if mime_lower == "application/octet-stream" || mime_lower == "application/x-binary" {
            // Will be analyzed by data characteristics
            return None;
        }

        None
    }

    /// Detect file type from magic numbers
    #[cfg(feature = "file-type-detection")]
    fn detect_from_magic_numbers(&self, data: &[u8]) -> Option<FileTypeCategory> {
        use infer;

        if let Some(kind) = infer::get(data) {
            let mime = kind.mime_type();
            return self.category_from_mime(mime);
        }

        None
    }

    /// Check if data looks compressible based on simple heuristics
    fn looks_compressible(&self, data: &[u8]) -> bool {
        if data.is_empty() {
            return false;
        }

        // Check for high repetition (compressible)
        let sample_size = data.len().min(4096);
        let sample = &data[..sample_size];

        // Count unique bytes
        let mut byte_counts = [0u32; 256];
        for &byte in sample {
            byte_counts[byte as usize] += 1;
        }

        // If most bytes are the same, it's compressible
        let max_count = byte_counts.iter().max().copied().unwrap_or(0);
        let repetition_ratio = max_count as f64 / sample_size as f64;

        if repetition_ratio > 0.5 {
            return true; // High repetition = compressible
        }

        // Check entropy
        let entropy = self.calculate_entropy(sample);
        entropy < 7.0 // Low entropy = compressible
    }

    /// Calculate simple entropy estimate
    fn calculate_entropy(&self, data: &[u8]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }

        let mut counts = [0u32; 256];
        for &byte in data {
            counts[byte as usize] += 1;
        }

        let len = data.len() as f64;
        let mut entropy = 0.0;

        for &count in &counts {
            if count > 0 {
                let probability = count as f64 / len;
                entropy -= probability * probability.log2();
            }
        }

        entropy
    }

    /// Get compression recommendation for a file type
    pub fn get_recommendation(
        &self,
        component_type: ComponentType,
        data: &[u8],
        mime_type: Option<&str>,
    ) -> CompressionRecommendation {
        if !self.should_compress(component_type, data, mime_type) {
            return CompressionRecommendation::Skip;
        }

        match component_type {
            ComponentType::Metadata | ComponentType::Text => {
                // Text-based: use balanced compression
                CompressionRecommendation::Balanced
            }
            ComponentType::Asset => {
                let category = self.detect_file_category(data, mime_type);
                match category {
                    FileTypeCategory::HighlyCompressible => CompressionRecommendation::Balanced,
                    FileTypeCategory::ModeratelyCompressible => CompressionRecommendation::Fast,
                    _ => CompressionRecommendation::Skip,
                }
            }
            ComponentType::Embeddings => {
                // Embeddings: use fast compression (they're usually sparse)
                CompressionRecommendation::Fast
            }
        }
    }
}

impl Default for CompressionEngine {
    fn default() -> Self {
        Self::new(CompressionConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_detection() {
        let strategy = CompressionStrategy::new(CompressionConfig::smart());
        let pdf_data = b"%PDF-1.4\n";
        assert!(!strategy.should_compress_asset(pdf_data, Some("application/pdf")));
    }

    #[test]
    fn test_text_detection() {
        let strategy = CompressionStrategy::new(CompressionConfig::smart());
        let text_data = b"Hello, this is some text that should compress well!";
        assert!(strategy.should_compress_asset(text_data, Some("text/plain")));
    }

    #[test]
    fn test_sparse_embeddings() {
        let strategy = CompressionStrategy::new(CompressionConfig::smart());
        let mut sparse_data = vec![0u8; 2048];
        sparse_data[0] = 1;
        sparse_data[100] = 2;
        assert!(strategy.should_compress_embeddings(&sparse_data));
    }

    #[test]
    fn test_random_embeddings() {
        let strategy = CompressionStrategy::new(CompressionConfig::smart());
        let random_data: Vec<u8> = (0..2048).map(|i| (i * 7) as u8).collect();
        // Random data should not compress well
        assert!(!strategy.should_compress_embeddings(&random_data));
    }
}

/// Backward compatibility alias
pub type CompressionStrategy = CompressionEngine;
