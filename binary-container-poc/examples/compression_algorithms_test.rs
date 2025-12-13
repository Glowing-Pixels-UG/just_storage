// SPDX-License-Identifier: MIT
//! Comprehensive compression algorithm comparison
//!
//! This example tests multiple compression algorithms (zstd, lz4, zlib, brotli, snappy)
//! with detailed performance metrics, compression ratios, and resource tracking.

use std::io::Write;
use std::time::Instant;

/// Compression algorithm to test
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CompressionAlgorithm {
    None,
    Zlib,   // Current default (flate2 with zlib-rs)
    Zstd1,  // zstd level 1 (fastest)
    Zstd3,  // zstd level 3 (default)
    Zstd6,  // zstd level 6 (balanced)
    Zstd9,  // zstd level 9 (best compression)
    Lz4,    // lz4_flex (fastest pure Rust LZ4)
    Brotli, // Google's brotli
    Snappy, // Google's snappy
}

impl CompressionAlgorithm {
    fn name(&self) -> &'static str {
        match self {
            CompressionAlgorithm::None => "none",
            CompressionAlgorithm::Zlib => "zlib",
            CompressionAlgorithm::Zstd1 => "zstd-1",
            CompressionAlgorithm::Zstd3 => "zstd-3",
            CompressionAlgorithm::Zstd6 => "zstd-6",
            CompressionAlgorithm::Zstd9 => "zstd-9",
            CompressionAlgorithm::Lz4 => "lz4",
            CompressionAlgorithm::Brotli => "brotli",
            CompressionAlgorithm::Snappy => "snappy",
        }
    }

    fn all() -> Vec<CompressionAlgorithm> {
        vec![
            CompressionAlgorithm::None,
            CompressionAlgorithm::Zlib,
            CompressionAlgorithm::Zstd1,
            CompressionAlgorithm::Zstd3,
            CompressionAlgorithm::Zstd6,
            CompressionAlgorithm::Zstd9,
            CompressionAlgorithm::Lz4,
            CompressionAlgorithm::Brotli,
            CompressionAlgorithm::Snappy,
        ]
    }
}

/// Compression result with detailed metrics
#[derive(Debug, Clone)]
struct CompressionResult {
    algorithm: CompressionAlgorithm,
    original_size: usize,
    compressed_size: usize,
    compression_time: std::time::Duration,
    decompression_time: std::time::Duration,
    compression_ratio: f64,
    compression_speed_mbps: f64,
    decompression_speed_mbps: f64,
    success: bool,
    error: Option<String>,
}

impl CompressionResult {
    fn compression_efficiency(&self) -> f64 {
        // Bytes saved per microsecond
        if self.compression_time.as_secs_f64() > 0.0 {
            let bytes_saved = self.original_size.saturating_sub(self.compressed_size);
            bytes_saved as f64 / (self.compression_time.as_secs_f64() * 1_000_000.0)
        } else {
            0.0
        }
    }

    fn space_savings_percent(&self) -> f64 {
        if self.original_size > 0 {
            if self.compressed_size <= self.original_size {
                ((self.original_size - self.compressed_size) as f64 / self.original_size as f64)
                    * 100.0
            } else {
                // Compression expanded the data (negative savings)
                -((self.compressed_size - self.original_size) as f64 / self.original_size as f64)
                    * 100.0
            }
        } else {
            0.0
        }
    }
}

