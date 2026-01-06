/// Common trait for garbage collection operations.
///
/// This trait defines the interface that all garbage collectors must implement.
/// It provides a standardized way to execute collection operations and identify
/// collectors for logging and monitoring purposes.
///
/// # Examples
///
/// ```rust,ignore
/// use async_trait::async_trait;
/// use crate::application::gc::collectors::Collector;
///
/// struct MyCollector;
///
/// #[async_trait]
/// impl Collector for MyCollector {
///     fn name(&self) -> &'static str {
///         "my_collector"
///     }
///
///     async fn collect(&self) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
///         // Perform collection logic here
///         Ok(42)
///     }
/// }
/// ```
#[async_trait::async_trait]
pub trait Collector {
    /// Returns the name of this collector for logging and identification.
    ///
    /// The name should be unique among all collectors and should be a valid
    /// identifier (snake_case recommended).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert_eq!(collector.name(), "orphaned_blob_collector");
    /// ```
    fn name(&self) -> &'static str;

    /// Performs one complete collection cycle.
    ///
    /// This method should execute all necessary logic to identify and clean up
    /// garbage items. The implementation should be idempotent and safe to call
    /// multiple times.
    ///
    /// # Returns
    ///
    /// Returns the number of items that were successfully cleaned up during
    /// this collection cycle.
    ///
    /// # Errors
    ///
    /// Returns an error if the collection process fails. Partial failures
    /// should be logged but not necessarily returned as errors unless the
    /// entire operation is compromised.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let cleaned_count = collector.collect().await?;
    /// println!("Cleaned up {} items", cleaned_count);
    /// ```
    async fn collect(&self) -> Result<usize, Box<dyn std::error::Error + Send + Sync>>;
}

/// Result of a single collection operation.
///
/// This struct encapsulates the outcome of running a collector, including
/// success/failure status and the number of items processed.
#[derive(Debug, Clone)]
pub struct CollectionResult {
    /// The name of the collector that produced this result.
    pub collector_name: String,
    /// The number of items that were successfully cleaned up.
    pub items_cleaned: usize,
    /// Any errors that occurred during the collection process.
    pub errors: Vec<String>,
}

impl CollectionResult {
    /// Creates a new collection result with no items cleaned and no errors.
    ///
    /// # Arguments
    ///
    /// * `collector_name` - The name of the collector.
    pub fn new(collector_name: impl Into<String>) -> Self {
        Self {
            collector_name: collector_name.into(),
            items_cleaned: 0,
            errors: Vec::new(),
        }
    }

    /// Creates a successful collection result.
    ///
    /// # Arguments
    ///
    /// * `collector_name` - The name of the collector.
    /// * `items_cleaned` - The number of items that were cleaned up.
    pub fn success(collector_name: impl Into<String>, items_cleaned: usize) -> Self {
        Self {
            collector_name: collector_name.into(),
            items_cleaned,
            errors: Vec::new(),
        }
    }

    /// Creates a failed collection result.
    ///
    /// # Arguments
    ///
    /// * `collector_name` - The name of the collector.
    /// * `error` - The error message describing what went wrong.
    pub fn error(collector_name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            collector_name: collector_name.into(),
            items_cleaned: 0,
            errors: vec![error.into()],
        }
    }

    /// Returns true if the collection completed without errors.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// if result.is_success() {
    ///     println!("Collection completed successfully");
    /// }
    /// ```
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns true if any items were cleaned up during the collection.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// if result.has_cleaned_items() {
    ///     println!("Cleaned up {} items", result.items_cleaned);
    /// }
    /// ```
    pub fn has_cleaned_items(&self) -> bool {
        self.items_cleaned > 0
    }
}
