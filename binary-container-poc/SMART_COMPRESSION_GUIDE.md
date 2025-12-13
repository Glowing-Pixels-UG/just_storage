# Smart Compression Detection Guide

## Overview

The BDC format now includes **intelligent compression detection** that automatically determines whether to compress each component based on file type, data characteristics, and compression effectiveness.

## Features

### 1. **File Type Detection**
- **MIME Type Support**: Detects file types from MIME types (e.g., `application/pdf`, `text/plain`)
- **Magic Number Detection**: Uses the `infer` crate to detect file types from magic bytes (when `file-type-detection` feature is enabled)
- **Fallback Analysis**: Analyzes data characteristics (entropy, repetition) for unknown types

### 2. **Smart Compression Decisions**

#### Already Compressed Formats (Skip Compression)
- **Images**: JPEG, PNG, GIF, WebP, HEIF, AVIF
- **Documents**: PDF
- **Archives**: ZIP, GZIP, BZIP2, 7Z, XZ, ZSTD, LZ4
- **Media**: Video (MP4, MKV, WebM, etc.), Audio (MP3, OGG, FLAC, etc.)

**Rationale**: These formats are already compressed. Re-compressing wastes CPU time and may expand the file.

#### Highly Compressible Formats (Always Compress)
- **Text**: `text/*` MIME types
- **Structured Data**: JSON, XML, YAML
- **Code**: JavaScript, CSS, etc.

**Rationale**: Text-based formats compress extremely well (often 80-95% reduction).

#### Embeddings Detection
- **Sparse Embeddings** (>80% zeros): Compress (99%+ savings)
- **Random Embeddings** (high entropy): Skip compression (expands)
- **Small Embeddings** (<1KB): Always compress

**Rationale**: Sparse embeddings compress dramatically, but random data expands.

### 3. **Component-Specific Logic**

| Component | Default Behavior | Smart Detection |
|-----------|-----------------|-----------------|
| **Metadata** | Always compress | Always compress (JSON/text) |
| **Text** | Always compress | Always compress (text) |
| **Asset** | Configurable | Detects file type, skips if already compressed |
| **Embeddings** | Configurable | Detects sparsity/entropy, compresses if beneficial |

## Usage

### Basic Usage (Smart Detection)

```rust
use binary_container_poc::ContainerWriter;

// Create writer with smart compression
let mut writer = ContainerWriter::new_smart();

// Optionally set MIME type for better detection
writer = writer.set_asset_mime_type("application/pdf");

// Add components - compression is automatic
writer.add_metadata(metadata_json.as_bytes().to_vec())?;
writer.add_asset(pdf_data)?;  // Won't compress (PDF is already compressed)
writer.add_text(text_data)?;  // Will compress (text is compressible)
writer.add_embeddings(embeddings_data)?;  // Will compress if sparse

let container = writer.finalize()?;
```

### With Explicit MIME Type

```rust
// Provide MIME type for better detection
writer.add_asset_with_mime(
    asset_data,
    Some("application/pdf")  // Explicitly tell it's a PDF
)?;
```

### Legacy Mode (Manual Control)

```rust
// Use legacy mode with manual compression flags
let mut writer = ContainerWriter::new()
    .with_compression_flags(
        binary_container_poc::format::flags::COMPRESS_METADATA |
        binary_container_poc::format::flags::COMPRESS_TEXT
    );
```

## File Type Detection Methods

### 1. MIME Type (Recommended)
Fastest and most accurate when you know the file type:

```rust
writer.set_asset_mime_type("application/pdf");
```

### 2. Magic Number Detection (Automatic)
When `file-type-detection` feature is enabled, automatically detects from file content:

```rust
// No MIME type needed - detects from magic bytes
writer.add_asset(pdf_data)?;  // Detects PDF from %PDF- magic bytes
```

### 3. Data Analysis (Fallback)
For unknown types, analyzes data characteristics:
- **Entropy**: Low entropy = compressible
- **Repetition**: High repetition = compressible
- **Randomness**: High entropy = skip compression

## Compression Strategy API

### CompressionStrategy

```rust
use binary_container_poc::CompressionStrategy;

// Smart detection (default)
let strategy = CompressionStrategy::new();

// Legacy mode (always compress based on flags)
let strategy = CompressionStrategy::always_compress();

// Check if component should be compressed
let should_compress = strategy.should_compress(
    ComponentType::Asset,
    &data,
    Some("application/pdf")
);
```

