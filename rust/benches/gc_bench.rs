use async_trait::async_trait;
use chrono::Utc;
/// GC worker performance benchmarks
/// Measures garbage collection performance with different batch sizes and blob counts
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use just_storage::application::gc::worker::GarbageCollector;
use just_storage::application::ports::{BlobRepository, BlobStore, RepositoryError, StorageError};
use just_storage::domain::entities::Blob;
use just_storage::domain::value_objects::{ContentHash, StorageClass};
use just_storage::infrastructure::storage::LocalFilesystemStore;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

// Mock blob repository for GC benchmarks
struct MockBlobRepository {
    blobs: Mutex<Vec<Blob>>,
}

impl MockBlobRepository {
    fn new_with_orphans(count: usize) -> Self {
        let mut blobs = Vec::new();
        for i in 0..count {
            let hash_hex = format!("{:064x}", i);
            let content_hash = ContentHash::from_hex(hash_hex.clone()).unwrap();
            // Create orphaned blob (ref_count = 0)
            let blob = Blob::reconstruct(
                content_hash,
                StorageClass::Hot,
                1024,
                0, // ref_count = 0 means orphaned
                Utc::now(),
            );
            blobs.push(blob);
        }
        Self {
            blobs: Mutex::new(blobs),
        }
    }
}

#[async_trait]
impl BlobRepository for MockBlobRepository {
    async fn get_or_create(
        &self,
        _content_hash: &ContentHash,
        _storage_class: StorageClass,
        _size_bytes: u64,
    ) -> Result<Blob, RepositoryError> {
        unreachable!("Not used in GC benchmarks")
    }

    async fn increment_ref(&self, _content_hash: &ContentHash) -> Result<(), RepositoryError> {
        unreachable!("Not used in GC benchmarks")
    }

    async fn decrement_ref(&self, _content_hash: &ContentHash) -> Result<i32, RepositoryError> {
        unreachable!("Not used in GC benchmarks")
    }

    async fn find_orphaned(&self, limit: i64) -> Result<Vec<Blob>, RepositoryError> {
        let blobs = self.blobs.lock().await;
        let orphaned: Vec<Blob> = blobs
            .iter()
            .filter(|b| b.can_gc())
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(orphaned)
    }

    async fn delete(&self, content_hash: &ContentHash) -> Result<(), RepositoryError> {
        let mut blobs = self.blobs.lock().await;
        blobs.retain(|b| b.content_hash() != content_hash);
        Ok(())
    }
}

// Mock blob store that tracks deletions
struct MockBlobStore {
    blobs: Mutex<HashMap<String, Vec<u8>>>,
    delete_count: Mutex<usize>,
}

impl MockBlobStore {
    fn new() -> Self {
        Self {
            blobs: Mutex::new(HashMap::new()),
            delete_count: Mutex::new(0),
        }
    }

    async fn setup_blobs(&self, count: usize) {
        let mut blobs = self.blobs.lock().await;
        for i in 0..count {
            let hash_hex = format!("{:064x}", i);
            blobs.insert(hash_hex, vec![0u8; 1024]);
        }
    }

    #[allow(dead_code)]
    async fn get_delete_count(&self) -> usize {
        *self.delete_count.lock().await
    }
}

#[async_trait]
impl BlobStore for MockBlobStore {
    async fn write(
        &self,
        mut reader: just_storage::application::ports::BlobReader,
        _storage_class: StorageClass,
    ) -> Result<(ContentHash, u64), StorageError> {
        use tokio::io::AsyncReadExt;
        let mut data = Vec::new();
        reader
            .read_to_end(&mut data)
            .await
            .map_err(StorageError::Io)?;
        let hash = ContentHash::from_hex(
            "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        )
        .unwrap();
        Ok((hash, data.len() as u64))
    }

    async fn read(
        &self,
        _content_hash: &ContentHash,
        _storage_class: StorageClass,
    ) -> Result<just_storage::application::ports::BlobReader, StorageError> {
        Err(StorageError::NotFound("not used".to_string()))
    }

    async fn delete(
        &self,
        content_hash: &ContentHash,
        _storage_class: StorageClass,
    ) -> Result<(), StorageError> {
        let mut blobs = self.blobs.lock().await;
        blobs.remove(content_hash.as_hex());
        let mut count = self.delete_count.lock().await;
        *count += 1;
        Ok(())
    }

    async fn exists(
        &self,
        content_hash: &ContentHash,
        _storage_class: StorageClass,
    ) -> Result<bool, StorageError> {
        let blobs = self.blobs.lock().await;
        Ok(blobs.contains_key(content_hash.as_hex()))
    }
}

fn gc_worker_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("gc_worker");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark GC with different batch sizes
    for batch_size in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("collect_once", batch_size),
            batch_size,
            |b, &batch| {
                b.to_async(&rt).iter_custom(|iters| async move {
                    let mut total_duration = Duration::default();
                    for _ in 0..iters {
                        let repo = Arc::new(MockBlobRepository::new_with_orphans(1000));
                        let store = Arc::new(MockBlobStore::new());
                        store.setup_blobs(1000).await;

                        let gc = GarbageCollector::new(
                            repo,
                            store,
                            Duration::from_secs(60),
                            batch as i64,
                        );

                        let start = std::time::Instant::now();
                        let _ = gc.collect_once().await;
                        total_duration += start.elapsed();
                    }
                    total_duration
                })
            },
        );
    }

    // Benchmark GC with different orphan counts
    for orphan_count in [100, 500, 1000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::new("collect_orphans", orphan_count),
            orphan_count,
            |b, &count| {
                b.to_async(&rt).iter_custom(|iters| {
                    async move {
                        let mut total_duration = Duration::default();
                        for _ in 0..iters {
                            let repo = Arc::new(MockBlobRepository::new_with_orphans(count));
                            let store = Arc::new(MockBlobStore::new());
                            store.setup_blobs(count).await;

                            let gc = GarbageCollector::new(
                                repo,
                                store,
                                Duration::from_secs(60),
                                100, // Fixed batch size
                            );

                            let start = std::time::Instant::now();
                            let _ = gc.collect_once().await;
                            total_duration += start.elapsed();
                        }
                        total_duration
                    }
                })
            },
        );
    }

    // Benchmark GC with real filesystem store (smaller scale)
    group.bench_function("collect_once_filesystem", |b| {
        b.to_async(&rt).iter_custom(|iters| {
            async move {
                let mut total_duration = Duration::default();
                for _ in 0..iters {
                    let hot_dir = TempDir::new().unwrap();
                    let cold_dir = TempDir::new().unwrap();
                    let store = Arc::new(LocalFilesystemStore::with_options(
                        hot_dir.path().to_path_buf(),
                        cold_dir.path().to_path_buf(),
                        false, // Disable durability for benchmarking
                        false,
                    ));
                    store.init().await.unwrap();

                    // Create some orphaned blobs
                    let repo = Arc::new(MockBlobRepository::new_with_orphans(50));

                    // Write actual files for the orphaned blobs
                    {
                        let blobs = repo.find_orphaned(50).await.unwrap();
                        for blob in blobs {
                            let data = vec![0u8; 1024];
                            let reader = Box::pin(std::io::Cursor::new(data));
                            let _ = store.write(reader, blob.storage_class()).await;
                        }
                    }

                    let gc = GarbageCollector::new(repo, store, Duration::from_secs(60), 50);

                    let start = std::time::Instant::now();
                    let _ = gc.collect_once().await;
                    total_duration += start.elapsed();
                }
                total_duration
            }
        })
    });

    group.finish();
}

criterion_group!(benches, gc_worker_benchmarks);
criterion_main!(benches);
