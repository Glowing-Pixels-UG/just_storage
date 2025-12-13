use chrono::Utc;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use futures_util::future::join_all;
use just_storage::application::ports::BlobStore;
use just_storage::domain::value_objects::StorageClass;
use just_storage::infrastructure::storage::LocalFilesystemStore;
use std::io::Write;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;

fn storage_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("storage_operations");

    // Create CSV file for historical data
    let mut csv_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("benchmark_history.csv")
        .expect("Failed to open benchmark history file");

    if csv_file.metadata().unwrap().len() == 0 {
        writeln!(
            csv_file,
            "timestamp,operation,size_bytes,throughput_mb_s,avg_time_ms"
        )
        .unwrap();
    }

    for size in [1024, 1024 * 1024, 10 * 1024 * 1024].iter() {
        let size = *size;
        group.throughput(Throughput::Bytes(size as u64));
        group.measurement_time(Duration::from_secs(10));

        // Benchmark Write
        group.bench_with_input(BenchmarkId::new("write", size), &size, |b, &s| {
            // Setup store once
            let hot_dir = TempDir::new().unwrap();
            let cold_dir = TempDir::new().unwrap();
            let store = LocalFilesystemStore::with_options(
                hot_dir.path().to_path_buf(),
                cold_dir.path().to_path_buf(),
                false, // Disable durability for benchmarking performance
                false, // Disable directory pre-creation for faster benchmark startup
            );
            rt.block_on(async { store.init().await.unwrap() });

            b.to_async(&rt).iter_custom(|iters| {
                let store = &store;
                // Pre-allocate buffer once to reduce allocations
                let mut data = vec![0u8; s];
                async move {
                    let mut total_duration = Duration::default();
                    for i in 0..iters {
                        // Reuse buffer, just modify first bytes for uniqueness
                        let prefix = i.to_le_bytes();
                        for (j, byte) in prefix.iter().enumerate() {
                            if j < data.len() {
                                data[j] = *byte;
                            }
                        }

                        let start = std::time::Instant::now();
                        // Clone data for the reader (required for ownership)
                        let data_clone = data.clone();
                        let reader = Box::pin(std::io::Cursor::new(data_clone));
                        store.write(reader, StorageClass::Hot).await.unwrap();
                        total_duration += start.elapsed();
                    }
                    total_duration
                }
            })
        });

        // Benchmark Concurrent Writes
        group.bench_with_input(BenchmarkId::new("write_concurrent_4", size), &size, |b, &s| {
            // Setup store once
            let hot_dir = TempDir::new().unwrap();
            let cold_dir = TempDir::new().unwrap();
            let store = LocalFilesystemStore::with_options(
                hot_dir.path().to_path_buf(),
                cold_dir.path().to_path_buf(),
                false, // Disable durability for benchmarking performance
                false, // Disable directory pre-creation for faster benchmark startup
            );
            rt.block_on(async { store.init().await.unwrap() });

            b.to_async(&rt).iter_custom(|iters| {
                let store = &store;
                async move {
                    let mut total_duration = Duration::default();
                    for i in 0..iters {
                        let start = std::time::Instant::now();

                        // Perform 4 concurrent writes
                        let futures = (0..4).map(|thread_id| {
                            let store = &store;
                            let size = s;
                            let iter = i;
                            async move {
                                // Create unique content to avoid deduplication
                                let mut data = vec![0u8; size];
                                // Modify first few bytes with thread and iteration info
                                let prefix = ((iter * 4 + thread_id) as u32).to_le_bytes();
                                for (j, byte) in prefix.iter().enumerate() {
                                    if j < data.len() {
                                        data[j] = *byte;
                                    }
                                }

                                let reader = Box::pin(std::io::Cursor::new(data));
                                store.write(reader, StorageClass::Hot).await.unwrap();
                            }
                        });

                        // Wait for all concurrent writes to complete
                        join_all(futures).await;

                        total_duration += start.elapsed();
                    }
                    total_duration
                }
            })
        });

        // Benchmark Read
        group.bench_with_input(BenchmarkId::new("read", size), &size, |b, &s| {
            // Setup once per iteration
            let hot_dir = TempDir::new().unwrap();
            let cold_dir = TempDir::new().unwrap();
            let store = LocalFilesystemStore::with_options(
                hot_dir.path().to_path_buf(),
                cold_dir.path().to_path_buf(),
                false, // Disable durability for benchmarking performance
                false, // Disable directory pre-creation for faster benchmark startup
            );
            let data = vec![0u8; s];

            // Prepare data (outside of measurement)
            let (hash, _) = rt.block_on(async {
                store.init().await.unwrap();
                let reader = Box::pin(std::io::Cursor::new(data.clone()));
                store.write(reader, StorageClass::Hot).await.unwrap()
            });

            b.to_async(&rt).iter(|| async {
                // Pre-allocate buffer to avoid reallocations during read
                let mut buffer = Vec::with_capacity(s);
                let mut reader = store.read(&hash, StorageClass::Hot).await.unwrap();
                tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut buffer)
                    .await
                    .unwrap();
                // Ensure we actually read the data (prevent optimization)
                std::hint::black_box(&buffer);
            })
        });


        // Let's implement a manual measurement for CSV logging purposes alongside Criterion
        // This ensures we satisfy "benchmark with historical data save in csv"

        // Run a quick check for CSV logging
        let timestamp = Utc::now().to_rfc3339();

        // Create shared store for CSV logging to avoid repeated directory creation
        let hot_dir = TempDir::new().unwrap();
        let cold_dir = TempDir::new().unwrap();
        let store = LocalFilesystemStore::with_options(
            hot_dir.path().to_path_buf(),
            cold_dir.path().to_path_buf(),
            false, // Disable durability for benchmarking performance
            false, // Disable directory pre-creation for faster benchmark startup
        );
        rt.block_on(async {
            store.init().await.unwrap();
        });

        // Write Test for CSV
        let start = std::time::Instant::now();
        rt.block_on(async {
            let data = vec![0u8; size];
            let reader = Box::pin(std::io::Cursor::new(data));
            store.write(reader, StorageClass::Hot).await.unwrap();
        });
        let duration = start.elapsed();
        let mb_s = (size as f64 / 1_000_000.0) / duration.as_secs_f64();
        writeln!(
            csv_file,
            "{},write_{},{},{:.2},{:.2}",
            timestamp,
            size,
            size,
            mb_s,
            duration.as_millis()
        )
        .unwrap();

        // Read Test for CSV - reuse the same data we just wrote
        let hash = rt.block_on(async {
            let data = vec![0u8; size];
            let reader = Box::pin(std::io::Cursor::new(data));
            let (hash, _) = store.write(reader, StorageClass::Hot).await.unwrap();
            hash
        });

        let start = std::time::Instant::now();
        rt.block_on(async {
            let mut buffer = Vec::with_capacity(size);
            let mut reader = store.read(&hash, StorageClass::Hot).await.unwrap();
            tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut buffer)
                .await
                .unwrap();
        });
        let duration = start.elapsed();
        let mb_s = (size as f64 / 1_000_000.0) / duration.as_secs_f64();
        writeln!(
            csv_file,
            "{},read_{},{},{:.2},{:.2}",
            timestamp,
            size,
            size,
            mb_s,
            duration.as_millis()
        )
        .unwrap();
    }
    group.finish();
}

criterion_group!(benches, storage_benchmarks);
criterion_main!(benches);
