//! Shared test fixtures and utilities for all test types
//!
//! This module provides common test setup patterns to reduce duplication
//! and make tests more maintainable.

use std::sync::Arc;
use std::path::PathBuf;
use tempfile::TempDir;
use sqlx::{PgPool, Executor};
use uuid::Uuid;

use just_storage::domain::entities::{Object, Blob};
use just_storage::domain::value_objects::{
    ContentHash, StorageClass, Namespace, TenantId, ObjectId
};
use just_storage::application::ports::{
    BlobRepository, BlobStore, ObjectRepository, ApiKeyRepository, AuditRepository
};
use just_storage::infrastructure::{
    persistence::{PostgresBlobRepository, PostgresObjectRepository},
    storage::LocalFilesystemStore,
};

/// Test environment container with all necessary components
pub struct TestEnvironment {
    pub pool: PgPool,
    pub object_repo: Arc<dyn ObjectRepository>,
    pub blob_repo: Arc<dyn BlobRepository>,
    pub blob_store: Arc<dyn BlobStore>,
    pub hot_dir: TempDir,
    pub cold_dir: TempDir,
    pub api_key_repo: Option<Arc<dyn ApiKeyRepository>>,
    pub audit_repo: Option<Arc<dyn AuditRepository>>,
}

impl TestEnvironment {
    /// Create a complete test environment with database and storage
    pub async fn new() -> Self {
        let pool = setup_test_database().await;
        let (hot_dir, cold_dir) = setup_test_storage();

        let object_repo: Arc<dyn ObjectRepository> =
            Arc::new(PostgresObjectRepository::new(pool.clone()));
        let blob_repo: Arc<dyn BlobRepository> =
            Arc::new(PostgresBlobRepository::new(pool.clone()));

        let store = LocalFilesystemStore::new(
            hot_dir.path().to_path_buf(),
            cold_dir.path().to_path_buf(),
        );
        store.init().await.expect("Failed to init storage");
        let blob_store: Arc<dyn BlobStore> = Arc::new(store);

        Self {
            pool,
            object_repo,
            blob_repo,
            blob_store,
            hot_dir,
            cold_dir,
            api_key_repo: None,
            audit_repo: None,
        }
    }

    /// Add API key repository to the environment
    pub fn with_api_key_repo(mut self, repo: Arc<dyn ApiKeyRepository>) -> Self {
        self.api_key_repo = Some(repo);
        self
    }

    /// Add audit repository to the environment
    pub fn with_audit_repo(mut self, repo: Arc<dyn AuditRepository>) -> Self {
        self.audit_repo = Some(repo);
        self
    }
}

/// Database setup utilities
pub async fn setup_test_database() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost:5432/test".to_string());

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations/schema setup
    setup_schema(&pool).await;
    cleanup_test_data(&pool).await;

    pool
}

/// Setup database schema from SQL file
async fn setup_schema(pool: &PgPool) {
    let schema = include_str!("../../schema.sql");
    let statements: Vec<&str> = schema
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with("--"))
        .collect();

    for statement in statements {
        if !statement.trim().is_empty() {
            pool.execute(statement)
                .await
                .unwrap_or_else(|e| panic!("Failed to execute schema statement: {}\nStatement: {}", e, statement));
        }
    }
}

/// Clean up test data between tests
pub async fn cleanup_test_data(pool: &PgPool) {
    sqlx::query("DELETE FROM audit_logs").execute(pool).await.ok();
    sqlx::query("DELETE FROM api_keys").execute(pool).await.ok();
    sqlx::query("DELETE FROM objects").execute(pool).await.ok();
    sqlx::query("DELETE FROM blobs").execute(pool).await.ok();
}

/// Storage setup utilities
pub fn setup_test_storage() -> (TempDir, TempDir) {
    let hot_dir = TempDir::new().expect("Failed to create temp hot storage dir");
    let cold_dir = TempDir::new().expect("Failed to create temp cold storage dir");
    (hot_dir, cold_dir)
}

