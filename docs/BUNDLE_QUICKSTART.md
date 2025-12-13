# Document Bundle Quick Start Guide

## Installation

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
just_storage = { path = "../rust" }
```

## Basic Usage

### 1. Create a Bundle

```rust
use just_storage::domain::entities::{BundleFile, DocumentBundleBuilder};
use just_storage::domain::value_objects::{BundleMetadata, DocumentInfo};
use just_storage::infrastructure::bundling::BundleWriter;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create metadata
    let doc = DocumentInfo::new(
        "my_document".to_string(),
        "scanner-1".to_string(),
        "email".to_string(),
    );
    let metadata = BundleMetadata::new(doc);
    
    // Create files
    let asset = BundleFile::with_data(
        PathBuf::from("doc.pdf"),
        std::fs::read("document.pdf")?,
        "application/pdf".to_string(),
    );
    
    let text = BundleFile::with_data(
        PathBuf::from("text.txt"),
        std::fs::read("text.txt")?,
        "text/plain".to_string(),
    );
    
    let embeddings = BundleFile::with_data(
        PathBuf::from("embeddings.parquet"),
        std::fs::read("embeddings.parquet")?,
        "application/x-parquet".to_string(),
    );
    
    // Build bundle
    let bundle = DocumentBundleBuilder::new()
        .metadata(metadata)
        .asset(asset)
        .text(text)
        .embeddings(embeddings)
        .build()?;
    
    // Write to disk
    let writer = BundleWriter::new();
    writer.write(&bundle, &PathBuf::from("output.dc"))?;
    
    println!("Bundle created: output.dc");
    Ok(())
}
```

### 2. Read a Bundle

```rust
use just_storage::infrastructure::bundling::BundleReader;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reader = BundleReader::new();
    let extracted = reader.read(&PathBuf::from("output.dc"))?;
    
    println!("Document: {}", extracted.metadata.document.name);
    println!("Text: {}", extracted.text);
    println!("Asset size: {} bytes", extracted.asset.len());
    
    Ok(())
}
```

### 3. Read Only Metadata (Fast)

```rust
use just_storage::infrastructure::bundling::BundleReader;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reader = BundleReader::new();
    
    // Fast: only reads manifest
    let manifest = reader.read_manifest_only(&PathBuf::from("output.dc"))?;
    println!("Format: {} v{}", manifest.format, manifest.version);
    
    // Fast: only reads manifest + metadata
    let metadata = reader.read_metadata_only(&PathBuf::from("output.dc"))?;
    println!("Document: {}", metadata.document.name);
    
    Ok(())
}
```

## Advanced Usage

### Add Processing Information

```rust
use just_storage::domain::value_objects::{
    BundleMetadata, DocumentInfo, OcrInfo, EmbeddingInfo, ProcessingInfo
};

let doc = DocumentInfo::new(
    "scan_001".to_string(),
    "canon-lide-120".to_string(),
    "archive".to_string(),
);

let ocr = OcrInfo::new(
    "tesseract".to_string(),
    "eng+deu".to_string(),
    0.95,
);

let embeddings = EmbeddingInfo::new(
    "text-embedding-ada-002".to_string(),
    1536,
);

let processing = ProcessingInfo::new()
    .with_ocr(ocr)
    .with_embeddings(embeddings);

let metadata = BundleMetadata::new(doc)
    .with_processing(processing);
```

### Custom Write Options

```rust
use just_storage::infrastructure::bundling::{BundleWriter, BundleWriteOptions};

let options = BundleWriteOptions {
    text_compression_level: 9,      // Maximum compression
    compress_binary_assets: true,    // Compress PDFs and Parquet
    creator: "my-app".to_string(),
};

let writer = BundleWriter::with_options(options);
writer.write(&bundle, &output_path)?;
```

### Skip Verification (Faster Reads)

```rust
use just_storage::infrastructure::bundling::{BundleReader, BundleReadOptions};

let options = BundleReadOptions {
    verify_checksums: false,  // Skip SHA-256 verification
    verify_sizes: false,      // Skip size verification
};

let reader = BundleReader::with_options(options);
let extracted = reader.read(&bundle_path)?;
```

## Running the Example

```bash
# Run the complete example
cargo run --example document_bundle_example

# Run tests
cargo test --lib bundling

# Check for linting errors
cargo clippy
```

## File Format

The `.dc` file is a standard ZIP archive with this structure:

```
document.dc
├── META-INF/
│   ├── manifest.json     # File index with checksums
│   └── metadata.json     # Document metadata
├── assets/
│   └── document.pdf      # Original file
└── data/
    ├── text.txt         # Extracted text
    └── embeddings.parquet  # Vector embeddings
```

You can inspect it with any ZIP tool:

```bash
unzip -l document.dc
```

## Common Patterns

### Pattern 1: Batch Processing

```rust
for file in scan_directory("scans/")? {
    let bundle = create_bundle_from_scan(&file)?;
    let output = format!("bundles/{}.dc", file.stem().unwrap());
    writer.write(&bundle, Path::new(&output))?;
}
```

### Pattern 2: Metadata Indexing

```rust
let mut index = Vec::new();
for bundle_file in list_bundles("archive/")? {
    let metadata = reader.read_metadata_only(&bundle_file)?;
    index.push((bundle_file, metadata));
}
```

### Pattern 3: Selective Extraction

```rust
let manifest = reader.read_manifest_only(&bundle_path)?;
if manifest.files.get("text").unwrap().size < 1_000_000 {
    let extracted = reader.read(&bundle_path)?;
    process_text(&extracted.text);
}
```

## Troubleshooting

### Error: "Validation error: Missing required file entry"

Make sure all four components are provided:
- metadata
- asset
- text
- embeddings

### Error: "Checksum mismatch"

The file may be corrupted. Try:
1. Re-download the bundle
2. Disable checksum verification (not recommended)
3. Check disk for errors

### Error: "File not found in bundle"

The bundle may be incomplete. Verify with:

```rust
let files = reader.list_files(&bundle_path)?;
println!("Files in bundle: {:?}", files);
```

## Best Practices

1. **Always validate** bundles after creation
2. **Use metadata-only reads** for indexing/searching
3. **Enable checksums** for production (default)
4. **Compress text**, don't compress binary assets
5. **Handle errors** appropriately (don't unwrap in production)
6. **Test with large files** (>1GB) if needed
7. **Monitor memory usage** for very large bundles

## Next Steps

- Read the [full POC documentation](DOCUMENT_BUNDLE_POC.md)
- Check the [API documentation](../rust/src/infrastructure/bundling/mod.rs)
- Review the [example code](../rust/examples/document_bundle_example.rs)
- Explore [integration tests](../rust/tests/)

## Support

For issues or questions:
- Check the [main README](../README.md)
- Review the [architecture documentation](ARCHITECTURE.md)
- Open an issue on GitHub

