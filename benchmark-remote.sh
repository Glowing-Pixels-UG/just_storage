#!/bin/bash
# Run benchmarks on remote host and compare with local results
# Uses the same build pattern as build-and-deploy.sh

set -e

# Usage function
usage() {
    echo "Usage: $0 [IMAGE_TAG] [TARGET_HOST] [RESULTS_FILE]"
    echo ""
    echo "Run JustStorage benchmarks on remote host and compare with local results."
    echo ""
    echo "Arguments:"
    echo "  IMAGE_TAG    Docker image tag (default: benchmark)"
    echo "  TARGET_HOST  SSH target host (default: root@10.10.10.2)"
    echo "  RESULTS_FILE Output file name (default: benchmark_results_TIMESTAMP.txt)"
    echo ""
    echo "Examples:"
    echo "  $0                           # Use defaults"
    echo "  $0 v1.0.0                    # Custom tag"
    echo "  $0 latest user@server.com    # Custom tag and host"
    echo "  $0 latest user@server.com results.txt  # All custom"
    echo ""
    exit 1
}

# Check for help flag
if [[ "$1" == "-h" || "$1" == "--help" ]]; then
    usage
fi

IMAGE_NAME="just-storage"
IMAGE_TAG="${1:-benchmark}"
TARGET_HOST="${2:-root@10.10.10.2}"
BENCHMARK_RESULTS_FILE="${3:-benchmark_results_$(date +%Y%m%d_%H%M%S).txt}"

echo "Running benchmarks on remote host ${TARGET_HOST}..."
echo "Results will be saved to: ${BENCHMARK_RESULTS_FILE}"
echo ""

# Copy source directly to host
echo "Step 1: Copying source code to host..."
cd "$(dirname "$0")"

# Clean any hidden files first
echo "Cleaning hidden files..."
find rust -name "._*" -delete 2>/dev/null || echo "No ._ files found"
find rust -name ".DS_Store" -delete 2>/dev/null || echo "No .DS_Store files found"

echo "Copying source files..."
rsync -avz --delete --exclude='target' --exclude='Cargo.lock' rust/ ${TARGET_HOST}:/tmp/rust/
if [ $? -ne 0 ]; then
    echo "ERROR: Failed to copy source to host!"
    exit 1
fi

echo "Copying benchmark configuration..."
rsync -avz rust/benches/ ${TARGET_HOST}:/tmp/rust/benches/ 2>/dev/null || echo "No benches directory found"

echo ""
echo "Step 2: Building and running benchmarks on remote host..."
ssh ${TARGET_HOST} "set -e && \
    cd /tmp && \
    echo 'Cleaning previous builds...' && \
    rm -rf just_storage && \
    cd rust && \
    echo 'Checking migrations directory...' && \
    ls -la migrations/ 2>/dev/null || echo 'No migrations directory' && \
    echo 'Cleaning hidden migration files...' && \
    if [ -d migrations ]; then \
        cd migrations && \
        rm -f ._*.sql .DS_Store ._* 2>/dev/null || true && \
        ls -la 2>/dev/null || true && \
        cd ..; \
    fi && \
    export PATH=\$PATH:/usr/local/cargo/bin && \
    echo 'Running benchmarks...' && \
    echo '=== REMOTE BENCHMARK RESULTS ===' > /tmp/${BENCHMARK_RESULTS_FILE} && \
    echo \"Date: \$(date)\" >> /tmp/${BENCHMARK_RESULTS_FILE} && \
    echo \"Host: ${TARGET_HOST}\" >> /tmp/${BENCHMARK_RESULTS_FILE} && \
    echo \"Rust version: \$(cargo --version)\" >> /tmp/${BENCHMARK_RESULTS_FILE} && \
    echo \"\" >> /tmp/${BENCHMARK_RESULTS_FILE} && \
    cargo bench --bench storage_bench >> /tmp/${BENCHMARK_RESULTS_FILE} 2>&1 && \
    echo \"\" >> /tmp/${BENCHMARK_RESULTS_FILE} && \
    cargo bench --bench hash_bench >> /tmp/${BENCHMARK_RESULTS_FILE} 2>&1 && \
    echo \"\" >> /tmp/${BENCHMARK_RESULTS_FILE} && \
    cargo bench --bench http_bench >> /tmp/${BENCHMARK_RESULTS_FILE} 2>&1 && \
    echo 'Benchmarks completed successfully'"