/// Test data factories
pub mod factories {
    use super::*;
    use just_storage::domain::entities::Object;
    use just_storage::domain::value_objects::{
        Namespace, TenantId, ObjectId, ObjectStatus
    };

    /// Create a test object with default values
    pub fn create_test_object() -> Object {
        let mut obj = Object::new(
            Namespace::new("test".to_string()).unwrap(),
            TenantId::new(Uuid::new_v4()),
            Some("test_key".to_string()),
            StorageClass::Hot,
        );
        // Commit the object to set content hash and size
        let content_hash = ContentHash::from_hex("testhash12345678901234567890123456789012".to_string()).unwrap();
        obj.commit(content_hash, 1024).unwrap();
        obj.set_content_type("application/json".to_string());
        // Metadata is already initialized with default values
        obj
    }

    /// Create a test object with custom parameters
    pub fn create_custom_object(
        namespace: &str,
        tenant_id: &str,
        key: Option<&str>,
        storage_class: StorageClass,
        status: just_storage::domain::value_objects::ObjectStatus,
        content_hash: &str,
        size_bytes: Option<u64>,
    ) -> Object {
        let uuid = Uuid::parse_str(tenant_id).unwrap_or_else(|_| Uuid::new_v4());
        let mut obj = Object::new(
            Namespace::new(namespace.to_string()).unwrap(),
            TenantId::new(uuid),
            key.map(|s| s.to_string()),
            storage_class,
        );

        // If the object should be committed, commit it with the provided hash and size
        if status == just_storage::domain::value_objects::ObjectStatus::Committed {
            if let Some(size) = size_bytes {
                let hash = ContentHash::from_hex(content_hash.to_string()).unwrap();
                obj.commit(hash, size).unwrap();
            }
        }

        obj.set_content_type("application/octet-stream".to_string());
        obj
    }

    /// Create a test blob
    pub fn create_test_blob(content_hash: &ContentHash, storage_class: StorageClass) -> Blob {
        Blob::new(
            content_hash.clone(),
            storage_class,
            1024, // size_bytes
        )
    }
}

/// HTTP testing utilities
pub mod http {
    use axum::body::Body;
    use axum::http::{Request, Method, Uri};
    use serde_json::Value;

    /// Create a JSON request body
    pub fn json_body(data: Value) -> Body {
        Body::from(serde_json::to_string(&data).unwrap())
    }

    /// Create a GET request
    pub fn get_request(uri: &str) -> Request<Body> {
        Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    /// Create a POST request with JSON body
    pub fn post_request(uri: &str, data: Value) -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header("content-type", "application/json")
            .body(json_body(data))
            .unwrap()
    }

    /// Create a PUT request with JSON body
    pub fn put_request(uri: &str, data: Value) -> Request<Body> {
        Request::builder()
            .method(Method::PUT)
            .uri(uri)
            .header("content-type", "application/json")
            .body(json_body(data))
            .unwrap()
    }

    /// Create a DELETE request
    pub fn delete_request(uri: &str) -> Request<Body> {
        Request::builder()
            .method(Method::DELETE)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    /// Create an authenticated request
    pub fn authenticated_request(method: Method, uri: &str, api_key: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("authorization", format!("Bearer {}", api_key))
            .body(Body::empty())
            .unwrap()
    }

    /// Create an authenticated request with JSON body
    pub fn authenticated_json_request(method: Method, uri: &str, api_key: &str, data: Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("authorization", format!("Bearer {}", api_key))
            .header("content-type", "application/json")
            .body(json_body(data))
            .unwrap()
    }
}

/// Mock implementations for unit testing
pub mod mocks {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// In-memory object repository for testing
    pub struct InMemoryObjectRepository {
        objects: Mutex<HashMap<ObjectId, Object>>,
    }

    impl InMemoryObjectRepository {
        pub fn new() -> Self {
            Self {
                objects: Mutex::new(HashMap::new()),
            }
        }

