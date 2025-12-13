#!/bin/bash
# Build Rust binary on remote host and deploy to Kubernetes
# Builds directly on the target host to avoid cross-compilation issues

set -e

IMAGE_NAME="just-storage"
IMAGE_TAG="${1:-latest}"
TARGET_HOST="${2:-root@10.10.10.2}"

echo "Building JustStorage on remote host ${TARGET_HOST}..."
echo ""

# Copy source directly to host
echo "Step 1: Copying source code to host..."
cd "$(dirname "$0")/.."

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

echo "Copying Dockerfile..."
rsync -avz k8s/Dockerfile.runtime ${TARGET_HOST}:/tmp/Dockerfile.runtime
if [ $? -ne 0 ]; then
    echo "ERROR: Failed to copy Dockerfile to host!"
    exit 1
fi

echo ""
echo "Step 2: Building Rust binary on remote host..."
ssh ${TARGET_HOST} "set -e && \
    cd /tmp && \
    echo 'Cleaning previous builds and target...' && \
    rm -rf just_storage && \
    cd rust && \
    cargo clean && \
    echo 'Checking migrations directory...' && \
    ls -la migrations/ && \
    echo 'Cleaning hidden migration files...' && \
    cd migrations && \
    rm -f ._*.sql .DS_Store ._* && \
    ls -la && \
    cd .. && \
    export PATH=\$PATH:/usr/local/cargo/bin && \
    echo 'Building with cargo...' && \
    cargo build --release && \
    echo 'Build completed successfully'"

echo ""
echo "Step 3: Binary built successfully!"
echo "Location on remote: /tmp/rust/target/release/just_storage"
echo ""

# Build container image on host
echo "Step 4: Building container image on host..."
ssh ${TARGET_HOST} "cd /tmp && \
    cp rust/target/release/just_storage . && \
    podman build -t storage.bk.glpx.pro/${IMAGE_NAME}:${IMAGE_TAG} -f Dockerfile.runtime . && \
    podman save storage.bk.glpx.pro/${IMAGE_NAME}:${IMAGE_TAG} | \
    ctr -n k8s.io images import --base-name storage.bk.glpx.pro/${IMAGE_NAME}:${IMAGE_TAG} -"

echo ""
echo "Step 5: Verifying image..."
ssh ${TARGET_HOST} "ctr -n k8s.io images ls | grep storage.bk.glpx.pro"

echo ""
echo "Step 6: Updating deployment with new image..."
kubectl set image deployment/just-storage -n just-storage just-storage=storage.bk.glpx.pro/${IMAGE_NAME}:${IMAGE_TAG}

echo ""
echo "Step 7: Restarting pods with new image..."
kubectl delete pod -n just-storage -l app=just-storage

echo ""
echo "Step 8: Waiting for pods to be ready..."
kubectl wait --for=condition=ready pod -l app=just-storage -n just-storage --timeout=300s

echo ""
echo "Step 9: Cleaning up temporary files..."
ssh ${TARGET_HOST} "rm -rf /tmp/rust /tmp/just_storage /tmp/Dockerfile.runtime"

echo ""
echo "✅ Build and deployment complete!"
echo ""
echo "Image: storage.bk.glpx.pro/${IMAGE_NAME}:${IMAGE_TAG}"
echo "Pods restarted and ready!"
echo ""
echo "New search endpoints available:"
echo "  POST /v1/objects/search        - Advanced search with filters"
echo "  POST /v1/objects/search/text   - Full-text search"
echo "  GET  /v1/objects/by-key/...    - Download by human-readable key"