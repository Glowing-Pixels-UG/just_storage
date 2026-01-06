/// Hash computation benchmarks
/// Measures SHA-256 performance with SIMD optimizations
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use just_storage::infrastructure::storage::ContentHasher;
use std::io::Cursor;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn hash_computation_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("hash_computation");
    group.measurement_time(Duration::from_secs(10));

    for size in [1024, 64 * 1024, 1024 * 1024, 10 * 1024 * 1024].iter() {
        let size = *size;
        group.throughput(Throughput::Bytes(size as u64));

        // Benchmark hash computation during write (streaming hash)
        group.bench_with_input(BenchmarkId::new("write_and_hash", size), &size, |b, &s| {
            let temp_dir = TempDir::new().unwrap();
            b.to_async(&rt).iter_custom(|iters| {
                let temp_dir = temp_dir.path().to_path_buf();
                async move {
                    let mut total_duration = Duration::default();
                    for i in 0..iters {
                        let mut data = vec![0u8; s];
                        // Make each iteration unique to avoid deduplication
                        let prefix = i.to_le_bytes();
                        for (j, byte) in prefix.iter().enumerate() {
                            if j < data.len() {
                                data[j] = *byte;
                            }
                        }

                        let start = std::time::Instant::now();
                        let path = temp_dir.join(format!("hash_{}.tmp", i));
                        let reader = Box::pin(Cursor::new(data));
                        ContentHasher::write_and_hash_with_durability(&path, reader, false)
                            .await
                            .unwrap();
                        total_duration += start.elapsed();
                    }
                    total_duration
                }
            })
        });

        // Benchmark hash computation of existing file
        group.bench_with_input(BenchmarkId::new("hash_file", size), &size, |b, &s| {
            let temp_dir = TempDir::new().unwrap();
            // Prepare file once
            let file_path = temp_dir.path().join("hash_file.tmp");
            rt.block_on(async {
                let data = vec![0u8; s];
                let reader = Box::pin(Cursor::new(data));
                ContentHasher::write_and_hash_with_durability(&file_path, reader, false)
                    .await
                    .unwrap();
            });

            b.to_async(&rt).iter(|| {
                let path = file_path.clone();
                async move {
                    ContentHasher::hash_file(&path).await.unwrap();
                }
            })
        });

        // Benchmark pure SHA-256 computation (without I/O)
        group.bench_with_input(BenchmarkId::new("sha256_pure", size), &size, |b, &s| {
            let mut data = vec![0u8; s];
            // Initialize with some data
            for (i, item) in data.iter_mut().enumerate() {
                *item = (i % 256) as u8;
            }

            b.iter(|| {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(&data);
                let _hash = hasher.finalize();
                std::hint::black_box(_hash);
            })
        });
    }

    group.finish();
}

criterion_group!(benches, hash_computation_benchmarks);
criterion_main!(benches);
