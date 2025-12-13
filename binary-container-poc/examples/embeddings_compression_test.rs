// SPDX-License-Identifier: MIT
//! Test embeddings compression with different data patterns
//!
//! This example tests how well different embeddings data patterns compress
//! and measures the compression time vs ratio trade-off.

use binary_container_poc::{ContainerReader, ContainerWriter};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Embeddings Compression Analysis");
    println!("==============================\n");

    // Test different embeddings patterns
    let patterns = vec![
        ("All Zeros", create_zeros_embeddings()),
        ("Random Floats", create_random_floats_embeddings()),
        ("Sparse (90% zeros)", create_sparse_embeddings()),
        ("Sequential", create_sequential_embeddings()),
        ("Real Embeddings Pattern", create_realistic_embeddings()),
    ];

    for (name, embeddings) in patterns {
        println!("Testing: {}", name);
        println!(
            "  Size: {} bytes ({:.2} KB)",
            embeddings.len(),
            embeddings.len() as f64 / 1024.0
        );

        // Test with compression
        test_compression(&embeddings, true, name)?;

        // Test without compression
        test_compression(&embeddings, false, name)?;

        println!();
    }

    Ok(())
}

fn test_compression(
    embeddings: &[u8],
    compress: bool,
    pattern_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = if compress {
        ContainerWriter::new() // Full compression
    } else {
        ContainerWriter::new().with_compression_flags(0) // No compression
    };

    // Create minimal other components
    let metadata = b"{\"test\":\"metadata\"}".to_vec();
    let asset = vec![0xFF; 1024]; // 1KB asset
    let text = b"Test text".to_vec();

    let start = Instant::now();

    writer.add_metadata(metadata)?;
    writer.add_asset(asset)?;
    writer.add_text(text)?;
    writer.add_embeddings(embeddings.to_vec())?;

    let add_time = start.elapsed();

    let finalize_start = Instant::now();
    let data = writer.finalize()?;
    let finalize_time = finalize_start.elapsed();

    let total_time = add_time + finalize_time;

    // Get compressed size from header (before decompression)
    let container = binary_container_poc::BinaryContainer::from_vec(data.clone())?;
    let header = container.header();
    let compressed_size =
        header.component_size(binary_container_poc::ComponentType::Embeddings) as usize;

    // Also get decompressed size for comparison
    let reader = ContainerReader::from_slice(&data)?;
    let embeddings_data = reader.embeddings()?;

    let original_size = embeddings.len();
    let decompressed_size = embeddings_data.len();
    let compression_ratio = if original_size > 0 {
        (compressed_size as f64 / original_size as f64) * 100.0
    } else {
        0.0
    };

    let savings = if original_size > compressed_size {
        ((original_size - compressed_size) as f64 / original_size as f64) * 100.0
    } else {
        0.0
    };

    println!(
        "  {} Compression:",
        if compress { "WITH" } else { "WITHOUT" }
    );
    println!("    Time: {:.2}µs", total_time.as_secs_f64() * 1_000_000.0);
    println!("    Original: {} bytes", original_size);
    println!("    Compressed (in file): {} bytes", compressed_size);
    println!("    Decompressed (verified): {} bytes", decompressed_size);
    println!("    Compression Ratio: {:.2}%", compression_ratio);
    if compress && savings > 0.0 {
        println!("    Space Savings: {:.2}%", savings);
        println!(
            "    Compression Efficiency: {:.2} bytes saved per µs",
            (original_size - compressed_size) as f64 / total_time.as_secs_f64()
        );
    }

    // Calculate time per MB
    let time_per_mb = if original_size > 0 {
        (total_time.as_secs_f64() * 1_000_000.0) / (original_size as f64 / 1_048_576.0)
    } else {
        0.0
    };
    println!("    Time per MB: {:.2}µs/MB", time_per_mb);

    Ok(())
}

/// Create embeddings filled with zeros (best case for compression)
fn create_zeros_embeddings() -> Vec<u8> {
    vec![0x00; 512 * 1024] // 512KB of zeros
}

/// Create embeddings with random float values (worst case for compression)
fn create_random_floats_embeddings() -> Vec<u8> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut data = Vec::with_capacity(512 * 1024);
    let mut hasher = DefaultHasher::new();
    "random_seed".hash(&mut hasher);
    let mut seed = hasher.finish();

    // Generate pseudo-random floats
    for _ in 0..(512 * 1024 / 4) {
        // Simple LCG for pseudo-random
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let float_bytes = (seed & 0xFFFFFFFF) as u32;
        data.extend_from_slice(&float_bytes.to_le_bytes());
    }

    data
}

/// Create sparse embeddings (90% zeros, 10% random) - common in real embeddings
fn create_sparse_embeddings() -> Vec<u8> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut data = vec![0x00; 512 * 1024];
    let mut hasher = DefaultHasher::new();
    "sparse_seed".hash(&mut hasher);
    let mut seed = hasher.finish();

    // Fill 10% with random values
    let num_values = (512 * 1024 / 4) / 10;
    for i in 0..num_values {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let float_bytes = (seed & 0xFFFFFFFF) as u32;
        let offset = (i * 10 * 4) % (512 * 1024 - 4);
        data[offset..offset + 4].copy_from_slice(&float_bytes.to_le_bytes());
    }

    data
}

/// Create sequential embeddings (incremental values) - moderate compression
fn create_sequential_embeddings() -> Vec<u8> {
    let mut data = Vec::with_capacity(512 * 1024);

    for i in 0..(512 * 1024 / 4) {
        let value = (i as f32) * 0.001; // Small incremental values
        let bytes = value.to_bits().to_le_bytes();
        data.extend_from_slice(&bytes);
    }

    data
}

/// Create realistic embeddings pattern (normalized vectors, small values)
fn create_realistic_embeddings() -> Vec<u8> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut data = Vec::with_capacity(512 * 1024);
    let mut hasher = DefaultHasher::new();
    "realistic_seed".hash(&mut hasher);
    let mut seed = hasher.finish();

    // Generate normalized-like values (typical for embeddings)
    // Values are typically small, centered around 0, with some structure
    for _ in 0..(512 * 1024 / 4) {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        // Generate value in range [-1.0, 1.0] with some bias toward 0
        let rand = (seed & 0x7FFFFFFF) as f64 / 0x7FFFFFFF as f64;
        let value = (rand - 0.5) * 2.0 * (1.0 - rand * 0.5); // Bias toward 0
        let float_value = value as f32;
        let bytes = float_value.to_bits().to_le_bytes();
        data.extend_from_slice(&bytes);
    }

    data
}
