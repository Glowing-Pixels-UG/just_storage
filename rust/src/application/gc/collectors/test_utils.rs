use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::application::ports::{
    BlobRepository, BlobStore, ObjectRepository, RepositoryError, StorageError,
};
use crate::domain::entities::Blob;
use crate::domain::value_objects::{ContentHash, StorageClass};

/// Mock blob repository for testing
pub struct MockBlobRepository {
    pub blobs: Mutex<Vec<Blob>>,
    pub deleted_hashes: Mutex<Vec<String>>,
    pub should_fail_find: bool,
    pub should_fail_delete: bool,
}

impl MockBlobRepository {
    pub fn new(blobs: Vec<Blob>) -> Self {
        Self {
            blobs: Mutex::new(blobs),
            deleted_hashes: Mutex::new(Vec::new()),
            should_fail_find: false,
            should_fail_delete: false,
        }
    }

    pub fn failing_find(blobs: Vec<Blob>) -> Self {
        Self {
            blobs: Mutex::new(blobs),
            deleted_hashes: Mutex::new(Vec::new()),
            should_fail_find: true,
            should_fail_delete: false,
        }
    }

    pub fn failing_delete(blobs: Vec<Blob>) -> Self {
        Self {
            blobs: Mutex::new(blobs),
            deleted_hashes: Mutex::new(Vec::new()),
            should_fail_find: false,
            should_fail_delete: true,
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
        unimplemented!("Not needed for GC collector tests")
    }

    async fn increment_ref(&self, _content_hash: &ContentHash) -> Result<(), RepositoryError> {
        unimplemented!("Not needed for GC collector tests")
    }

    async fn decrement_ref(&self, _content_hash: &ContentHash) -> Result<i32, RepositoryError> {
        unimplemented!("Not needed for GC collector tests")
    }

    async fn find_orphaned(&self, limit: i64) -> Result<Vec<Blob>, RepositoryError> {
        if self.should_fail_find {
            return Err(RepositoryError::Database(sqlx::Error::RowNotFound));
        }

        let blobs = self.blobs.lock().unwrap();
        let orphaned: Vec<Blob> = blobs
            .iter()
            .filter(|b| b.ref_count() == 0)
            .take(limit as usize)
            .cloned()
            .collect();
        Ok(orphaned)
    }

    async fn delete(&self, content_hash: &ContentHash) -> Result<(), RepositoryError> {
        if self.should_fail_delete {
            return Err(RepositoryError::Database(sqlx::Error::RowNotFound));
        }
        self.deleted_hashes
            .lock()
            .unwrap()
            .push(content_hash.to_string());
        Ok(())
    }
}

/// Mock blob store for testing
pub struct MockBlobStore {
    pub deleted_files: Mutex<Vec<String>>,
    pub should_fail_delete: bool,
}

impl MockBlobStore {
    pub fn new() -> Self {
        Self {
            deleted_files: Mutex::new(Vec::new()),
            should_fail_delete: false,
        }
    }

    pub fn failing() -> Self {
        Self {
            deleted_files: Mutex::new(Vec::new()),
            should_fail_delete: true,
        }
    }
}

#[async_trait]
impl BlobStore for MockBlobStore {
    async fn write(
        &self,
        _reader: crate::application::ports::BlobReader,
        _storage_class: StorageClass,
    ) -> Result<(ContentHash, u64), StorageError> {
        unimplemented!("Not needed for GC collector tests")
    }

    async fn read(
        &self,
        _content_hash: &ContentHash,
        _storage_class: StorageClass,
    ) -> Result<crate::application::ports::BlobReader, StorageError> {
        unimplemented!("Not needed for GC collector tests")
    }

    async fn delete(
        &self,
        content_hash: &ContentHash,
        _storage_class: StorageClass,
    ) -> Result<(), StorageError> {
        if self.should_fail_delete {
            return Err(StorageError::NotFound("blob not found".to_string()));
        }
        self.deleted_files
            .lock()
            .unwrap()
            .push(content_hash.to_string());
        Ok(())
    }

