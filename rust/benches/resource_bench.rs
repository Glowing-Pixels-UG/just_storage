/// Resource usage benchmarks - Database, HTTP, and system resources
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Benchmark database connection pool efficiency
/// This measures the overhead of connection acquisition and release
fn database_pool_benchmarks(c: &mut Criterion) {
    // Note: This benchmark requires a running database
    // Skip if DATABASE_URL is not set
    if std::env::var("DATABASE_URL").is_err() {
        eprintln!("Skipping database benchmarks: DATABASE_URL not set");
        return;
    }

    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("database_pool");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark connection acquisition with different pool sizes
    for max_connections in [5, 10, 20, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("acquire_connection", max_connections),
            max_connections,
            |b, &max| {
                let db_url = std::env::var("DATABASE_URL").unwrap();
                b.to_async(&rt).iter_custom(|iters| {
                    let db_url = db_url.clone();
                    async move {
                        use sqlx::postgres::PgPoolOptions;
                        let pool = PgPoolOptions::new()
                            .max_connections(max)
                            .acquire_timeout(Duration::from_secs(30))
                            .connect(&db_url)
                            .await
                            .unwrap();

                        let start = std::time::Instant::now();
                        for _ in 0..iters {
                            let _conn = pool.acquire().await.unwrap();
                            // Connection is automatically returned to pool on drop
                        }
                        start.elapsed()
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark file descriptor usage
fn file_descriptor_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("file_descriptors");
    group.measurement_time(Duration::from_secs(10));

    for concurrent_ops in [1, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_file_ops", concurrent_ops),
            concurrent_ops,
            |b, &concurrent| {
                let temp_dir = TempDir::new().unwrap();
                let temp_path = temp_dir.path().to_path_buf();
                b.to_async(&rt).iter_custom(|iters| {
                    let temp_dir = temp_path.clone();
                    async move {
                        use futures_util::future::join_all;
                        use tokio::fs;
                        use tokio::io::AsyncWriteExt;

                        let start = std::time::Instant::now();
                        let iterations = (iters / concurrent as u64).max(1);
                        for _ in 0..iterations {
                            let futures: Vec<_> = (0..concurrent)
                                .map(|i| {
                                    let path = temp_dir.join(format!("file_{}.tmp", i));
                                    async move {
                                        let mut file = fs::File::create(&path).await.unwrap();
                                        file.write_all(b"test data").await.unwrap();
                                        file.sync_all().await.unwrap();
                                        fs::remove_file(&path).await.unwrap();
                                    }
                                })
                                .collect();
                            join_all(futures).await;
                        }
                        start.elapsed()
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark memory allocation patterns for different buffer sizes
fn buffer_allocation_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_allocation");
    group.measurement_time(Duration::from_secs(5));

    for size in [1024, 64 * 1024, 256 * 1024, 1024 * 1024].iter() {
        let size = *size;

        // Benchmark Vec::new() - multiple reallocations
        group.bench_with_input(BenchmarkId::new("vec_new", size), &size, |b, &s| {
            b.iter(|| {
                let mut vec = Vec::new();
                for i in 0..(s / 8) {
                    vec.push(i as u64);
                }
                std::hint::black_box(vec)
            })
        });

        // Benchmark Vec::with_capacity() - single allocation
        group.bench_with_input(
            BenchmarkId::new("vec_with_capacity", size),
            &size,
            |b, &s| {
                b.iter(|| {
                    let mut vec = Vec::with_capacity(s / 8);
                    for i in 0..(s / 8) {
                        vec.push(i as u64);
                    }
                    std::hint::black_box(vec)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark Arc cloning overhead
fn arc_cloning_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("arc_cloning");
    group.measurement_time(Duration::from_secs(5));

    // Benchmark Arc::clone() vs Arc::new()
    group.bench_function("arc_clone", |b| {
        let arc = std::sync::Arc::new(vec![0u8; 1024]);
        b.iter(|| {
            let _cloned = std::sync::Arc::clone(&arc);
            std::hint::black_box(_cloned)
        })
    });

    group.bench_function("arc_new", |b| {
        let data = vec![0u8; 1024];
        b.iter(|| {
            let _arc = std::sync::Arc::new(data.clone());
            std::hint::black_box(_arc)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    buffer_allocation_benchmarks,
    arc_cloning_benchmarks,
    file_descriptor_benchmarks,
    database_pool_benchmarks
);
criterion_main!(benches);
