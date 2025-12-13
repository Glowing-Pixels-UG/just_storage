// SPDX-License-Identifier: MIT
//! Basic usage example for Binary Document Container

use binary_container_poc::{ContainerReader, ContainerWriter};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Binary Document Container - Basic Usage ===\n");

    // Step 1: Create component data
    println!("1. Creating component data...");
    let metadata = br#"{
        "document": {
            "id": "bdc-example-123",
            "name": "binary_container_demo.pdf",
            "created": "2025-12-13T14:00:00Z",
            "source": "binary-poc-scanner",
            "workflow": "performance-demo"
        },
        "processing": {
            "ocr": {
                "engine": "tesseract",
                "language": "eng",
                "confidence": 0.95
            },
            "embeddings": {
                "model": "binary-optimized-model",
                "dimensions": 1536,
                "generated": "2025-12-13T14:00:01Z"
            }
        }
    }"#
    .to_vec();

    let asset = b"Binary PDF content - this could be much larger".to_vec();
    let text = b"Extracted text from the binary document container".to_vec();
    let embeddings = b"Binary embeddings data (would be float32 arrays)".to_vec();

    println!("   Metadata: {} bytes", metadata.len());
    println!("   Asset: {} bytes", asset.len());
    println!("   Text: {} bytes", text.len());
    println!("   Embeddings: {} bytes", embeddings.len());

    // Step 2: Create container
    println!("\n2. Creating binary container...");
    let mut writer = ContainerWriter::new();
    writer.add_metadata(metadata)?;
    writer.add_asset(asset)?;
    writer.add_text(text)?;
    writer.add_embeddings(embeddings)?;

    let binary_data = writer.finalize()?;
    println!("   Container created: {} bytes", binary_data.len());
    println!(
        "   Header size: {} bytes",
        binary_container_poc::format::BDC_HEADER_SIZE
    );

    // Step 3: Write to file
    println!("\n3. Writing to file...");
    let output_path = PathBuf::from("/tmp/binary_container.bdc");
    std::fs::write(&output_path, &binary_data)?;
    println!("   Written to: {}", output_path.display());

    // Step 4: Read from file
    println!("\n4. Reading from file...");
    let file_data = std::fs::read(&output_path)?;
    let reader = ContainerReader::from_slice(&file_data)?;
    println!("   Container loaded successfully");

    // Step 5: Access components uniformly (like JSON)
    println!("\n5. Accessing components uniformly...");

    // JSON-like access
    let metadata_str = reader.metadata_json()?;
    println!("   Metadata (JSON): {} chars", metadata_str.len());

    let asset_data = reader.asset()?;
    println!("   Asset: {} bytes", asset_data.len());

    let text_str = reader.text_string()?;
    let preview_len = text_str.len().min(50);
    println!(
        "   Text: '{}' ({} chars)",
        &text_str[..preview_len],
        text_str.len()
    );

    let embeddings_data = reader.embeddings()?;
    println!("   Embeddings: {} bytes", embeddings_data.len());

    // Step 6: Get statistics
    println!("\n6. Container statistics...");
    let stats = reader.stats();
    println!("   Total size: {} bytes", stats.total_size);
    println!("   Header size: {} bytes", stats.header_size);
    println!("   Metadata: {} bytes", stats.metadata_size);
    println!("   Asset: {} bytes", stats.asset_size);
    println!("   Text: {} bytes", stats.text_size);
    println!("   Embeddings: {} bytes", stats.embeddings_size);
    println!(
        "   Compression ratio: {:.2}%",
        stats.compression_ratio() * 100.0
    );

    // Step 7: Compare with ZIP size
    println!("\n7. Comparing with ZIP format...");
    let zip_size = compare_with_zip(&binary_data)?;
    println!("   BDC size: {} bytes", binary_data.len());
    println!("   ZIP size: {} bytes", zip_size);
    println!(
        "   BDC is {:.1}% of ZIP size",
        (binary_data.len() as f64 / zip_size as f64) * 100.0
    );

    println!("\n=== Binary Container Demo Complete ===");
    println!("\nKey advantages demonstrated:");
    println!("  ✅ O(1) component access via fixed header");
    println!("  ✅ Uniform access (JSON-like field access)");
    println!("  ✅ Minimal overhead (32-byte header)");
    println!("  ✅ High performance for document bundling");
    println!("  ✅ Memory mappable for large files");

    Ok(())
}

fn compare_with_zip(bdc_data: &[u8]) -> Result<u64, Box<dyn std::error::Error>> {
    use document_bundler::{BundleBuilder, BundleWriter, DocumentInfo};

    // Extract components from BDC
    let reader = ContainerReader::from_vec(bdc_data.to_vec())?;
    let asset = reader.asset()?;
    let text = reader.text_string()?;
    let embeddings = reader.embeddings()?;

    // Create a simple metadata for comparison (avoid UUID parsing issues)
    let doc = DocumentInfo::new(
        "bdc-comparison".to_string(),
        "comparison-scanner".to_string(),
        "comparison-workflow".to_string(),
    );
    let metadata = document_bundler::BundleMetadata::new(doc);

    // Create ZIP bundle
    let bundle = BundleBuilder::new()
        .metadata(metadata)
        .asset(document_bundler::BundleFile::with_data(
            "asset.pdf".into(),
            asset,
            "application/pdf".to_string(),
        ))
        .text(document_bundler::BundleFile::with_data(
            "text.txt".into(),
            text.into_bytes(),
            "text/plain".to_string(),
        ))
        .embeddings(document_bundler::BundleFile::with_data(
            "embeddings.parquet".into(),
            embeddings,
            "application/x-parquet".to_string(),
        ))
        .build()?;

    // Write to temp file and get size
    let temp_file = tempfile::NamedTempFile::new()?;
    let writer = BundleWriter::new();
    writer.write(&bundle, temp_file.path())?;

    let zip_size = std::fs::metadata(temp_file.path())?.len();
    Ok(zip_size)
}
