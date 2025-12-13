# JustStorage Kubernetes Deployment

Kubernetes manifests for deploying JustStorage on the Laika cluster.

## Prerequisites

- kubectl configured with access to Laika cluster
- PostgreSQL database available (either as a separate deployment or external service)
- Longhorn storage class available (`longhorn-fast`)
- Traefik ingress controller configured

## Files

- `namespace.yaml` - Creates the `just-storage` namespace
- `pvc.yaml` - Persistent volume claims for hot (100Gi) and cold (500Gi) storage
- `secret-template.yaml` - Template for creating secrets (database URL, JWT, API keys)
- `deployment.yaml` - JustStorage application deployment
- `service.yaml` - ClusterIP service exposing port 8080
- `ingress.yaml` - Traefik ingress configuration

## Deployment Steps

### 1. Create Namespace

```bash
kubectl apply -f namespace.yaml
```

### 2. Create Database and User

Create a dedicated database and user for JustStorage:

```bash
# Install uv if not already installed
# macOS/Linux: curl -LsSf https://astral.sh/uv/install.sh | sh
# Or: brew install uv

# Install dependencies and run the database setup script
cd k8s
uv sync
uv run python create-database.py
```

**Alternative:** If you prefer using requirements.txt directly:

```bash
cd k8s
uv pip install -r requirements.txt
uv run python create-database.py
```

This script will:
- Create a `just_storage` user with a secure random password
- Create a `just_storage` database
- Apply the full database schema
- Display the connection string to use in secrets

**Note:** The script uses the existing `infra-postgres` cluster in the `data` namespace.

### 3. Create Secrets

Copy the secret template and update with the database connection string from step 2:

```bash
cp secret-template.yaml secret.yaml
# Edit secret.yaml with the connection string from create-database.py output
```

Then create the secret:

```bash
kubectl apply -f secret.yaml
```

**Important:** The `database-url` should be in the format:
- `postgresql://just_storage:PASSWORD@infra-postgres-rw.data:5432/just_storage`
- The password will be URL-encoded (the script shows the correct format)

### 4. Create Persistent Volume Claims

```bash
kubectl apply -f pvc.yaml
```

This creates:
- `just-storage-hot` - 100Gi for hot storage (NVMe-backed)
- `just-storage-cold` - 500Gi for cold storage (HDD-backed)

Both use `longhorn-fast` storage class. Adjust sizes in `pvc.yaml` if needed.

### 5. Update Deployment Image

Edit `deployment.yaml` and update the image reference:

```yaml
image: storage.bk.glpx.pro/just_storage:latest
```

Replace with your actual image registry and tag.

### 6. Deploy Application

```bash
kubectl apply -f deployment.yaml
```

### 7. Create Service

```bash
kubectl apply -f service.yaml
```

### 8. Configure Ingress

Update `ingress.yaml` with your desired hostname, then apply:

```bash
kubectl apply -f ingress.yaml
```

The default hostname is `storage.bk.glpx.pro`. Update if needed.

## Verify Deployment

```bash
# Check pods
kubectl get pods -n just-storage

# Check services
kubectl get svc -n just-storage

# Check ingress
kubectl get ingress -n just-storage

# View logs
kubectl logs -n just-storage -l app=just-storage -f

# Test health endpoint
kubectl port-forward -n just-storage svc/just-storage 8080:8080
curl http://localhost:8080/health
```

## Configuration

### Environment Variables

The deployment uses the following environment variables (configured in `deployment.yaml`):

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | From secret |
| `HOT_STORAGE_ROOT` | Hot storage path | `/data/hot` |
| `COLD_STORAGE_ROOT` | Cold storage path | `/data/cold` |
| `LISTEN_ADDR` | Server bind address | `0.0.0.0:8080` |
| `GC_INTERVAL_SECS` | Garbage collection interval | `60` |
| `GC_BATCH_SIZE` | Blobs per GC cycle | `100` |
| `RUST_LOG` | Log level | `info` |

### Database Connection Pool

Optional database pool settings (with defaults):
- `DB_MAX_CONNECTIONS`: 20
- `DB_MIN_CONNECTIONS`: 5
- `DB_ACQUIRE_TIMEOUT_SECS`: 30
- `DB_IDLE_TIMEOUT_SECS`: 600
- `DB_MAX_LIFETIME_SECS`: 1800

### Authentication

For production, configure authentication via secrets:
- `JWT_SECRET` - Secret for JWT validation
- `API_KEYS` - Comma-separated API keys
- `DISABLE_AUTH` - Set to `false` (or omit) for production

Uncomment the authentication section in `deployment.yaml` and add the secrets to `secret.yaml`.

## Resource Requirements

Default resource requests/limits:
- Requests: 512Mi memory, 500m CPU
- Limits: 2Gi memory, 2 CPU

Adjust in `deployment.yaml` based on your workload.

## Storage

- **Hot Storage**: 100Gi on `longhorn-fast` (NVMe-backed)
- **Cold Storage**: 500Gi on `longhorn-fast` (HDD-backed)

Both volumes use `ReadWriteOnce` access mode. Adjust sizes in `pvc.yaml` as needed.

## Troubleshooting

### Pod Not Starting

```bash
# Check pod status
kubectl describe pod -n just-storage -l app=just-storage

# Check logs
kubectl logs -n just-storage -l app=just-storage
```

### Database Connection Issues

Verify the database URL in the secret:
```bash
kubectl get secret -n just-storage just-storage-secrets -o jsonpath='{.data.database-url}' | base64 -d
```

### Storage Issues

Check PVC status:
```bash
kubectl get pvc -n just-storage
kubectl describe pvc -n just-storage just-storage-hot
```

### Health Check Failures

```bash
# Port forward and test manually
kubectl port-forward -n just-storage svc/just-storage 8080:8080
curl http://localhost:8080/health
```

## Scaling

To scale the deployment:

```bash
kubectl scale deployment just-storage -n just-storage --replicas=2
```

**Note:** Since storage volumes use `ReadWriteOnce`, only one pod can mount each volume. For horizontal scaling, consider:
- Using a shared storage solution (NFS, CephFS)
- Implementing a distributed storage layer
- Using a StatefulSet with multiple replicas and separate volumes

## Backup

Longhorn provides volume snapshots. Configure backup policies in Longhorn UI or via Longhorn API.

## Updates

To update the deployment:

```bash
# Update image tag in deployment.yaml, then:
kubectl apply -f deployment.yaml

# Or use kubectl set image:
kubectl set image deployment/just-storage -n just-storage just-storage=storage.bk.glpx.pro/just_storage:v1.1
```

## Cleanup

To remove all resources:

```bash
kubectl delete -f ingress.yaml
kubectl delete -f service.yaml
kubectl delete -f deployment.yaml
kubectl delete -f pvc.yaml
kubectl delete -f secret.yaml
kubectl delete -f namespace.yaml
```

**Warning:** Deleting PVCs will delete the data. Ensure you have backups before cleanup.

