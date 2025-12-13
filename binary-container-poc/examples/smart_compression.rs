// SPDX-License-Identifier: MIT
//! Example demonstrating smart compression detection
//!
//! This example shows how the BDC format automatically detects file types
//! and compresses only when beneficial.

use binary_container_poc::{ContainerReader, ContainerWriter};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Smart Compression Detection Example");
    println!("===================================\n");

    // Example 1: PDF file (already compressed - should skip compression)
    println!("Example 1: PDF File (Already Compressed)");
    println!("----------------------------------------");

    let pdf_path = std::env::args().nth(1).unwrap_or_else(|| {
        "/Users/damirmukimov/projects/just_storage/document_20251116_050039.pdf".to_string()
    });

    if let Ok(pdf_data) = fs::read(&pdf_path) {
        println!(
            "  PDF size: {} bytes ({:.2} KB)",
            pdf_data.len(),
            pdf_data.len() as f64 / 1024.0
        );

        // Create container with smart compression
        let mut writer = ContainerWriter::new().set_asset_mime_type("application/pdf");

        writer.add_metadata(
            serde_json::json!({
                "document": {
                    "id": "test-001",
                    "name": "test_document",
                    "created": "2025-01-16T05:00:39Z"
                }
            })
            .to_string()
            .into_bytes(),
        )?;

        writer.add_asset(pdf_data.clone())?;
        writer.add_text(b"This is some extracted text from the document.".to_vec())?;
        writer.add_embeddings(vec![])?; // Empty embeddings for this example

        let container_data = writer.finalize()?;
        println!(
            "  Container size: {} bytes ({:.2} KB)",
            container_data.len(),
            container_data.len() as f64 / 1024.0
        );

        // Read back
        let reader = ContainerReader::from_vec(container_data)?;
        let header = reader.header();
        println!("  Compression flags: 0x{:08x}", header.flags);

        // Check if asset was compressed
        let asset_compressed =
            (header.flags & binary_container_poc::format::flags::COMPRESS_ASSET) != 0;
        println!("  Asset compressed: {}", asset_compressed);
        println!("  → PDF should NOT be compressed (already compressed format)\n");
    } else {
        println!("  PDF file not found: {}\n", pdf_path);
    }

    // Example 2: Text file (highly compressible - should compress)
    println!("Example 2: Text File (Highly Compressible)");
    println!("-------------------------------------------");

    let text_data = b"Hello, this is a text file that should compress very well! ".repeat(100);
    println!(
        "  Text size: {} bytes ({:.2} KB)",
        text_data.len(),
        text_data.len() as f64 / 1024.0
    );

    let mut writer = ContainerWriter::new().set_asset_mime_type("text/plain");

    writer.add_metadata(
        serde_json::json!({
            "document": {"id": "test-002"}
        })
        .to_string()
        .into_bytes(),
    )?;

    writer.add_asset(text_data.clone())?;
    writer.add_text(b"Some extracted text.".to_vec())?;
    writer.add_embeddings(vec![])?; // Empty embeddings for this example

    let container_data = writer.finalize()?;
    println!(
        "  Container size: {} bytes ({:.2} KB)",
        container_data.len(),
        container_data.len() as f64 / 1024.0
    );

    let reader = ContainerReader::from_vec(container_data)?;
    let header = reader.header();
    let asset_compressed =
        (header.flags & binary_container_poc::format::flags::COMPRESS_ASSET) != 0;
    println!("  Asset compressed: {}", asset_compressed);
    println!("  → Text SHOULD be compressed (highly compressible format)\n");

    // Example 3: Sparse embeddings (should compress)
    println!("Example 3: Sparse Embeddings (Should Compress)");
    println!("----------------------------------------------");

    let mut sparse_embeddings = vec![0u8; 512 * 1024]; // 512KB of mostly zeros
    sparse_embeddings[0] = 1;
    sparse_embeddings[100] = 2;
    sparse_embeddings[1000] = 3;
    println!(
        "  Embeddings size: {} bytes ({:.2} KB)",
        sparse_embeddings.len(),
        sparse_embeddings.len() as f64 / 1024.0
    );

    let mut writer = ContainerWriter::new();
    writer.add_metadata(
        serde_json::json!({
            "document": {"id": "test-003"}
        })
        .to_string()
        .into_bytes(),
    )?;

    writer.add_asset(vec![])?; // Empty asset
    writer.add_text(b"Some text.".to_vec())?;
    writer.add_embeddings(sparse_embeddings.clone())?;

    let container_data = writer.finalize()?;
    println!(
        "  Container size: {} bytes ({:.2} KB)",
        container_data.len(),
        container_data.len() as f64 / 1024.0
    );

    let reader = ContainerReader::from_vec(container_data)?;
    let header = reader.header();
    let embeddings_compressed =
        (header.flags & binary_container_poc::format::flags::COMPRESS_EMBEDDINGS) != 0;
    println!("  Embeddings compressed: {}", embeddings_compressed);
    println!("  → Sparse embeddings SHOULD be compressed (99%+ savings)\n");

    // Example 4: Random data (should skip compression)
    println!("Example 4: Random Data (Should Skip Compression)");
    println!("------------------------------------------------");

    let random_data: Vec<u8> = (0..512 * 1024).map(|i| (i * 7) as u8).collect();
    println!(
        "  Random data size: {} bytes ({:.2} KB)",
        random_data.len(),
        random_data.len() as f64 / 1024.0
    );

    let mut writer = ContainerWriter::new();
    writer.add_metadata(
        serde_json::json!({
            "document": {"id": "test-004"}
        })
        .to_string()
        .into_bytes(),
    )?;

    writer.add_asset(vec![])?; // Empty asset
    writer.add_text(b"Some text.".to_vec())?;
    writer.add_embeddings(random_data.clone())?;

    let container_data = writer.finalize()?;
    println!(
        "  Container size: {} bytes ({:.2} KB)",
        container_data.len(),
        container_data.len() as f64 / 1024.0
    );

    let reader = ContainerReader::from_vec(container_data)?;
    let header = reader.header();
    let embeddings_compressed =
        (header.flags & binary_container_poc::format::flags::COMPRESS_EMBEDDINGS) != 0;
    println!("  Embeddings compressed: {}", embeddings_compressed);
    println!("  → Random data should NOT be compressed (expands)\n");

    println!("Summary");
    println!("=======");
    println!("Smart compression automatically:");
    println!("  ✓ Skips compression for already-compressed formats (PDF, JPEG, etc.)");
    println!("  ✓ Compresses highly compressible formats (text, JSON, etc.)");
    println!("  ✓ Detects sparse embeddings and compresses them");
    println!("  ✓ Skips compression for random/uncompressible data");
    println!("\nThis results in:");
    println!("  • Faster writes (skip unnecessary compression)");
    println!("  • Better compression ratios (compress only what benefits)");
    println!("  • Optimal file sizes (no expansion from compressing random data)");

    Ok(())
}