    async fn exists(
        &self,
        _content_hash: &ContentHash,
        _storage_class: StorageClass,
    ) -> Result<bool, StorageError> {
        unimplemented!("Not needed for GC collector tests")
    }
}

/// Mock object repository for testing
pub struct MockObjectRepository {
    pub cleanup_calls: Mutex<Vec<i64>>,
    pub cleanup_result: Mutex<Result<usize, RepositoryError>>,
}

impl MockObjectRepository {
    pub fn new(result: Result<usize, RepositoryError>) -> Self {
        Self {
            cleanup_calls: Mutex::new(Vec::new()),
            cleanup_result: Mutex::new(result),
        }
    }

    pub fn success(count: usize) -> Self {
        Self::new(Ok(count))
    }

    pub fn failure(error: RepositoryError) -> Self {
        Self::new(Err(error))
    }
}

#[async_trait]
impl ObjectRepository for MockObjectRepository {
    async fn cleanup_stuck_uploads(&self, age_hours: i64) -> Result<usize, RepositoryError> {
        self.cleanup_calls.lock().unwrap().push(age_hours);
        match *self.cleanup_result.lock().unwrap() {
            Ok(count) => Ok(count),
            Err(_) => Err(RepositoryError::Database(sqlx::Error::RowNotFound)),
        }
    }

    async fn save(&self, _object: &crate::domain::entities::Object) -> Result<(), RepositoryError> {
        unimplemented!("Not needed for GC collector tests")
    }

    async fn find_by_id(
        &self,
        _id: &crate::domain::value_objects::ObjectId,
    ) -> Result<Option<crate::domain::entities::Object>, RepositoryError> {
        unimplemented!("Not needed for GC collector tests")
    }

    async fn find_by_key(
        &self,
        _namespace: &crate::domain::value_objects::Namespace,
        _tenant_id: &crate::domain::value_objects::TenantId,
        _key: &str,
    ) -> Result<Option<crate::domain::entities::Object>, RepositoryError> {
        unimplemented!("Not needed for GC collector tests")
    }

    async fn list(
        &self,
        _namespace: &crate::domain::value_objects::Namespace,
        _tenant_id: &crate::domain::value_objects::TenantId,
        _limit: i64,
        _offset: i64,
    ) -> Result<Vec<crate::domain::entities::Object>, RepositoryError> {
        unimplemented!("Not needed for GC collector tests")
    }

    async fn search(&self, _request: &crate::application::dto::SearchRequest) -> Result<Vec<crate::domain::entities::Object>, RepositoryError> {
        unimplemented!("Not needed for GC collector tests")
    }

    async fn text_search(&self, _request: &crate::application::dto::TextSearchRequest) -> Result<Vec<crate::domain::entities::Object>, RepositoryError> {
        unimplemented!("Not needed for GC collector tests")
    }

    async fn delete(
        &self,
        _id: &crate::domain::value_objects::ObjectId,
    ) -> Result<(), RepositoryError> {
        unimplemented!("Not needed for GC collector tests")
    }

    async fn find_stuck_writing_objects(
        &self,
        _age_hours: i64,
        _limit: i64,
    ) -> Result<Vec<crate::domain::value_objects::ObjectId>, RepositoryError> {
        unimplemented!("Not needed for GC collector tests")
    }
}

/// Helper function to create a test blob
pub fn create_test_blob(content_hash_str: &str, ref_count: i32) -> Blob {
    Blob::new(
        ContentHash::from_hex(content_hash_str.to_string()).unwrap(),
        StorageClass::Hot,
        100,
    )
}

/// Helper function to create multiple test blobs
pub fn create_test_blobs(hashes_and_counts: Vec<(&str, i32)>) -> Vec<Blob> {
    hashes_and_counts
        .into_iter()
        .map(|(hash, count)| create_test_blob(hash, count))
        .collect()
}
