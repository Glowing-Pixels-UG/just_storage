# Operations Manual

Day-to-day operations guide for JustStorage.

## Table of Contents

- [Configuration](#configuration)
- [Monitoring](#monitoring)
- [Maintenance](#maintenance)
- [Backup and Recovery](#backup-and-recovery)
- [Troubleshooting](#troubleshooting)
- [Security](#security)

---

## Configuration

### Environment Variables

#### Required

| Variable | Description | Example |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgresql://user:pass@localhost/just_storage` |
| `HOT_STORAGE_ROOT` | Path to hot storage directory | `/data/hot` |
| `COLD_STORAGE_ROOT` | Path to cold storage directory | `/data/cold` |

#### Optional

| Variable | Description | Default |
|----------|-------------|---------|
| `SERVER_PORT` | HTTP server port | `8080` |
| `SERVER_HOST` | Bind address | `0.0.0.0` |
| `GC_INTERVAL_SECS` | Garbage collection interval | `60` |
| `GC_BATCH_SIZE` | Max blobs to delete per GC run | `100` |
| `LOG_LEVEL` | Log level | `info` |
| `LOG_FORMAT` | Log format (`json` or `pretty`) | `json` |

#### Authentication

| Variable | Description | Example |
|----------|-------------|---------|
| `JWT_SECRET` | Secret for JWT validation | `your-secret-key-here` |
| `API_KEYS` | Comma-separated API keys | `key1,key2,key3` |
| `DISABLE_AUTH` | Disable auth (dev only!) | `false` |

### Configuration File

Create `.env` file in project root:

```bash
# Database
DATABASE_URL=postgresql://just_storage:password@postgres:5432/just_storage

# Storage
HOT_STORAGE_ROOT=/data/hot
COLD_STORAGE_ROOT=/data/cold

# Server
SERVER_PORT=8080
SERVER_HOST=0.0.0.0

# Garbage Collection
GC_INTERVAL_SECS=60
GC_BATCH_SIZE=100

# Logging
LOG_LEVEL=info
LOG_FORMAT=json

# Authentication
JWT_SECRET=change-this-in-production
API_KEYS=dev-key-1,dev-key-2
DISABLE_AUTH=false
```

---

## Monitoring

### Health Checks

#### Liveness Probe

```bash
curl http://localhost:8080/health
```

Returns 200 if service is running.

#### Readiness Probe

Check database and storage:

```bash
# Database check
psql $DATABASE_URL -c "SELECT 1"

# Storage check
ls $HOT_STORAGE_ROOT && ls $COLD_STORAGE_ROOT
```

### Metrics

JustStorage uses structured logging. Key metrics to monitor:

#### Request Metrics

```json
{
  "timestamp": "2025-12-11T10:30:00Z",
  "level": "INFO",
  "target": "just_storage::api::middleware::metrics",
  "fields": {
    "message": "Request completed",
    "method": "POST",
    "uri": "/v1/objects",
    "status": 201,
    "duration_ms": 150
  }
}
```

#### GC Metrics

```json
{
  "timestamp": "2025-12-11T10:31:00Z",
  "level": "INFO",
  "target": "just_storage::application::gc::worker",
  "fields": {
    "message": "Garbage collection completed",
    "blobs_deleted": 42,
    "duration_ms": 1250
  }
}
```

### Log Aggregation

#### Export to File

```bash
# Redirect logs
./just_storage 2>&1 | tee -a /var/log/just_storage.log
```

#### Docker Logs

```bash
# Follow logs
docker logs -f just_storage

# Export to file
docker logs just_storage > logs.txt
```

#### Kubernetes

```bash
# View logs
kubectl logs -f deployment/just_storage

# Export logs
kubectl logs deployment/just_storage --since=24h > logs.txt
```

### Recommended Alerts

| Alert | Condition | Severity |
|-------|-----------|----------|
| High Error Rate | >1% 5xx responses | Critical |
| Upload Failures | Upload errors >5/min | Warning |
| Storage Full | Disk usage >90% | Critical |
| Database Unreachable | Connection failures | Critical |
| GC Backlog | Orphaned blobs >1000 | Warning |
| Slow Requests | p99 latency >5s | Warning |

---

## Maintenance

### Database Operations

#### Vacuum

```sql
-- Regular vacuum (run weekly)
VACUUM ANALYZE objects;
VACUUM ANALYZE blobs;

-- Full vacuum (run monthly during maintenance window)
VACUUM FULL ANALYZE objects;
VACUUM FULL ANALYZE blobs;
```

#### Reindex

```sql
-- Rebuild indexes if fragmented
REINDEX TABLE objects;
REINDEX TABLE blobs;
```

#### Statistics

```sql
-- Check table sizes
SELECT
  schemaname,
  tablename,
  pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- Check object counts by state
SELECT status, COUNT(*) FROM objects GROUP BY status;

-- Check storage usage
SELECT
  storage_class,
  COUNT(*) as blob_count,
  pg_size_pretty(SUM(size_bytes)) as total_size
FROM objects
WHERE status = 'COMMITTED'
GROUP BY storage_class;
```

### Storage Cleanup

#### Validate Database

Use the included CLI tool:

```bash
cd rust
cargo build --bin validate_db

./target/debug/validate_db \
  --database-url "$DATABASE_URL" \
  --fix  # Add --fix to automatically fix issues
```

Checks for:

- Invalid status values
- Invalid storage_class values
- Invalid content_hash formats

#### Manual Cleanup

Find orphaned files (files without DB entries):

```bash
# List all files
find $HOT_STORAGE_ROOT -type f > /tmp/physical_files.txt

# Compare with DB
psql $DATABASE_URL <<EOF
COPY (
  SELECT DISTINCT content_hash
  FROM objects
  WHERE content_hash IS NOT NULL AND status != 'DELETED'
) TO '/tmp/db_hashes.txt';
EOF

# Find orphans (files not in DB)
comm -23 <(sort /tmp/physical_files.txt) <(sort /tmp/db_hashes.txt)
```

### Garbage Collection

#### Monitor GC

```bash
# Check GC logs
docker logs just_storage 2>&1 | grep "Garbage collection"

# Query orphaned blobs
psql $DATABASE_URL <<EOF
SELECT COUNT(*)
FROM blobs
WHERE ref_count = 0;
EOF
```

#### Force GC Run

GC runs automatically every 60 seconds. To force immediate run:

```bash
# Restart service (GC runs on startup)
docker restart just_storage
```

#### Tune GC

Adjust via environment variables:

```bash
# Increase batch size for faster cleanup
GC_BATCH_SIZE=500

# Decrease interval for more frequent runs
GC_INTERVAL_SECS=30
```

---

## Backup and Recovery

### Database Backup

#### Daily Backup

```bash
#!/bin/bash
# backup-db.sh

DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR=/backups/postgres
BACKUP_FILE=$BACKUP_DIR/just_storage_$DATE.sql.gz

mkdir -p $BACKUP_DIR

pg_dump $DATABASE_URL | gzip > $BACKUP_FILE

# Keep 7 days of backups
find $BACKUP_DIR -name "*.sql.gz" -mtime +7 -delete

echo "Backup completed: $BACKUP_FILE"
```

#### Point-in-Time Recovery

Enable WAL archiving in PostgreSQL:

```bash
# postgresql.conf
wal_level = replica
archive_mode = on
archive_command = 'cp %p /var/lib/postgresql/wal_archive/%f'
```

### Storage Backup

#### Incremental Backup

```bash
#!/bin/bash
# backup-storage.sh

STORAGE_DIRS="/data/hot /data/cold"
BACKUP_DEST=/backups/storage

for dir in $STORAGE_DIRS; do
  rsync -av --delete $dir $BACKUP_DEST/
done
```

#### Snapshot-Based Backup

If using Longhorn/ZFS:

```bash
# Create snapshot
zfs snapshot tank/just_storage@$(date +%Y%m%d)

# Send to backup server
zfs send tank/just_storage@latest | \
  ssh backup-server zfs receive backup/just_storage
```

### Recovery Procedures

#### Full Recovery

```bash
# 1. Stop service
docker stop just_storage

# 2. Restore database
gunzip < backup.sql.gz | psql $DATABASE_URL

# 3. Restore storage
rsync -av /backups/storage/hot/ /data/hot/
rsync -av /backups/storage/cold/ /data/cold/

# 4. Start service
docker start just_storage

# 5. Verify
curl http://localhost:8080/health
```

#### Partial Recovery

To recover specific objects:

```sql
-- Find object in backup
SELECT * FROM objects WHERE key = 'important-model';

-- Get content_hash
-- Copy file from backup to storage
-- Restore DB row
INSERT INTO objects (...) VALUES (...);
```

---

## Security

### Authentication Setup

#### Generate JWT Secret

```bash
# Generate strong random secret
openssl rand -base64 32
```

Set in environment:

```bash
JWT_SECRET=your-generated-secret-here
```

#### Generate API Keys

```bash
# Generate API keys
openssl rand -hex 32  # Generate key1
openssl rand -hex 32  # Generate key2

# Set in environment
API_KEYS=key1-here,key2-here
```

### Access Control

#### Tenant Isolation

All operations require `tenant_id`. Objects are isolated by:

- Namespace
- Tenant ID
- Optional key

Ensure applications:

- Always set correct tenant_id
- Never access other tenants' data
- Validate tenant_id matches authenticated user

### Network Security

#### Production Setup

```yaml
# Use TLS termination at load balancer
# Restrict direct access to service
apiVersion: v1
kind: Service
metadata:
  name: just-storage
spec:
  type: ClusterIP  # Internal only
  ports:
  - port: 8080
```

#### Database Security

```bash
# Use SSL for database connections
DATABASE_URL=postgresql://user:pass@host/db?sslmode=require

# Restrict database access
# In pg_hba.conf:
# host just_storage just_storage 10.0.0.0/8 scram-sha-256
```

### Audit Logging

Enable audit logging for sensitive operations:

```bash
LOG_LEVEL=debug  # Logs all operations with details
```

Review logs regularly:

```bash
# Find all delete operations
grep '"message":"Object deleted"' logs.json

# Find failed auth attempts
grep '"status":401' logs.json
```

---

## Daily Operations

### Morning Checklist

```bash
# 1. Check service health
curl http://localhost:8080/health

# 2. Review overnight logs
docker logs just_storage --since=24h | grep ERROR

# 3. Check disk usage
df -h /data/hot /data/cold

# 4. Verify GC is running
docker logs just_storage 2>&1 | tail -100 | grep "Garbage collection"

# 5. Check database size
psql $DATABASE_URL -c "SELECT pg_size_pretty(pg_database_size('just_storage'));"
```

### Weekly Tasks

- Review error logs
- Check backup success
- Vacuum database
- Review storage growth trends
- Update documentation if needed

### Monthly Tasks

- Full database vacuum
- Review and archive old backups
- Security audit (check logs for suspicious activity)
- Capacity planning review
- Update dependencies

---

## Common Tasks

### Add New API Key

```bash
# Generate new key
NEW_KEY=$(openssl rand -hex 32)

# Add to existing keys
export API_KEYS="$API_KEYS,$NEW_KEY"

# Restart service
docker restart just_storage

echo "New API key: $NEW_KEY"
```

### Rotate JWT Secret

```bash
# Generate new secret
NEW_SECRET=$(openssl rand -base64 32)

# Update environment
export JWT_SECRET=$NEW_SECRET

# Restart service
docker restart just_storage

# Note: All existing JWT tokens will be invalidated
```

### Migrate Storage Location

```bash
# 1. Stop service
docker stop just_storage

# 2. Copy data
rsync -av /old/hot/ /new/hot/
rsync -av /old/cold/ /new/cold/

# 3. Update configuration
# Edit .env or docker-compose.yml

# 4. Start service
docker start just_storage

# 5. Verify
curl http://localhost:8080/health
```

### Scale Storage

```bash
# If using Longhorn/ZFS:
# 1. Expand volume
# 2. Resize filesystem
# Service continues running, no downtime needed
```

---

## Performance Tuning

### Database

```sql
-- Adjust PostgreSQL settings for your workload
ALTER SYSTEM SET shared_buffers = '4GB';
ALTER SYSTEM SET effective_cache_size = '12GB';
ALTER SYSTEM SET maintenance_work_mem = '1GB';
ALTER SYSTEM SET checkpoint_completion_target = 0.9;
ALTER SYSTEM SET wal_buffers = '16MB';
```

### Application

```bash
# Tune GC for high-churn environments
GC_BATCH_SIZE=1000
GC_INTERVAL_SECS=30

# Increase worker threads for high load
TOKIO_WORKER_THREADS=8
```

---

## See Also

- [DEPLOYMENT.md](DEPLOYMENT.md) - Production deployment
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues
- [MONITORING.md](MONITORING.md) - Detailed monitoring setup
- [API.md](API.md) - API reference
