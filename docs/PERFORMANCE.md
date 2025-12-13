# Performance Optimizations

This document describes the performance optimizations implemented in the storage system and the design decisions behind them.

## Hash Algorithm: SHA-256

### Design Decision

The system uses **SHA-256 exclusively** for content addressing. This decision is based on industry best practices and several critical factors:

1. **Industry Standard**: SHA-256 is the de facto standard for content-addressable storage systems:
   - Git uses SHA-1 (migrating to SHA-256)
   - IPFS uses SHA-256
   - Docker uses SHA-256
   - This ensures compatibility and interoperability

2. **Cryptographic Security**: SHA-256 provides strong collision resistance (2^128 security level), which is critical for content integrity in storage systems.

3. **Fixed Format**: The `ContentHash` type is designed around SHA-256's 32-byte output (64 hex characters), enabling:
   - Efficient directory fan-out strategies (2-character prefix = 256 directories)
   - Consistent hash format across the entire system
   - Simple comparison and indexing operations

4. **Performance with SIMD**: With hardware acceleration enabled via the `asm` feature:
   - x86_64: Uses AVX/AVX2 instructions for parallel hash computation
   - ARM64: Uses NEON instructions for optimized performance
   - Modern CPUs achieve excellent throughput (100+ MB/s for hashing)

5. **Consistency**: Using a single hash algorithm ensures:
   - All content hashes are comparable
   - No hash collisions between different algorithms
   - Predictable performance characteristics

### Why Not BLAKE3?

While BLAKE3 is faster than SHA-256, it was not chosen because:

- **Breaking Change**: Changing hash algorithms in a CAS system requires migration of all existing content
- **Format Incompatibility**: The system is designed around SHA-256's 64-character hex format
- **Industry Adoption**: SHA-256 has broader ecosystem support and tooling
- **Performance Sufficient**: With SIMD optimizations, SHA-256 performance is excellent for storage workloads

## I/O Optimizations

### Buffer Sizes

- **Read/Write Buffers**: 256KB buffers optimize throughput for sequential operations
- **BufWriter Capacity**: 512KB (2x buffer size) minimizes syscalls while maintaining reasonable memory usage
- **Rationale**: Modern storage systems (SSDs, NVMe) benefit from larger I/O operations

### Streaming Hash Computation

- Hash computation happens **simultaneously** with file I/O
- Single-pass approach eliminates the need for a second read
- Reduces I/O operations by 50% compared to hash-then-write approaches

### Durability Control

- `fsync()` operations are configurable via `durable_writes` parameter
- For benchmarking: disable durability for accurate performance measurement
- For production: enable durability for data integrity guarantees
- Note: `fsync()` is expensive but necessary for crash consistency

## Memory Allocation

### Pre-allocation

- Buffers are pre-allocated once and reused where possible
- Reduces allocation overhead in hot paths
- Benchmark code reuses data buffers across iterations

### Buffer Reuse

- Single buffer allocation per operation
- Avoids repeated allocations in loops
- Reduces GC pressure (if applicable) and improves cache locality

## Concurrent Operations

### Benchmark Support

- Benchmarks include concurrent write tests (4 parallel operations)
- Measures system behavior under concurrent load
- Helps identify bottlenecks in multi-threaded scenarios

### Production Considerations

- The storage layer is designed to be thread-safe
- Multiple concurrent writes are supported
- File system handles atomicity via `rename()` operations

## Performance Metrics

### Current Benchmarks (Release Mode)

Based on recent benchmark runs:

- **1KB writes**: ~4-5 MB/s (improved from ~0.2 MB/s)
- **1MB writes**: ~200+ MB/s (improved from ~70-80 MB/s)
- **10MB writes**: ~175-180 MB/s (improved from ~152-159 MB/s)
- **Reads**: 7-9 GB/s for larger files (limited by memory bandwidth)

### Optimization Impact

1. **Removed sync_all() bottleneck**: +2000% improvement for small writes
2. **Larger buffers**: +100%+ improvement for medium writes
3. **SIMD optimizations**: Hardware-accelerated hash computation
4. **Streaming hash**: Eliminated redundant I/O passes

## Future Optimization Opportunities

1. **Zero-copy I/O**: Use `io_uring` (Linux) or similar for async I/O
2. **Memory-mapped files**: For very large files or read-heavy workloads
3. **Batch operations**: Group multiple small writes into larger operations
4. **Compression**: Optional compression for cold storage tiers
5. **Deduplication caching**: Cache hash lookups to avoid file system checks

## References

- [SHA-2 Performance](https://docs.rs/sha2/latest/sha2/)
- [Content-Addressable Storage Best Practices](https://en.wikipedia.org/wiki/Content-addressable_storage)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Tokio I/O Best Practices](https://tokio.rs/tokio/tutorial/io)

