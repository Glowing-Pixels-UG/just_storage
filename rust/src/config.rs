use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub hot_storage_root: PathBuf,
    pub cold_storage_root: PathBuf,
    pub listen_addr: String,
    pub gc_interval_secs: u64,
    pub gc_batch_size: i64,
    // Database connection pool settings
    pub db_max_connections: u32,
    pub db_min_connections: u32,
    pub db_acquire_timeout_secs: u64,
    pub db_idle_timeout_secs: u64,
    pub db_max_lifetime_secs: u64,
    // Request limits
    pub max_upload_size_bytes: u64,
    // Performance tuning options
    pub adaptive_buffering_enabled: bool,
    pub concurrent_cache_threshold: usize,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://postgres:password@localhost/just_storage".to_string()
            }),
            hot_storage_root: std::env::var("HOT_STORAGE_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/data/hot")),
            cold_storage_root: std::env::var("COLD_STORAGE_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/data/cold")),
            listen_addr: {
                // Support PORT environment variable for PaaS platforms (Heroku, Fly.io, Railway, etc.)
                let port = std::env::var("PORT")
                    .ok()
                    .or_else(|| std::env::var("LISTEN_ADDR").ok())
                    .unwrap_or_else(|| "8080".to_string());

                // If PORT is set, use it; otherwise check if LISTEN_ADDR has full format
                if std::env::var("PORT").is_ok() {
                    format!("0.0.0.0:{}", port)
                } else if std::env::var("LISTEN_ADDR").is_ok() {
                    std::env::var("LISTEN_ADDR").unwrap()
                } else {
                    format!("0.0.0.0:{}", port)
                }
            },
            gc_interval_secs: std::env::var("GC_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            gc_batch_size: std::env::var("GC_BATCH_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            // Database pool settings with sensible defaults
            // max_connections: Typically 2 * CPU cores + effective_spindle_count
            // For most applications, 10-20 is a good starting point
            db_max_connections: std::env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),
            db_min_connections: std::env::var("DB_MIN_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            db_acquire_timeout_secs: std::env::var("DB_ACQUIRE_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            db_idle_timeout_secs: std::env::var("DB_IDLE_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(600), // 10 minutes
            db_max_lifetime_secs: std::env::var("DB_MAX_LIFETIME_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1800), // 30 minutes
            // Request size limits (default: 10GB)
            max_upload_size_bytes: std::env::var("MAX_UPLOAD_SIZE_BYTES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10 * 1024 * 1024 * 1024), // 10 GB
            // Performance tuning (adaptive features enabled by default)
            adaptive_buffering_enabled: std::env::var("ADAPTIVE_BUFFERING_ENABLED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
            concurrent_cache_threshold: std::env::var("CONCURRENT_CACHE_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10), // Switch to concurrent cache after 10 concurrent ops
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

        // Validate storage paths exist or can be created
        if let Some(parent) = self.hot_storage_root.parent() {
            if !parent.exists() {
                return Err(format!(
                    "HOT_STORAGE_ROOT parent directory does not exist: {}",
                    parent.display()
                ));
            }
        }

        if let Some(parent) = self.cold_storage_root.parent() {
            if !parent.exists() {
                return Err(format!(
                    "COLD_STORAGE_ROOT parent directory does not exist: {}",
                    parent.display()
                ));
            }
        }

        // Validate GC settings
        if self.gc_interval_secs < 10 {
            return Err("GC_INTERVAL_SECS must be at least 10 seconds".to_string());
        }

        if self.gc_batch_size < 1 || self.gc_batch_size > 1000 {
            return Err("GC_BATCH_SIZE must be between 1 and 1000".to_string());
        }

        // Validate database pool settings
        if self.db_max_connections < self.db_min_connections {
            return Err("DB_MAX_CONNECTIONS must be >= DB_MIN_CONNECTIONS".to_string());
        }

        if self.db_max_connections == 0 {
            return Err("DB_MAX_CONNECTIONS must be > 0".to_string());
        }

        if self.db_acquire_timeout_secs == 0 {
            return Err("DB_ACQUIRE_TIMEOUT_SECS must be > 0".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn with_env_var<F, R>(key: &str, value: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        env::set_var(key, value);
        let result = f();
        env::remove_var(key);
        result
    }

    #[test]
    fn test_default_config_values() {
        let config = Config::from_env();

        // Test default values when no env vars are set
        assert!(config.database_url.contains("just_storage"));
        assert_eq!(config.hot_storage_root, PathBuf::from("/data/hot"));
        assert_eq!(config.cold_storage_root, PathBuf::from("/data/cold"));
        assert_eq!(config.gc_interval_secs, 300);
        assert_eq!(config.gc_batch_size, 100);
        assert_eq!(config.db_max_connections, 10);
        assert_eq!(config.db_min_connections, 1);
        assert_eq!(config.db_acquire_timeout_secs, 30);
        assert_eq!(config.db_idle_timeout_secs, 300);
        assert_eq!(config.db_max_lifetime_secs, 1800);
        assert_eq!(config.max_upload_size_bytes, 100 * 1024 * 1024);
        assert!(config.adaptive_buffering_enabled);
        assert_eq!(config.concurrent_cache_threshold, 1024 * 1024);
    }

    #[test]
    fn test_env_var_override_database_url() {
        with_env_var(
            "DATABASE_URL",
            "postgres://test:test@localhost/testdb",
            || {
                let config = Config::from_env();
                assert_eq!(config.database_url, "postgres://test:test@localhost/testdb");
            },
        );
    }

    #[test]
    fn test_env_var_override_storage_roots() {
        with_env_var("HOT_STORAGE_ROOT", "/tmp/hot", || {
            with_env_var("COLD_STORAGE_ROOT", "/tmp/cold", || {
                let config = Config::from_env();
                assert_eq!(config.hot_storage_root, PathBuf::from("/tmp/hot"));
                assert_eq!(config.cold_storage_root, PathBuf::from("/tmp/cold"));
            });
        });
    }

    #[test]
    fn test_env_var_override_listen_addr() {
        with_env_var("LISTEN_ADDR", "127.0.0.1:9000", || {
            let config = Config::from_env();
            assert_eq!(config.listen_addr, "127.0.0.1:9000");
        });
    }

    #[test]
    fn test_env_var_override_port() {
        with_env_var("PORT", "9001", || {
            let config = Config::from_env();
            assert_eq!(config.listen_addr, "0.0.0.0:9001");
        });
    }

    #[test]
    fn test_port_takes_precedence_over_listen_addr() {
        with_env_var("LISTEN_ADDR", "127.0.0.1:9000", || {
            with_env_var("PORT", "9001", || {
                let config = Config::from_env();
                assert_eq!(config.listen_addr, "0.0.0.0:9001");
            });
        });
    }

    #[test]
    fn test_env_var_override_gc_settings() {
        with_env_var("GC_INTERVAL_SECS", "600", || {
            with_env_var("GC_BATCH_SIZE", "200", || {
                let config = Config::from_env();
                assert_eq!(config.gc_interval_secs, 600);
                assert_eq!(config.gc_batch_size, 200);
            });
        });
    }

    #[test]
    fn test_env_var_override_db_settings() {
        with_env_var("DB_MAX_CONNECTIONS", "20", || {
            with_env_var("DB_MIN_CONNECTIONS", "5", || {
                with_env_var("DB_ACQUIRE_TIMEOUT_SECS", "60", || {
                    with_env_var("DB_IDLE_TIMEOUT_SECS", "600", || {
                        with_env_var("DB_MAX_LIFETIME_SECS", "3600", || {
                            let config = Config::from_env();
                            assert_eq!(config.db_max_connections, 20);
                            assert_eq!(config.db_min_connections, 5);
                            assert_eq!(config.db_acquire_timeout_secs, 60);
                            assert_eq!(config.db_idle_timeout_secs, 600);
                            assert_eq!(config.db_max_lifetime_secs, 3600);
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn test_env_var_override_performance_settings() {
        with_env_var("ADAPTIVE_BUFFERING_ENABLED", "false", || {
            with_env_var("CONCURRENT_CACHE_THRESHOLD", "2048", || {
                let config = Config::from_env();
                assert!(!config.adaptive_buffering_enabled);
                assert_eq!(config.concurrent_cache_threshold, 2048);
            });
        });
    }

    #[test]
    fn test_config_validation_success() {
        let config = Config::from_env();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_database_url_required() {
        let mut config = Config::from_env();
        config.database_url = String::new();
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("DATABASE_URL"));
    }

    #[test]
    fn test_config_validation_storage_roots_required() {
        let mut config = Config::from_env();
        config.hot_storage_root = PathBuf::new();
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("HOT_STORAGE_ROOT"));

        let mut config = Config::from_env();
        config.cold_storage_root = PathBuf::new();
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("COLD_STORAGE_ROOT"));
    }

    #[test]
    fn test_config_validation_gc_settings() {
        let mut config = Config::from_env();
        config.gc_interval_secs = 0;
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("GC_INTERVAL_SECS"));

        let mut config = Config::from_env();
        config.gc_batch_size = 0;
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("GC_BATCH_SIZE"));
    }

    #[test]
    fn test_config_validation_db_settings() {
        let mut config = Config::from_env();
        config.db_max_connections = 0;
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("DB_MAX_CONNECTIONS"));

        let mut config = Config::from_env();
        config.db_min_connections = 0;
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("DB_MIN_CONNECTIONS"));

        let mut config = Config::from_env();
        config.db_acquire_timeout_secs = 0;
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("DB_ACQUIRE_TIMEOUT_SECS"));
    }

    #[test]
    fn test_config_validation_upload_size() {
        let mut config = Config::from_env();
        config.max_upload_size_bytes = 0;
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("MAX_UPLOAD_SIZE_BYTES"));
    }

    #[test]
    fn test_config_clone() {
        let config = Config::from_env();
        let cloned = config.clone();
        assert_eq!(config.database_url, cloned.database_url);
        assert_eq!(config.hot_storage_root, cloned.hot_storage_root);
        assert_eq!(config.listen_addr, cloned.listen_addr);
    }
}