echo ""
echo "Step 3: Retrieving benchmark results..."
rsync -avz ${TARGET_HOST}:/tmp/${BENCHMARK_RESULTS_FILE} ./
if [ $? -ne 0 ]; then
    echo "ERROR: Failed to retrieve benchmark results!"
    exit 1
fi

echo ""
echo "Step 4: Comparing with local results..."

# Get local benchmark results
echo "Running local benchmarks for comparison..."
echo "" >> ${BENCHMARK_RESULTS_FILE}
echo "=== LOCAL BENCHMARK RESULTS ===" >> ${BENCHMARK_RESULTS_FILE}
echo "Date: $(date)" >> ${BENCHMARK_RESULTS_FILE}
echo "Host: $(hostname)" >> ${BENCHMARK_RESULTS_FILE}
echo "Rust version: $(cargo --version 2>/dev/null || echo 'cargo not found')" >> ${BENCHMARK_RESULTS_FILE}
echo "" >> ${BENCHMARK_RESULTS_FILE}

echo "Running local storage benchmarks..."
cd rust
cargo bench --bench storage_bench >> ../${BENCHMARK_RESULTS_FILE} 2>&1 || echo "Local storage bench failed" >> ../${BENCHMARK_RESULTS_FILE}
echo "" >> ../${BENCHMARK_RESULTS_FILE}

echo "Running local hash benchmarks..."
cargo bench --bench hash_bench >> ../${BENCHMARK_RESULTS_FILE} 2>&1 || echo "Local hash bench failed" >> ../${BENCHMARK_RESULTS_FILE}
echo "" >> ../${BENCHMARK_RESULTS_FILE}

echo "Running local HTTP benchmarks..."
cargo bench --bench http_bench >> ../${BENCHMARK_RESULTS_FILE} 2>&1 || echo "Local HTTP bench failed" >> ../${BENCHMARK_RESULTS_FILE}
cd ..

echo ""
echo "Step 5: Generating comparison summary..."
echo "" >> ${BENCHMARK_RESULTS_FILE}
echo "=== COMPARISON SUMMARY ===" >> ${BENCHMARK_RESULTS_FILE}
echo "This file contains benchmark results from both remote and local environments." >> ${BENCHMARK_RESULTS_FILE}
echo "Compare the throughput numbers to understand performance differences." >> ${BENCHMARK_RESULTS_FILE}
echo "" >> ${BENCHMARK_RESULTS_FILE}
echo "Key metrics to compare:" >> ${BENCHMARK_RESULTS_FILE}
echo "- Storage operations: write/read throughput" >> ${BENCHMARK_RESULTS_FILE}
echo "- Hash computation: SHA-256 performance" >> ${BENCHMARK_RESULTS_FILE}
echo "- HTTP handlers: API performance" >> ${BENCHMARK_RESULTS_FILE}
echo "" >> ${BENCHMARK_RESULTS_FILE}

echo ""
echo "Step 6: Cleaning up temporary files on remote host..."
ssh ${TARGET_HOST} "rm -rf /tmp/rust /tmp/${BENCHMARK_RESULTS_FILE}"

echo ""
echo "✅ Benchmarking complete!"
echo ""
echo "Results saved to: ${BENCHMARK_RESULTS_FILE}"
echo ""
echo "Summary of key findings:"
echo "- Remote host: ${TARGET_HOST}"
echo "- Local host: $(hostname)"
echo "- Check ${BENCHMARK_RESULTS_FILE} for detailed throughput comparisons"
echo ""
echo "To view results:"
echo "  cat ${BENCHMARK_RESULTS_FILE} | less"
echo ""
echo "To compare specific benchmarks:"
echo "  grep -A 20 'storage_operations/write' ${BENCHMARK_RESULTS_FILE}"
echo "  grep -A 20 'hash_computation' ${BENCHMARK_RESULTS_FILE}"