/// Test compression with a specific algorithm
fn test_compression(
    algorithm: CompressionAlgorithm,
    data: &[u8],
    _name: &str,
) -> CompressionResult {
    let original_size = data.len();

    // Compression
    let compress_start = Instant::now();
    let compress_result = match algorithm {
        CompressionAlgorithm::None => Ok(data.to_vec()),

        CompressionAlgorithm::Zlib => {
            #[cfg(feature = "compression")]
            {
                use flate2::write::ZlibEncoder;
                use flate2::Compression;
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
                match encoder.write_all(data) {
                    Ok(_) => encoder.finish().map_err(|e| format!("Zlib error: {}", e)),
                    Err(e) => Err(format!("Zlib write error: {}", e)),
                }
            }
            #[cfg(not(feature = "compression"))]
            Err("zlib not available".to_string())
        }

        CompressionAlgorithm::Zstd1 => {
            #[cfg(feature = "zstd")]
            {
                zstd::encode_all(data, 1).map_err(|e| format!("Zstd-1 error: {}", e))
            }
            #[cfg(not(feature = "zstd"))]
            Err("zstd not available".to_string())
        }

        CompressionAlgorithm::Zstd3 => {
            #[cfg(feature = "zstd")]
            {
                zstd::encode_all(data, 3).map_err(|e| format!("Zstd-3 error: {}", e))
            }
            #[cfg(not(feature = "zstd"))]
            Err("zstd not available".to_string())
        }

        CompressionAlgorithm::Zstd6 => {
            #[cfg(feature = "zstd")]
            {
                zstd::encode_all(data, 6).map_err(|e| format!("Zstd-6 error: {}", e))
            }
            #[cfg(not(feature = "zstd"))]
            Err("zstd not available".to_string())
        }

        CompressionAlgorithm::Zstd9 => {
            #[cfg(feature = "zstd")]
            {
                zstd::encode_all(data, 9).map_err(|e| format!("Zstd-9 error: {}", e))
            }
            #[cfg(not(feature = "zstd"))]
            Err("zstd not available".to_string())
        }

        CompressionAlgorithm::Lz4 => {
            #[cfg(feature = "lz4")]
            {
                Ok(lz4_flex::block::compress(data))
            }
            #[cfg(not(feature = "lz4"))]
            Err("lz4 not available".to_string())
        }

        CompressionAlgorithm::Brotli => {
            #[cfg(feature = "brotli")]
            {
                use std::io::Write;
                let mut encoder = brotli::CompressorWriter::new(Vec::new(), 4096, 6, 22);
                match encoder.write_all(data) {
                    Ok(_) => {
                        encoder.flush().ok();
                        match encoder.into_inner() {
                            Ok(result) => Ok(result),
                            Err(e) => Err(format!("Brotli finish error: {:?}", e)),
                        }
                    }
                    Err(e) => Err(format!("Brotli write error: {}", e)),
                }
            }
            #[cfg(not(feature = "brotli"))]
            Err("brotli not available".to_string())
        }

        CompressionAlgorithm::Snappy => {
            #[cfg(feature = "snappy")]
            {
                snap::raw::Encoder::new()
                    .compress_vec(data)
                    .map_err(|e| format!("Snappy error: {}", e))
            }
            #[cfg(not(feature = "snappy"))]
            Err("snappy not available".to_string())
        }
    };
    let compression_time = compress_start.elapsed();

    let (compressed_data, success, error) = match compress_result {
        Ok(data) => (data, true, None),
        Err(e) => (Vec::new(), false, Some(e)),
    };

    let compressed_size = compressed_data.len();

    // Decompression (if compression succeeded)
    let decompression_time = if success {
        let decompress_start = Instant::now();
        let _decompressed = match algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),

            CompressionAlgorithm::Zlib => {
                #[cfg(feature = "compression")]
                {
                    use flate2::read::ZlibDecoder;
                    use std::io::Read;
                    let mut decoder = ZlibDecoder::new(&compressed_data[..]);
                    let mut result = Vec::new();
                    decoder
                        .read_to_end(&mut result)
                        .map(|_| result)
                        .map_err(|e| format!("Zlib decompress error: {}", e))
                }
                #[cfg(not(feature = "compression"))]
                Err("zlib not available".to_string())
            }

            CompressionAlgorithm::Zstd1
            | CompressionAlgorithm::Zstd3
            | CompressionAlgorithm::Zstd6
            | CompressionAlgorithm::Zstd9 => {
                #[cfg(feature = "zstd")]
                {
                    zstd::decode_all(&compressed_data[..])
                        .map_err(|e| format!("Zstd decompress error: {}", e))
                }
                #[cfg(not(feature = "zstd"))]
                Err("zstd not available".to_string())
            }

            CompressionAlgorithm::Lz4 => {
                #[cfg(feature = "lz4")]
                {
                    lz4_flex::block::decompress(&compressed_data, original_size)
                        .map_err(|e| format!("LZ4 decompress error: {:?}", e))
                }
                #[cfg(not(feature = "lz4"))]
                Err("lz4 not available".to_string())
            }

            CompressionAlgorithm::Brotli => {
                #[cfg(feature = "brotli")]
                {
                    use std::io::Read;
                    let mut decoder = brotli::Decompressor::new(&compressed_data[..], 4096);
                    let mut result = Vec::new();
                    match decoder.read_to_end(&mut result) {
                        Ok(_) => Ok(result),
                        Err(e) => Err(format!("Brotli decompress error: {}", e)),
                    }
                }
                #[cfg(not(feature = "brotli"))]
                Err("brotli not available".to_string())
            }

            CompressionAlgorithm::Snappy => {
                #[cfg(feature = "snappy")]
                {
                    snap::raw::Decoder::new()
                        .decompress_vec(&compressed_data)
                        .map_err(|e| format!("Snappy decompress error: {}", e))
                }
                #[cfg(not(feature = "snappy"))]
                Err("snappy not available".to_string())
            }
        };

        let decompress_time = decompress_start.elapsed();

        // Verify round-trip
        if let Ok(decompressed) = _decompressed {
            if decompressed != data {
                return CompressionResult {
                    algorithm,
                    original_size,
                    compressed_size,
                    compression_time,
                    decompression_time: decompress_time,
                    compression_ratio: compressed_size as f64 / original_size as f64,
                    compression_speed_mbps: 0.0,
                    decompression_speed_mbps: 0.0,
                    success: false,
                    error: Some("Round-trip verification failed".to_string()),
                };
            }
        }

        decompress_time
    } else {
        std::time::Duration::ZERO
    };

    let compression_ratio = if original_size > 0 {
        compressed_size as f64 / original_size as f64
    } else {
        0.0
    };

    let compression_speed_mbps = if compression_time.as_secs_f64() > 0.0 {
        (original_size as f64 / 1_048_576.0) / compression_time.as_secs_f64()
    } else {
        0.0
    };

    let decompression_speed_mbps = if decompression_time.as_secs_f64() > 0.0 {
        (original_size as f64 / 1_048_576.0) / decompression_time.as_secs_f64()
    } else {
        0.0
    };

    CompressionResult {
        algorithm,
        original_size,
        compressed_size,
        compression_time,
        decompression_time,
        compression_ratio,
        compression_speed_mbps,
        decompression_speed_mbps,
        success,
        error,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Compression Algorithms Comparison");
    println!("==================================\n");

    // Check if PDF file is provided as argument
    let pdf_path = std::env::args().nth(1);

    let test_cases = if let Some(path) = pdf_path {
        println!("Using PDF file: {}\n", path);
        let pdf_data = std::fs::read(&path)?;
        vec![("Real PDF File", pdf_data)]
    } else {
        // Create test data patterns
        vec![
            ("All Zeros (512KB)", vec![0x00; 512 * 1024]),
            ("Sparse (90% zeros, 512KB)", create_sparse_data(512 * 1024)),
            ("Text (100KB)", vec![b'A'; 100 * 1024]),
            ("Random (512KB)", create_random_data(512 * 1024)),
            ("Mixed (1MB)", create_mixed_data(1024 * 1024)),
        ]
    };

    // Test each algorithm on each data pattern
    let mut all_results: Vec<(String, Vec<CompressionResult>)> = Vec::new();

    for (name, data) in test_cases {
        println!("Testing: {}", name);
        println!(
            "  Size: {} bytes ({:.2} KB)\n",
            data.len(),
            data.len() as f64 / 1024.0
        );

        let mut results = Vec::new();
        for &algorithm in CompressionAlgorithm::all().iter() {
            let result = test_compression(algorithm, &data, name);
            results.push(result.clone());

            if result.success {
                println!("  {}:", result.algorithm.name());
                println!(
                    "    Compress: {:.2}µs ({:.2} MB/s)",
                    result.compression_time.as_secs_f64() * 1_000_000.0,
                    result.compression_speed_mbps
                );
                println!(
                    "    Decompress: {:.2}µs ({:.2} MB/s)",
                    result.decompression_time.as_secs_f64() * 1_000_000.0,
                    result.decompression_speed_mbps
                );
                println!(
                    "    Size: {} bytes ({:.2}% of original, {:.2}% savings)",
                    result.compressed_size,
                    result.compression_ratio * 100.0,
                    result.space_savings_percent()
                );
                println!(
                    "    Efficiency: {:.2} bytes saved/µs",
                    result.compression_efficiency()
                );
            } else {
                println!(
                    "  {}: FAILED - {}",
                    result.algorithm.name(),
                    result
                        .error
                        .as_ref()
                        .unwrap_or(&"Unknown error".to_string())
                );
            }
            println!();
        }

        all_results.push((name.to_string(), results));
    }

    // Generate summary report
    generate_summary_report(&all_results);

    Ok(())
}

