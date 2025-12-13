use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use just_storage::infrastructure::storage::LocalFilesystemStore;
use just_storage::application::ports::BlobStore;
use just_storage::domain::value_objects::StorageClass;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use std::time::Duration;
use std::fs::File;
use std::io::Write;
use chrono::Utc;

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
        writeln!(csv_file, "timestamp,operation,size_bytes,throughput_mb_s,avg_time_ms").unwrap();
    }

    for size in [1024, 1024 * 1024, 10 * 1024 * 1024].iter() {
        let size = *size;
        group.throughput(Throughput::Bytes(size as u64));
        group.measurement_time(Duration::from_secs(10));

        // Benchmark Write
        group.bench_with_input(BenchmarkId::new("write", size), &size, |b, &s| {
            b.to_async(&rt).iter(|| async {
                let hot_dir = TempDir::new().unwrap();
                let cold_dir = TempDir::new().unwrap();
                let store = LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());
                store.init().await.unwrap();

                let data = vec![0u8; s];
                let reader = Box::pin(std::io::Cursor::new(data));
                store.write(reader, StorageClass::Hot).await.unwrap();
            })
        });

        // Benchmark Read
        group.bench_with_input(BenchmarkId::new("read", size), &size, |b, &s| {
             // Setup once per iteration
            let hot_dir = TempDir::new().unwrap();
            let cold_dir = TempDir::new().unwrap();
            let store = LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());
            let data = vec![0u8; s];

            // Prepare data (outside of measurement)
            let (hash, _) = rt.block_on(async {
                store.init().await.unwrap();
                let reader = Box::pin(std::io::Cursor::new(data.clone()));
                store.write(reader, StorageClass::Hot).await.unwrap()
            });

            b.to_async(&rt).iter(|| async {
                let _reader = store.read(&hash, StorageClass::Hot).await.unwrap();
                // Note: we're not reading the content here to strictly measure open/access overhead + IO setup,
                // but usually reading the content is part of the benchmark.
                // However, `BlobReader` is just a Box<dyn AsyncRead>, so just getting it might not read data.
                // Let's read it to be fair.
                let mut buffer = Vec::with_capacity(s);
                let mut reader = store.read(&hash, StorageClass::Hot).await.unwrap();
                tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut buffer).await.unwrap();
            })
        });

        // Manual "hook" to save data to CSV is tricky with standard Criterion API because it handles reporting.
        // However, we can access the latest report directory or use a custom reporter.
        // For simplicity in this task, we will just note that Criterion saves JSON/HTML.
        // But the requirement is "save in csv".
        // We can't easily extract the exact result *during* the run without a custom reporter.
        // A workaround is to parse the generated JSON or use a custom runner.
        // Or simpler: run a separate manual timing loop for the CSV part, which duplicates work but satisfies the requirement simply.

        // Let's implement a manual measurement for CSV logging purposes alongside Criterion
        // This ensures we satisfy "benchmark with historical data save in csv"

        // Run a quick check for CSV logging
        let timestamp = Utc::now().to_rfc3339();

        // Write Test for CSV
        let start = std::time::Instant::now();
        rt.block_on(async {
            let hot_dir = TempDir::new().unwrap();
            let cold_dir = TempDir::new().unwrap();
            let store = LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());
            store.init().await.unwrap();
            let data = vec![0u8; size];
            let reader = Box::pin(std::io::Cursor::new(data));
            store.write(reader, StorageClass::Hot).await.unwrap();
        });
        let duration = start.elapsed();
        let mb_s = (size as f64 / 1_000_000.0) / duration.as_secs_f64();
        writeln!(csv_file, "{},write_{},{},{:.2},{:.2}", timestamp, size, size, mb_s, duration.as_millis()).unwrap();

        // Read Test for CSV
        // Need to keep TempDir alive
        let hot_dir = TempDir::new().unwrap();
        let cold_dir = TempDir::new().unwrap();
        let store = LocalFilesystemStore::new(hot_dir.path().to_path_buf(), cold_dir.path().to_path_buf());

        let hash = rt.block_on(async {
            store.init().await.unwrap();
            let data = vec![0u8; size];
            let reader = Box::pin(std::io::Cursor::new(data));
            let (hash, _) = store.write(reader, StorageClass::Hot).await.unwrap();
            hash
        });

        let start = std::time::Instant::now();
        rt.block_on(async {
            let mut buffer = Vec::with_capacity(size);
            let mut reader = store.read(&hash, StorageClass::Hot).await.unwrap();
            tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut buffer).await.unwrap();
        });
        let duration = start.elapsed();
         let mb_s = (size as f64 / 1_000_000.0) / duration.as_secs_f64();
        writeln!(csv_file, "{},read_{},{},{:.2},{:.2}", timestamp, size, size, mb_s, duration.as_millis()).unwrap();

    }
    group.finish();
}

criterion_group!(benches, storage_benchmarks);
criterion_main!(benches);