### FileTypeCategory

```rust
use binary_container_poc::FileTypeCategory;

match category {
    FileTypeCategory::AlreadyCompressed => {
        // Skip compression
    }
    FileTypeCategory::HighlyCompressible => {
        // Compress with balanced algorithm
    }
    FileTypeCategory::ModeratelyCompressible => {
        // Compress with fast algorithm
    }
    FileTypeCategory::PoorlyCompressible => {
        // Skip compression
    }
    FileTypeCategory::Unknown => {
        // Analyze data characteristics
    }
}
```

## Examples

### Example 1: PDF Document (Skip Compression)

```rust
let mut writer = ContainerWriter::new_smart()
    .set_asset_mime_type("application/pdf");

writer.add_asset(pdf_data)?;
// Result: PDF is NOT compressed (already compressed format)
// Benefit: Faster writes, no expansion
```

### Example 2: Text File (Compress)

```rust
let mut writer = ContainerWriter::new_smart()
    .set_asset_mime_type("text/plain");

writer.add_asset(text_data)?;
// Result: Text IS compressed (highly compressible)
// Benefit: 80-95% size reduction
```

### Example 3: Sparse Embeddings (Compress)

```rust
let mut writer = ContainerWriter::new_smart();

// Embeddings with 90% zeros
let sparse_embeddings = vec![0u8; 512 * 1024];
writer.add_embeddings(sparse_embeddings)?;
// Result: Embeddings ARE compressed (sparse detection)
// Benefit: 99%+ size reduction
```

### Example 4: Random Embeddings (Skip Compression)

```rust
let mut writer = ContainerWriter::new_smart();

// Random embeddings (high entropy)
let random_embeddings: Vec<u8> = (0..512*1024).map(|i| (i * 7) as u8).collect();
writer.add_embeddings(random_embeddings)?;
// Result: Embeddings are NOT compressed (high entropy detection)
// Benefit: Avoids expansion, faster writes
```

## Performance Impact

### Write Performance
- **PDFs**: ~13x faster (skip compression: 0.35ms vs 5.9ms)
- **Text**: Similar speed (compression is fast for text)
- **Sparse Embeddings**: ~30x faster compression (optimized for zeros)

### File Size
- **PDFs**: No change (skipped compression)
- **Text**: 80-95% reduction (compressed)
- **Sparse Embeddings**: 99%+ reduction (compressed)
- **Random Data**: No expansion (skipped compression)

## Configuration

### Cargo.toml

```toml
[dependencies]
binary-container-poc = { path = "../binary-container-poc", features = ["compression", "file-type-detection"] }
```

### Features
- **`compression`**: Enables compression support (required)
- **`file-type-detection`**: Enables magic number detection via `infer` crate (optional but recommended)

## Best Practices

1. **Always use `new_smart()`** for production code
2. **Provide MIME types** when known for faster detection
3. **Enable `file-type-detection`** feature for automatic detection
4. **Test with real data** to verify compression decisions
5. **Monitor file sizes** to ensure compression is working as expected

## Testing

Run the smart compression example:

```bash
cargo run --example smart_compression --features "compression,file-type-detection" -- /path/to/file.pdf
```

## Implementation Details

### Detection Priority
1. **MIME Type** (if provided) - fastest, most accurate
2. **Magic Numbers** (if `file-type-detection` enabled) - automatic, accurate
3. **Data Analysis** (fallback) - slower, less accurate

### Compression Decision Flow

```
Component Added
    ↓
Is smart detection enabled?
    ↓ Yes
Check component type:
    - Metadata → Always compress
    - Text → Always compress
    - Asset → Detect file type
        → Already compressed? → Skip
        → Highly compressible? → Compress
        → Unknown? → Analyze data
    - Embeddings → Check sparsity/entropy
        → >80% zeros? → Compress
        → High entropy? → Skip
        → Small? → Compress
    ↓ No
Use header flags (legacy mode)
```

## Future Enhancements

- [ ] Support for custom compression algorithms per component
- [ ] Compression level selection based on file type
- [ ] Parallel compression for large components
- [ ] Compression statistics and reporting
- [ ] Machine learning-based compression prediction

## See Also

- `COMPRESSION_ALGORITHMS_ANALYSIS.md` - Detailed compression algorithm comparison
- `EMBEDDINGS_COMPRESSION_ANALYSIS.md` - Embeddings-specific compression analysis
- `examples/smart_compression.rs` - Complete working example

