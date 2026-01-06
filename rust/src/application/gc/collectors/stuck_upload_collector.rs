use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

use super::{collector::Collector, errors::GcResult};
use crate::application::ports::ObjectRepository;

/// Collector for stuck uploads (objects in WRITING state that are too old)
pub struct StuckUploadCollector {
    object_repo: Arc<dyn ObjectRepository>,
    stuck_upload_age_hours: i64,
}

#[async_trait]
impl Collector for StuckUploadCollector {
    fn name(&self) -> &'static str {
        "stuck_upload_collector"
    }

    async fn collect(&self) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        self.collect_internal()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

impl StuckUploadCollector {
    pub fn new(object_repo: Arc<dyn ObjectRepository>, stuck_upload_age_hours: i64) -> Self {
        Self {
            object_repo,
            stuck_upload_age_hours,
        }
    }

    /// Collect and cleanup stuck uploads (internal implementation)
    async fn collect_internal(&self) -> GcResult<usize> {
        let count = self
            .object_repo
            .cleanup_stuck_uploads(self.stuck_upload_age_hours)
            .await
            .map_err(|e| super::errors::GcError::DeletionError { source: e.into() })?;

        if count > 0 {
            info!("Cleaned up {} stuck WRITING objects", count);
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::gc::collectors::test_utils::MockObjectRepository;
    use crate::application::ports::RepositoryError;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_collect_successful_cleanup() {
        let mock_repo = Arc::new(MockObjectRepository::success(5));
        let collector = StuckUploadCollector::new(mock_repo.clone(), 24);

        let result = collector.collect().await.unwrap();
        assert_eq!(result, 5);

        let calls = mock_repo.cleanup_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], 24);
    }

    #[tokio::test]
    async fn test_collect_no_stuck_uploads() {
        let mock_repo = Arc::new(MockObjectRepository::success(0));
        let collector = StuckUploadCollector::new(mock_repo, 24);

        let result = collector.collect().await.unwrap();
        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn test_collect_cleanup_error() {
        let mock_repo = Arc::new(MockObjectRepository::failure(RepositoryError::Database(
            sqlx::Error::RowNotFound,
        )));
        let collector = StuckUploadCollector::new(mock_repo, 24);

        let result = collector.collect().await;
        assert!(result.is_err());
    }
}
