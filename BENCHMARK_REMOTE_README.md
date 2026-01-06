# Benchmark Remote Host

Run JustStorage benchmarks on a remote host and compare with local results.

## Usage

```bash
# Basic usage with default settings
./benchmark-remote.sh

# With custom image tag and host
./benchmark-remote.sh v1.0.0 root@remote-server.com

# With custom output file
./benchmark-remote.sh latest root@remote-server.com custom_results.txt
```

## What it does

1. Copies Rust source code to remote host
2. Builds and runs all benchmarks on remote host
3. Retrieves results and runs same benchmarks locally
4. Generates comparison report
5. Cleans up temporary files

## Requirements

- SSH access to remote host
- rsync available on both local and remote
- Rust toolchain on remote host
- Remote host should have similar CPU architecture for meaningful comparison

## Output

Creates a timestamped results file (e.g., `benchmark_results_20250106_143022.txt`) containing:
- Remote benchmark results
- Local benchmark results
- Comparison summary

## Example Output File

```
=== REMOTE BENCHMARK RESULTS ===
Date: Mon Jan 6 14:30:22 UTC 2025
Host: root@10.10.10.2
Rust version: cargo 1.75.0

[benchmark results...]

=== LOCAL BENCHMARK RESULTS ===
Date: Mon Jan 6 14:35:15 UTC 2025
Host: local-machine
Rust version: cargo 1.75.0

[benchmark results...]

=== COMPARISON SUMMARY ===
[this file contains benchmark results...]
```