fn create_sparse_data(size: usize) -> Vec<u8> {
    let mut data = vec![0x00; size];
    // Fill 10% with random values
    for i in 0..(size / 10) {
        data[i * 10] = (i % 256) as u8;
    }
    data
}

fn create_random_data(size: usize) -> Vec<u8> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut data = Vec::with_capacity(size);
    let mut hasher = DefaultHasher::new();
    "random_seed".hash(&mut hasher);
    let mut seed = hasher.finish();

    for _ in 0..size {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        data.push((seed & 0xFF) as u8);
    }

    data
}

fn create_mixed_data(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);

    // Mix of patterns: text, zeros, random
    let text = b"Hello world, this is test data for compression benchmarking. ";
    let text_len = text.len();

    for i in 0..size {
        if i % 3 == 0 {
            data.push(text[i % text_len]);
        } else if i % 3 == 1 {
            data.push(0x00);
        } else {
            data.push((i % 256) as u8);
        }
    }

    data
}

fn generate_summary_report(all_results: &[(String, Vec<CompressionResult>)]) {
    println!("\n");
    println!("{}", "=".repeat(100));
    println!("SUMMARY REPORT");
    println!("{}", "=".repeat(100));
    println!();

    // Find best algorithm for each metric
    for (name, results) in all_results {
        println!("Data Pattern: {}", name);
        println!("{}", "-".repeat(100));

        // Best compression ratio
        if let Some(best_ratio) = results.iter().filter(|r| r.success).min_by(|a, b| {
            a.compression_ratio
                .partial_cmp(&b.compression_ratio)
                .unwrap()
        }) {
            println!(
                "  Best Compression Ratio: {} ({:.2}%, {:.2}% savings)",
                best_ratio.algorithm.name(),
                best_ratio.compression_ratio * 100.0,
                best_ratio.space_savings_percent()
            );
        }

        // Fastest compression
        if let Some(fastest_compress) = results
            .iter()
            .filter(|r| r.success)
            .min_by(|a, b| a.compression_time.cmp(&b.compression_time))
        {
            println!(
                "  Fastest Compression: {} ({:.2}µs, {:.2} MB/s)",
                fastest_compress.algorithm.name(),
                fastest_compress.compression_time.as_secs_f64() * 1_000_000.0,
                fastest_compress.compression_speed_mbps
            );
        }

        // Fastest decompression
        if let Some(fastest_decompress) = results
            .iter()
            .filter(|r| r.success)
            .min_by(|a, b| a.decompression_time.cmp(&b.decompression_time))
        {
            println!(
                "  Fastest Decompression: {} ({:.2}µs, {:.2} MB/s)",
                fastest_decompress.algorithm.name(),
                fastest_decompress.decompression_time.as_secs_f64() * 1_000_000.0,
                fastest_decompress.decompression_speed_mbps
            );
        }

        // Best efficiency (bytes saved per µs)
        if let Some(best_efficiency) = results
            .iter()
            .filter(|r| r.success && r.compression_ratio < 1.0)
            .max_by(|a, b| {
                a.compression_efficiency()
                    .partial_cmp(&b.compression_efficiency())
                    .unwrap()
            })
        {
            println!(
                "  Best Efficiency: {} ({:.2} bytes saved/µs)",
                best_efficiency.algorithm.name(),
                best_efficiency.compression_efficiency()
            );
        }

        println!();
    }

    // Overall recommendations
    println!("RECOMMENDATIONS");
    println!("{}", "-".repeat(100));
    println!("Based on the results:");
    println!("  - For maximum speed: Use LZ4 or Snappy");
    println!("  - For best ratio: Use Zstd-9 or Brotli");
    println!("  - For balanced: Use Zstd-3 or Zstd-6");
    println!("  - For zero-filled/sparse data: All algorithms compress well, choose by speed");
    println!("  - For random data: Compression may expand, consider skipping");
    println!();
}
