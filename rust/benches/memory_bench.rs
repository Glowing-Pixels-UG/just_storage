/// Memory and resource usage benchmarks
/// Measures allocation patterns and memory efficiency
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use just_storage::application::ports::BlobStore;
use just_storage::domain::value_objects::StorageClass;
use just_storage::infrastructure::storage::LocalFilesystemStore;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Benchmark allocation patterns - comparing pre-allocated vs new allocations
fn allocation_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("allocation_patterns");
    group.measurement_time(Duration::from_secs(10));

    for size in [1024, 64 * 1024, 1024 * 1024].iter() {
        let size = *size;
        group.throughput(Throughput::Bytes(size as u64));

        // Benchmark with pre-allocated buffer (optimized)
        group.bench_with_input(
            BenchmarkId::new("write_preallocated", size),
            &size,
            |b, &s| {
                let hot_dir = TempDir::new().unwrap();
                let cold_dir = TempDir::new().unwrap();
                let store = LocalFilesystemStore::with_options(
                    hot_dir.path().to_path_buf(),
                    cold_dir.path().to_path_buf(),
                    false,
                    false,
                );
                rt.block_on(async { store.init().await.unwrap() });

                b.to_async(&rt).iter_custom(|iters| {
                    let store = &store;
                    // Pre-allocate buffer once
                    let mut data = vec![0u8; s];
                    async move {
                        let mut total_duration = Duration::default();
                        for i in 0..iters {
                            // Reuse buffer, just modify first bytes
                            let prefix = i.to_le_bytes();
                            for (j, byte) in prefix.iter().enumerate() {
                                if j < data.len() {
                                    data[j] = *byte;
                                }
                            }

                            let start = std::time::Instant::now();
                            let data_clone = data.clone();
                            let reader = Box::pin(std::io::Cursor::new(data_clone));
                            store.write(reader, StorageClass::Hot).await.unwrap();
                            total_duration += start.elapsed();
                        }
                        total_duration
                    }
                })
            },
        );

        // Benchmark with new allocation each time (baseline)
        group.bench_with_input(
            BenchmarkId::new("write_new_alloc", size),
            &size,
            |b, &s| {
                let hot_dir = TempDir::new().unwrap();
                let cold_dir = TempDir::new().unwrap();
                let store = LocalFilesystemStore::with_options(
                    hot_dir.path().to_path_buf(),
                    cold_dir.path().to_path_buf(),
                    false,
                    false,
                );
                rt.block_on(async { store.init().await.unwrap() });

                b.to_async(&rt).iter_custom(|iters| {
                    let store = &store;
                    async move {
                        let mut total_duration = Duration::default();
                        for i in 0..iters {
                            // New allocation each time
                            let mut data = vec![0u8; s];
                            let prefix = i.to_le_bytes();
                            for (j, byte) in prefix.iter().enumerate() {
                                if j < data.len() {
                                    data[j] = *byte;
                                }
                            }

                            let start = std::time::Instant::now();
                            let reader = Box::pin(std::io::Cursor::new(data));
                            store.write(reader, StorageClass::Hot).await.unwrap();
                            total_duration += start.elapsed();
                        }
                        total_duration
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark read operations with different buffer allocation strategies
fn read_allocation_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("read_allocation_patterns");
    group.measurement_time(Duration::from_secs(10));

    for size in [1024, 64 * 1024, 1024 * 1024].iter() {
        let size = *size;
        group.throughput(Throughput::Bytes(size as u64));

        // Benchmark read with pre-allocated buffer
        group.bench_with_input(
            BenchmarkId::new("read_preallocated", size),
            &size,
            |b, &s| {
                let hot_dir = TempDir::new().unwrap();
                let cold_dir = TempDir::new().unwrap();
                let store = LocalFilesystemStore::with_options(
                    hot_dir.path().to_path_buf(),
                    cold_dir.path().to_path_buf(),
                    false,
                    false,
                );

                // Prepare data
                let hash = rt.block_on(async {
                    store.init().await.unwrap();
                    let data = vec![0u8; s];
                    let reader = Box::pin(std::io::Cursor::new(data));
                    let (hash, _) = store.write(reader, StorageClass::Hot).await.unwrap();
                    hash
                });

                b.to_async(&rt).iter(|| {
                    let store = &store;
                    let hash = &hash;
                    async move {
                        // Pre-allocate buffer with known capacity
                        let mut buffer = Vec::with_capacity(s);
                        let mut reader = store.read(hash, StorageClass::Hot).await.unwrap();
                        tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut buffer)
                            .await
                            .unwrap();
                        std::hint::black_box(&buffer);
                    }
                })
            },
        );

        // Benchmark read with Vec::new() (causes reallocations)
        group.bench_with_input(
            BenchmarkId::new("read_vec_new", size),
            &size,
            |b, &s| {
                let hot_dir = TempDir::new().unwrap();
                let cold_dir = TempDir::new().unwrap();
                let store = LocalFilesystemStore::with_options(
                    hot_dir.path().to_path_buf(),
                    cold_dir.path().to_path_buf(),
                    false,
                    false,
                );

                // Prepare data
                let hash = rt.block_on(async {
                    store.init().await.unwrap();
                    let data = vec![0u8; s];
                    let reader = Box::pin(std::io::Cursor::new(data));
                    let (hash, _) = store.write(reader, StorageClass::Hot).await.unwrap();
                    hash
                });

                b.to_async(&rt).iter(|| {
                    let store = &store;
                    let hash = &hash;
                    async move {
                        // Vec::new() causes multiple reallocations
                        let mut buffer = Vec::new();
                        let mut reader = store.read(hash, StorageClass::Hot).await.unwrap();
                        tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut buffer)
                            .await
                            .unwrap();
                        std::hint::black_box(&buffer);
                    }
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, allocation_benchmarks, read_allocation_benchmarks);
criterion_main!(benches);
