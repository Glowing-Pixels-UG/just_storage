use std::time::Duration;

/// Configuration for garbage collection operations
#[derive(Debug, Clone)]
pub struct GcConfig {
    /// How often to run garbage collection
    pub interval: Duration,
    /// Number of items to process in each batch
    pub batch_size: i64,
    /// Age threshold for considering uploads "stuck" (in hours)
    pub stuck_upload_age_hours: i64,
    /// How often to run stuck upload cleanup (relative to main interval)
    pub stuck_upload_cleanup_multiplier: u32,
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(300), // 5 minutes
            batch_size: 100,
            stuck_upload_age_hours: 1,
            stuck_upload_cleanup_multiplier: 10, // Run stuck upload cleanup 10x less frequently
        }
    }
}

impl GcConfig {
    pub fn new(interval: Duration, batch_size: i64, stuck_upload_age_hours: i64) -> Self {
        Self {
            interval,
            batch_size,
            stuck_upload_age_hours,
            stuck_upload_cleanup_multiplier: 10,
        }
    }

    /// Calculate the stuck upload cleanup interval
    pub fn stuck_upload_cleanup_interval(&self) -> Duration {
        self.interval * self.stuck_upload_cleanup_multiplier
    }
}
