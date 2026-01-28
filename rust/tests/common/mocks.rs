#![allow(dead_code)]

//! In-memory mocks for tests

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;

use just_storage::application::ports::ObjectRepository;
use just_storage::application::ports::RepositoryError;
use just_storage::domain::entities::Object;
use just_storage::domain::value_objects::{Namespace, ObjectId, TenantId};

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
            map.insert(*obj.id(), obj);
        }
        Self {
            objects: Mutex::new(map),
        }
    }
}

#[async_trait]
impl ObjectRepository for InMemoryObjectRepository {
    async fn save(&self, object: &Object) -> Result<(), RepositoryError> {
        let mut objects = self.objects.lock().unwrap();
        objects.insert(*object.id(), object.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &ObjectId) -> Result<Option<Object>, RepositoryError> {
        let objects = self.objects.lock().unwrap();
        Ok(objects.get(id).cloned())
    }

    async fn find_by_key(
        &self,
        namespace: &Namespace,
        tenant_id: &TenantId,
        key: &str,
    ) -> Result<Option<Object>, RepositoryError> {
        let objects = self.objects.lock().unwrap();
        Ok(objects
            .values()
            .find(|obj| {
                obj.namespace() == namespace
                    && obj.tenant_id() == tenant_id
                    && obj.key().as_ref().map(|s| *s == key) == Some(true)
            })
            .cloned())
    }

    async fn list(
        &self,
        namespace: &Namespace,
        tenant_id: &TenantId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Object>, RepositoryError> {
        let objects = self.objects.lock().unwrap();
        let mut filtered: Vec<_> = objects
            .values()
            .filter(|obj| obj.namespace() == namespace && obj.tenant_id() == tenant_id)
            .cloned()
            .collect();

        filtered.sort_by_key(|a| a.created_at());
        let start = offset as usize;
        let end = (offset + limit) as usize;
        Ok(filtered.into_iter().skip(start).take(end - start).collect())
    }

    async fn search(
        &self,
        _request: &just_storage::application::dto::SearchRequest,
    ) -> Result<Vec<Object>, RepositoryError> {
        Ok(vec![])
    }

    async fn text_search(
        &self,
        _request: &just_storage::application::dto::TextSearchRequest,
    ) -> Result<Vec<Object>, RepositoryError> {
        Ok(vec![])
    }

    async fn delete(&self, id: &ObjectId) -> Result<(), RepositoryError> {
        let mut objects = self.objects.lock().unwrap();
        objects.remove(id);
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