        pub fn with_objects(objects: Vec<Object>) -> Self {
            let mut map = HashMap::new();
            for obj in objects {
                map.insert(obj.id().clone(), obj);
            }
            Self {
                objects: Mutex::new(map),
            }
        }
    }

    #[async_trait]
    impl ObjectRepository for InMemoryObjectRepository {
        async fn save(&self, object: &Object) -> Result<(), just_storage::application::ports::RepositoryError> {
            let mut objects = self.objects.lock().unwrap();
            objects.insert(object.id().clone(), object.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: &ObjectId) -> Result<Option<Object>, just_storage::application::ports::RepositoryError> {
            let objects = self.objects.lock().unwrap();
            Ok(objects.get(id).cloned())
        }

        async fn find_by_key(
            &self,
            namespace: &Namespace,
            tenant_id: &TenantId,
            key: &str,
        ) -> Result<Option<Object>, just_storage::application::ports::RepositoryError> {
            let objects = self.objects.lock().unwrap();
            Ok(objects.values().find(|obj|
                obj.namespace() == namespace &&
                obj.tenant_id() == tenant_id &&
                obj.key().as_ref().map(|s| *s == key) == Some(true)
            ).cloned())
        }

        async fn list(
            &self,
            namespace: &Namespace,
            tenant_id: &TenantId,
            limit: i64,
            offset: i64,
        ) -> Result<Vec<Object>, just_storage::application::ports::RepositoryError> {
            let objects = self.objects.lock().unwrap();
            let mut filtered: Vec<_> = objects.values()
                .filter(|obj| obj.namespace() == namespace && obj.tenant_id() == tenant_id)
                .cloned()
                .collect();

            filtered.sort_by(|a, b| a.created_at().cmp(&b.created_at()));
            let start = offset as usize;
            let end = (offset + limit) as usize;
            Ok(filtered.into_iter().skip(start).take(end - start).collect())
        }

        async fn search(&self, _request: &just_storage::application::dto::SearchRequest) -> Result<Vec<Object>, just_storage::application::ports::RepositoryError> {
            // Simplified implementation for testing
            Ok(vec![])
        }

        async fn text_search(&self, _request: &just_storage::application::dto::TextSearchRequest) -> Result<Vec<Object>, just_storage::application::ports::RepositoryError> {
            // Simplified implementation for testing
            Ok(vec![])
        }

        async fn delete(&self, id: &ObjectId) -> Result<(), just_storage::application::ports::RepositoryError> {
            let mut objects = self.objects.lock().unwrap();
            objects.remove(id);
            Ok(())
        }

        async fn find_stuck_writing_objects(
            &self,
            _age_hours: i64,
            _limit: i64,
        ) -> Result<Vec<ObjectId>, just_storage::application::ports::RepositoryError> {
            // Return empty for testing
            Ok(vec![])
        }

        async fn cleanup_stuck_uploads(&self, _age_hours: i64) -> Result<usize, just_storage::application::ports::RepositoryError> {
            Ok(0)
        }
    }
}

/// Test assertions and helpers
pub mod assertions {
    use axum::http::StatusCode;
    use axum::body::to_bytes;
    use serde_json::Value;

    /// Assert that a response has the expected status code
    pub async fn assert_status(response: axum::response::Response, expected: StatusCode) {
        assert_eq!(response.status(), expected,
            "Expected status {}, got {}", expected, response.status());
    }

    /// Assert that a response contains JSON with expected structure
    pub async fn assert_json_response(response: axum::response::Response, expected_keys: &[&str]) {
        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body_bytes).unwrap();

        for key in expected_keys {
            assert!(json.get(key).is_some(),
                "Expected JSON response to contain key '{}'", key);
        }
    }

    /// Assert that a response contains an error message
    pub async fn assert_error_response(response: axum::response::Response, expected_status: StatusCode) {
        assert_eq!(response.status(), expected_status);
        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(json.get("error").is_some(), "Expected error response");
    }
}