use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub hot_storage_root: PathBuf,
    pub cold_storage_root: PathBuf,
    pub listen_addr: String,
    pub gc_interval_secs: u64,
    pub gc_batch_size: i64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:password@localhost/activestorage".to_string()
            }),
            hot_storage_root: std::env::var("HOT_STORAGE_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/data/hot")),
            cold_storage_root: std::env::var("COLD_STORAGE_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/data/cold")),
            listen_addr: std::env::var("LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            gc_interval_secs: std::env::var("GC_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            gc_batch_size: std::env::var("GC_BATCH_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate database URL format
        if !self.database_url.starts_with("postgres://")
            && !self.database_url.starts_with("postgresql://")
        {
            return Err("DATABASE_URL must start with postgres:// or postgresql://".to_string());
        }

        // Validate listen address
        if self.listen_addr.is_empty() {
            return Err("LISTEN_ADDR cannot be empty".to_string());
        }

        // Validate GC settings
        if self.gc_interval_secs < 10 {
            return Err("GC_INTERVAL_SECS must be at least 10 seconds".to_string());
        }

        if self.gc_batch_size < 1 || self.gc_batch_size > 1000 {
            return Err("GC_BATCH_SIZE must be between 1 and 1000".to_string());
        }

        Ok(())
    }
}
