use async_trait::async_trait;
/// HTTP handler benchmarks
/// Measures end-to-end handler performance including middleware
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use just_storage::application::dto::{SearchRequest, TextSearchRequest};
use just_storage::application::ports::{
    BlobRepository, BlobStore, ObjectRepository, RepositoryError,
};
use just_storage::application::use_cases::{
    DownloadObjectUseCase, ListObjectsUseCase, UploadObjectUseCase,
};
use just_storage::domain::entities::{Blob, Object};
use just_storage::domain::value_objects::{
    ContentHash, Namespace, ObjectId, StorageClass, TenantId,
};
use just_storage::infrastructure::storage::LocalFilesystemStore;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use uuid::Uuid;

// Mock implementations for benchmarking
struct MockObjectRepository {
    objects: Mutex<HashMap<String, Object>>,
}

impl MockObjectRepository {
    fn new() -> Self {
        Self {
            objects: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl ObjectRepository for MockObjectRepository {
    async fn save(&self, object: &Object) -> Result<(), RepositoryError> {
        let mut objects = self.objects.lock().await;
        objects.insert(object.id().to_string(), object.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &ObjectId) -> Result<Option<Object>, RepositoryError> {
        let objects = self.objects.lock().await;
        Ok(objects.get(&id.to_string()).cloned())
    }

    async fn find_by_key(
        &self,
        _namespace: &Namespace,
        _tenant_id: &TenantId,
        _key: &str,
    ) -> Result<Option<Object>, RepositoryError> {
        Ok(None)
    }

    async fn list(
        &self,
        _namespace: &Namespace,
        _tenant_id: &TenantId,
        _limit: i64,
        _offset: i64,
    ) -> Result<Vec<Object>, RepositoryError> {
        Ok(vec![])
    }

    async fn search(&self, _request: &SearchRequest) -> Result<Vec<Object>, RepositoryError> {
        Ok(vec![])
    }

    async fn text_search(
        &self,
        _request: &TextSearchRequest,
    ) -> Result<Vec<Object>, RepositoryError> {
        Ok(vec![])
    }

    async fn delete(&self, _id: &ObjectId) -> Result<(), RepositoryError> {
        Ok(())
    }

    async fn find_stuck_writing_objects(
        &self,
        _age_hours: i64,
        _limit: i64,
    ) -> Result<Vec<ObjectId>, RepositoryError> {
        Ok(vec![])
    }

    async fn cleanup_stuck_uploads(&self, _age_hours: i64) -> Result<usize, RepositoryError> {
        Ok(0)
    }
}

struct MockBlobRepository {
    blobs: Mutex<HashMap<String, Blob>>,
}

impl MockBlobRepository {
    fn new() -> Self {
        Self {
            blobs: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl BlobRepository for MockBlobRepository {
    async fn get_or_create(
        &self,
        content_hash: &ContentHash,
        storage_class: StorageClass,
        size_bytes: u64,
    ) -> Result<Blob, RepositoryError> {
        let mut blobs = self.blobs.lock().await;
        let key = format!("{}_{:?}", content_hash.as_hex(), storage_class);

        if let Some(blob) = blobs.get_mut(&key) {
            blob.increment_ref();
            return Ok(blob.clone());
        }

        let blob = Blob::new(content_hash.clone(), storage_class, size_bytes);
        blobs.insert(key, blob.clone());
        Ok(blob)
    }

    async fn increment_ref(&self, _content_hash: &ContentHash) -> Result<(), RepositoryError> {
        Ok(())
    }

    async fn decrement_ref(&self, _content_hash: &ContentHash) -> Result<i32, RepositoryError> {
        Ok(0)
    }

    async fn find_orphaned(&self, _limit: i64) -> Result<Vec<Blob>, RepositoryError> {
        Ok(vec![])
    }

    async fn delete(&self, _content_hash: &ContentHash) -> Result<(), RepositoryError> {
        Ok(())
    }
}

fn http_handler_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("http_handlers");
    group.measurement_time(Duration::from_secs(10));

    for size in [1024, 64 * 1024, 1024 * 1024].iter() {
        let size = *size;
        group.throughput(Throughput::Bytes(size as u64));

        // Benchmark upload handler (use case execution)
        group.bench_with_input(BenchmarkId::new("upload_handler", size), &size, |b, &s| {
            let hot_dir = TempDir::new().unwrap();
            let cold_dir = TempDir::new().unwrap();
            let store = Arc::new(LocalFilesystemStore::with_options(
                hot_dir.path().to_path_buf(),
                cold_dir.path().to_path_buf(),
                false, // Disable durability for benchmarking
                false, // Disable pre-creation for faster startup
            ));
            let object_repo = Arc::new(MockObjectRepository::new());
            let blob_repo = Arc::new(MockBlobRepository::new());

            rt.block_on(async {
                store.init().await.unwrap();
            });

            let use_case = Arc::new(UploadObjectUseCase::new(object_repo, blob_repo, store));

            b.to_async(&rt).iter_custom(|iters| {
                let use_case = Arc::clone(&use_case);
                async move {
                    let mut total_duration = Duration::default();
                    for i in 0..iters {
                        let mut data = vec![0u8; s];
                        // Make each iteration unique
                        let prefix = i.to_le_bytes();
                        for (j, byte) in prefix.iter().enumerate() {
                            if j < data.len() {
                                data[j] = *byte;
                            }
                        }

                        let start = std::time::Instant::now();
                        let reader = Box::pin(Cursor::new(data));
                        let request = just_storage::application::dto::UploadRequest {
                            namespace: "test".to_string(),
                            tenant_id: Uuid::new_v4().to_string(),
                            key: Some(format!("key_{}", i)),
                            storage_class: Some(StorageClass::Hot),
                        };

                        let _ = use_case.execute(request, reader).await;
                        total_duration += start.elapsed();
                    }
                    total_duration
                }
            })
        });

        // Benchmark download handler (use case execution)
        group.bench_with_input(
            BenchmarkId::new("download_handler", size),
            &size,
            |b, &s| {
                let hot_dir = TempDir::new().unwrap();
                let cold_dir = TempDir::new().unwrap();
                let store = Arc::new(LocalFilesystemStore::with_options(
                    hot_dir.path().to_path_buf(),
                    cold_dir.path().to_path_buf(),
                    false,
                    false,
                ));

                // Prepare data
                let (object_id, object_repo) = rt.block_on(async {
                    store.init().await.unwrap();
                    let data = vec![0u8; s];
                    let reader = Box::pin(Cursor::new(data));
                    let (hash, _) = store.write(reader, StorageClass::Hot).await.unwrap();

                    let namespace = Namespace::new("test".to_string()).unwrap();
                    let tenant_id = TenantId::new(Uuid::new_v4());
                    let mut object = Object::new(
                        namespace,
                        tenant_id,
                        Some("key1".to_string()),
                        StorageClass::Hot,
                    );
                    object.commit(hash.clone(), s as u64).unwrap();
                    let object_id = *object.id();

                    let object_repo = Arc::new(MockObjectRepository::new());
                    object_repo.save(&object).await.unwrap();

                    (object_id, object_repo)
                });

                let use_case = Arc::new(DownloadObjectUseCase::new(object_repo, store));

                b.to_async(&rt).iter(|| {
                    let use_case = Arc::clone(&use_case);
                    async move {
                        let (_, mut reader) = use_case.execute_by_id(&object_id).await.unwrap();
                        let mut buffer = Vec::with_capacity(s);
                        tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut buffer)
                            .await
                            .unwrap();
                        std::hint::black_box(&buffer);
                    }
                })
            },
        );
    }

    // Benchmark list handler
    group.bench_function("list_handler", |b| {
        let object_repo = Arc::new(MockObjectRepository::new());
        let use_case = Arc::new(ListObjectsUseCase::new(object_repo));

        b.to_async(&rt).iter(|| {
            let use_case = Arc::clone(&use_case);
            async move {
                let request = just_storage::application::dto::ListRequest {
                    namespace: "test".to_string(),
                    tenant_id: Uuid::new_v4().to_string(),
                    limit: Some(10),
                    offset: Some(0),
                };
                let _ = use_case.execute(request).await;
            }
        })
    });

    group.finish();
}

criterion_group!(benches, http_handler_benchmarks);
criterion_main!(benches);
