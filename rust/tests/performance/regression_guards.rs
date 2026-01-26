/// Performance validation tests to prevent regressions
/// Run this with: cargo test --bench performance_validation
use just_storage::application::ports::BlobStore;
use just_storage::domain::value_objects::StorageClass;
use just_storage::infrastructure::storage::LocalFilesystemStore;
use std::time::Instant;
use tempfile::TempDir;
use tokio::runtime::Runtime;

#[test]
fn test_minimum_write_performance() {
    let rt = Runtime::new().unwrap();
    let hot_dir = TempDir::new().unwrap();
    let cold_dir = TempDir::new().unwrap();

    // Use optimized settings for performance testing
    let store = LocalFilesystemStore::with_options(
        hot_dir.path().to_path_buf(),
        cold_dir.path().to_path_buf(),
        false, // Disable durability for performance
        false, // Disable directory pre-creation
    );

    rt.block_on(async {
        store.init().await.unwrap();

        // Test 1KB writes
        let start = Instant::now();
        for _ in 0..100 {
            let data = vec![0u8; 1024];
            let reader = Box::pin(std::io::Cursor::new(data));
            store.write(reader, StorageClass::Hot).await.unwrap();
        }
        let duration = start.elapsed();
        let throughput_mb_s = (1024.0 * 100.0 / 1_000_000.0) / duration.as_secs_f64();

        // 1KB writes should be > 1 MB/s in debug mode (much better than the original ~0.2 MB/s)
        // In release mode this should be > 4 MB/s
        assert!(
            throughput_mb_s > 1.0,
            "Performance regression: 1KB write throughput {:.2} MB/s is below minimum threshold of 1.0 MB/s",
            throughput_mb_s
        );

        // Test 1MB writes
        let start = Instant::now();
        for _ in 0..10 {
            let data = vec![0u8; 1024 * 1024];
            let reader = Box::pin(std::io::Cursor::new(data));
            store.write(reader, StorageClass::Hot).await.unwrap();
        }
        let duration = start.elapsed();
        let throughput_mb_s = (1024.0 * 1024.0 * 10.0 / 1_000_000.0) / duration.as_secs_f64();

        // 1MB writes should be > 10 MB/s in debug mode
        // In release mode this should be > 100 MB/s
        assert!(
            throughput_mb_s > 10.0,
            "Performance regression: 1MB write throughput {:.2} MB/s is below minimum threshold of 10.0 MB/s",
            throughput_mb_s
        );
    });
}

#[test]
fn test_read_performance() {
    let rt = Runtime::new().unwrap();
    let hot_dir = TempDir::new().unwrap();
    let cold_dir = TempDir::new().unwrap();

    let store = LocalFilesystemStore::with_options(
        hot_dir.path().to_path_buf(),
        cold_dir.path().to_path_buf(),
        false, // Disable durability for performance
        false, // Disable directory pre-creation
    );

    rt.block_on(async {
        store.init().await.unwrap();

        // Prepare test data
        let data = vec![0u8; 1024 * 1024]; // 1MB
        let reader = Box::pin(std::io::Cursor::new(data.clone()));
        let (hash, _) = store.write(reader, StorageClass::Hot).await.unwrap();

        // Test reads
        let start = Instant::now();
        for _ in 0..10 {
            let mut reader = store.read(&hash, StorageClass::Hot).await.unwrap();
            let mut buffer = Vec::with_capacity(1024 * 1024);
            tokio::io::AsyncReadExt::read_to_end(&mut reader, &mut buffer).await.unwrap();
        }
        let duration = start.elapsed();
        let throughput_mb_s = (1024.0 * 1024.0 * 10.0 / 1_000_000.0) / duration.as_secs_f64();

        // 1MB reads should be > 50 MB/s in debug mode
        // In release mode this should be > 100 MB/s
        assert!(
            throughput_mb_s > 50.0,
            "Performance regression: 1MB read throughput {:.2} MB/s is below minimum threshold of 50.0 MB/s",
            throughput_mb_s
        );
    });
}
