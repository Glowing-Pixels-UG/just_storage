/// Result types and utilities for garbage collection operations
///
/// This module contains all the result types returned by GC operations,
/// along with their associated methods and utilities.
/// Result of a complete garbage collection cycle
#[derive(Debug, Default, Clone)]
pub struct GcResult {
    /// Total number of items deleted across all collectors.
    ///
    /// This is automatically calculated as the sum of all specific deletion counts.
    pub total_deleted: usize,
    /// Number of orphaned blobs that were successfully deleted.
    pub orphaned_blobs_deleted: usize,
    /// Number of stuck uploads that were successfully cleaned up.
    pub stuck_uploads_deleted: usize,
    /// Any errors that occurred during the collection process.
    ///
    /// Each error represents a failure in one of the collectors. The collection
    /// process continues even if individual collectors fail.
    pub errors: Vec<String>,
}

impl GcResult {
    /// Creates a new empty GC result
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if the collection completed without any errors.
    ///
    /// Note that this indicates the collection process itself succeeded,
    /// not necessarily that any items were deleted.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// if result.is_success() {
    ///     println!("All collectors completed successfully");
    /// } else {
    ///     println!("Some collectors failed: {:?}", result.errors);
    /// }
    /// ```
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns true if any items were deleted during the collection cycle.
    ///
    /// This is useful for determining if the garbage collection had any effect.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// if result.has_deletions() {
    ///     println!("Reclaimed space: {} items deleted", result.total_deleted);
    /// } else {
    ///     println!("No garbage found to clean up");
    /// }
    /// ```
    pub fn has_deletions(&self) -> bool {
        self.total_deleted > 0
    }

    /// Returns true if orphaned blobs were deleted
    pub fn has_orphaned_blob_deletions(&self) -> bool {
        self.orphaned_blobs_deleted > 0
    }

    /// Returns true if stuck uploads were cleaned up
    pub fn has_stuck_upload_cleanups(&self) -> bool {
        self.stuck_uploads_deleted > 0
    }

    /// Adds an error to the result
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    /// Merges another GcResult into this one
    pub fn merge(&mut self, other: GcResult) {
        self.total_deleted += other.total_deleted;
        self.orphaned_blobs_deleted += other.orphaned_blobs_deleted;
        self.stuck_uploads_deleted += other.stuck_uploads_deleted;
        self.errors.extend(other.errors);
    }

    /// Returns a summary of the collection results as a formatted string
    pub fn summary(&self) -> String {
        if self.errors.is_empty() {
            format!(
                "GC completed successfully: {} total deleted ({} orphaned blobs, {} stuck uploads)",
                self.total_deleted, self.orphaned_blobs_deleted, self.stuck_uploads_deleted
            )
        } else {
            format!(
                "GC completed with {} errors: {} total deleted ({} orphaned blobs, {} stuck uploads)",
                self.errors.len(), self.total_deleted, self.orphaned_blobs_deleted, self.stuck_uploads_deleted
            )
        }
    }

    /// Returns detailed information about the collection results
    pub fn details(&self) -> String {
        let mut details = vec![
            format!("Total items deleted: {}", self.total_deleted),
            format!("Orphaned blobs deleted: {}", self.orphaned_blobs_deleted),
            format!("Stuck uploads cleaned: {}", self.stuck_uploads_deleted),
            format!("Errors encountered: {}", self.errors.len()),
        ];

        if !self.errors.is_empty() {
            details.push("Errors:".to_string());
            for (i, error) in self.errors.iter().enumerate() {
                details.push(format!("  {}. {}", i + 1, error));
            }
        }

        details.join("\n")
    }
}

/// Summary statistics for GC operations
#[derive(Debug, Clone, Default)]
pub struct GcStatistics {
    /// Total number of collection cycles run
    pub cycles_completed: usize,
    /// Total items deleted across all cycles
    pub total_items_deleted: usize,
    /// Total orphaned blobs deleted
    pub total_orphaned_blobs_deleted: usize,
    /// Total stuck uploads cleaned
    pub total_stuck_uploads_cleaned: usize,
    /// Total errors encountered
    pub total_errors: usize,
    /// Average items deleted per cycle
    pub average_deletions_per_cycle: f64,
}

impl GcStatistics {
    /// Updates statistics with a new GC result
    pub fn update(&mut self, result: &GcResult) {
        self.cycles_completed += 1;
        self.total_items_deleted += result.total_deleted;
        self.total_orphaned_blobs_deleted += result.orphaned_blobs_deleted;
        self.total_stuck_uploads_cleaned += result.stuck_uploads_deleted;
        self.total_errors += result.errors.len();

        if self.cycles_completed > 0 {
            self.average_deletions_per_cycle =
                self.total_items_deleted as f64 / self.cycles_completed as f64;
        }
    }

    /// Returns a formatted summary of the statistics
    pub fn summary(&self) -> String {
        format!(
            "GC Statistics:\n\
             Cycles completed: {}\n\
             Total items deleted: {}\n\
             Orphaned blobs: {}\n\
             Stuck uploads: {}\n\
             Total errors: {}\n\
             Average deletions/cycle: {:.2}",
            self.cycles_completed,
            self.total_items_deleted,
            self.total_orphaned_blobs_deleted,
            self.total_stuck_uploads_cleaned,
            self.total_errors,
            self.average_deletions_per_cycle
        )
    }

    /// Resets all statistics to zero
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gc_result_is_success() {
        let mut result = GcResult::new();
        assert!(result.is_success());

        result.add_error("Test error");
        assert!(!result.is_success());
    }

    #[test]
    fn test_gc_result_has_deletions() {
        let mut result = GcResult::new();
        assert!(!result.has_deletions());

        result.orphaned_blobs_deleted = 5;
        assert!(result.has_deletions());
    }

    #[test]
    fn test_gc_result_merge() {
        let mut result1 = GcResult {
            total_deleted: 5,
            orphaned_blobs_deleted: 3,
            stuck_uploads_deleted: 2,
            errors: vec!["error1".to_string()],
        };

        let result2 = GcResult {
            total_deleted: 3,
            orphaned_blobs_deleted: 2,
            stuck_uploads_deleted: 1,
            errors: vec!["error2".to_string()],
        };

        result1.merge(result2);

        assert_eq!(result1.total_deleted, 8);
        assert_eq!(result1.orphaned_blobs_deleted, 5);
        assert_eq!(result1.stuck_uploads_deleted, 3);
        assert_eq!(result1.errors.len(), 2);
    }

    #[test]
    fn test_gc_result_summary() {
        let result = GcResult {
            total_deleted: 10,
            orphaned_blobs_deleted: 7,
            stuck_uploads_deleted: 3,
            errors: vec![],
        };

        let summary = result.summary();
        assert!(summary.contains("10 total deleted"));
        assert!(summary.contains("7 orphaned blobs"));
        assert!(summary.contains("3 stuck uploads"));
    }

    #[test]
    fn test_gc_statistics_update() {
        let mut stats = GcStatistics::default();

        let result = GcResult {
            total_deleted: 5,
            orphaned_blobs_deleted: 3,
            stuck_uploads_deleted: 2,
            errors: vec!["error".to_string()],
        };

        stats.update(&result);

        assert_eq!(stats.cycles_completed, 1);
        assert_eq!(stats.total_items_deleted, 5);
        assert_eq!(stats.total_orphaned_blobs_deleted, 3);
        assert_eq!(stats.total_stuck_uploads_cleaned, 2);
        assert_eq!(stats.total_errors, 1);
        assert_eq!(stats.average_deletions_per_cycle, 5.0);
    }
}
