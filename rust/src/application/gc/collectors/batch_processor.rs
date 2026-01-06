use std::future::Future;
use tracing::warn;

/// Configuration for batch processing operations.
///
/// This struct controls how items are processed in concurrent batches,
/// allowing fine-tuning of performance characteristics.
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// The number of items to process concurrently in each batch.
    ///
    /// Higher values increase parallelism but may overwhelm the system.
    /// Lower values reduce memory usage but may be slower.
    pub concurrent_batch_size: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            concurrent_batch_size: 10,
        }
    }
}

/// Result of processing a single item in a batch.
///
/// This struct contains both the original item and the result of processing it.
#[derive(Debug)]
pub struct BatchItemResult<T, R = ()> {
    /// The original item that was processed.
    pub item: T,
    /// The result of processing the item.
    pub result: R,
}

/// Processor for handling concurrent batch operations.
///
/// This utility provides a way to process collections of items concurrently
/// in configurable batch sizes, improving performance for I/O-bound operations
/// while maintaining control over resource usage.
///
/// # Examples
///
/// ```rust,ignore
/// use crate::application::gc::collectors::{BatchProcessor, BatchConfig};
///
/// let items = vec![1, 2, 3, 4, 5];
/// let config = BatchConfig::default();
///
/// let results = BatchProcessor::process_concurrent(
///     items,
///     &config,
///     |item| async move {
///         // Process item here
///         item * 2
///     }
/// ).await;
/// ```
pub struct BatchProcessor;

impl BatchProcessor {
    /// Process items in concurrent batches
    pub async fn process_concurrent<F, Fut, T, R>(
        items: Vec<T>,
        config: &BatchConfig,
        processor: F,
    ) -> Vec<BatchItemResult<T, R>>
    where
        F: Fn(T) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = R> + Send,
        T: Send + 'static + Clone,
        R: Send + 'static,
    {
        let mut results = Vec::new();

        // Process in chunks for better concurrency control
        for chunk in items.chunks(config.concurrent_batch_size) {
            let chunk_results = Self::process_chunk(chunk.to_vec(), processor.clone()).await;
            results.extend(chunk_results);
        }

        results
    }

    /// Process a single chunk concurrently
    async fn process_chunk<F, Fut, T, R>(chunk: Vec<T>, processor: F) -> Vec<BatchItemResult<T, R>>
    where
        F: Fn(T) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = R> + Send,
        T: Send + 'static + Clone,
        R: Send + 'static,
    {
        let mut handles = Vec::new();

        // Start concurrent tasks
        for item in chunk {
            let processor = processor.clone();
            let handle = tokio::spawn(async move {
                let result = processor(item.clone()).await;
                BatchItemResult { item, result }
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!("Batch processing task panicked: {}", e);
                    // For panicked tasks, we can't recover the result
                    // This is a rare case and should be logged
                }
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_process_concurrent_all_success() {
        let counter = Arc::new(AtomicUsize::new(0));
        let items = vec![1, 2, 3, 4, 5];

        let processor = {
            let counter = Arc::clone(&counter);
            move |item: i32| {
                let counter = Arc::clone(&counter);
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    true // success
                }
            }
        };

        let config = BatchConfig::default();
        let results = BatchProcessor::process_concurrent(items, &config, processor).await;

        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.result));
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn test_process_concurrent_mixed_results() {
        let items = vec![1, 2, 3, 4, 5];

        let processor = |item: i32| async move {
            // Fail for even numbers
            item % 2 != 0
        };

        let config = BatchConfig::default();
        let results = BatchProcessor::process_concurrent(items, &config, processor).await;

        assert_eq!(results.len(), 5);
        let successful_items: Vec<_> = results
            .iter()
            .filter(|r| r.result)
            .map(|r| r.item)
            .collect();
        let failed_items: Vec<_> = results
            .iter()
            .filter(|r| !r.result)
            .map(|r| r.item)
            .collect();

        assert_eq!(successful_items, vec![1, 3, 5]);
        assert_eq!(failed_items, vec![2, 4]);
    }

    #[tokio::test]
    async fn test_process_concurrent_empty_batch() {
        let items: Vec<i32> = vec![];

        let processor = |item: i32| async move {
            false // never called
        };

        let config = BatchConfig::default();
        let results = BatchProcessor::process_concurrent(items, &config, processor).await;

        assert_eq!(results.len(), 0);
    }
}
