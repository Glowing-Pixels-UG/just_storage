# Quick Start Guide

Get JustStorage running in 5 minutes.

## Prerequisites

- Docker and Docker Compose
- (Optional) curl for testing

## Option 1: Docker Compose (Recommended)

### 1. Clone and Start

```bash
git clone <repository-url>
cd just_storage
docker-compose up -d
```

This starts:

- PostgreSQL database
- JustStorage service on port 8080

### 2. Verify Health

```bash
curl http://localhost:8080/health
```

Expected response:

```json
{"status":"healthy"}
```

### 3. Set Authentication (Development)

For testing, disable auth:

```bash
export DISABLE_AUTH=true
docker-compose restart just_storage
```

### 4. Upload Your First Object

```bash
echo "Hello, JustStorage!" > test.txt

curl -X POST \
  'http://localhost:8080/v1/objects?namespace=demo&tenant_id=test&key=first-object' \
  -H "Content-Type: application/octet-stream" \
  --data-binary @test.txt
```

Response:

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "namespace": "demo",
  "tenant_id": "test",
  "key": "first-object",
  "content_hash": "sha256:...",
  "size_bytes": 21,
  "storage_class": "hot",
  "status": "COMMITTED",
  "created_at": "2025-12-11T10:30:00Z"
}
```

### 5. Download It Back

```bash
# By key
curl 'http://localhost:8080/v1/objects/by-key/demo/test/first-object' \
  -o downloaded.txt

cat downloaded.txt
# Output: Hello, JustStorage!
```

### 6. List Objects

```bash
curl 'http://localhost:8080/v1/objects?namespace=demo&tenant_id=test'
```

### 7. Delete Object

```bash
curl -X DELETE \
  'http://localhost:8080/v1/objects/550e8400-e29b-41d4-a716-446655440000?tenant_id=test'
```

## Option 2: Local Development

### 1. Install Dependencies

```bash
# Rust 1.75+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# PostgreSQL 14+
brew install postgresql@14  # macOS
# OR
apt install postgresql-14   # Linux
```

### 2. Setup Database

```bash
# Start PostgreSQL
brew services start postgresql@14  # macOS
# OR
sudo systemctl start postgresql    # Linux

# Create database
createdb just_storage

# Apply schema
psql just_storage < schema.sql
```

### 3. Configure Environment

```bash
cp env.template .env

# Edit .env with your settings:
# DATABASE_URL=postgresql://localhost/just_storage
# HOT_STORAGE_ROOT=/tmp/just_storage/hot
# COLD_STORAGE_ROOT=/tmp/just_storage/cold
# DISABLE_AUTH=true
```

### 4. Build and Run

```bash
cd rust
cargo build --release
cargo run --release
```

Service starts on <http://localhost:8080>

### 5. Test It

Follow steps 4-7 from Option 1 above.

## Next Steps

- **API Reference**: See [API.md](API.md) for complete API documentation
- **Authentication**: See [OPERATIONS.md](OPERATIONS.md) for JWT/API key setup
- **Production Deploy**: See [DEPLOYMENT.md](DEPLOYMENT.md)
- **Development**: See [DEVELOPMENT.md](../DEVELOPMENT.md)

## Common Issues

### Port 8080 Already in Use

```bash
# Change port in docker-compose.yml or .env
SERVER_PORT=8081
```

### Database Connection Failed

```bash
# Check PostgreSQL is running
docker-compose logs postgres

# Verify connection string in .env
DATABASE_URL=postgresql://user:pass@localhost/just_storage
```

### Permission Denied on Storage Directories

```bash
# Create and set permissions
mkdir -p /tmp/just_storage/{hot,cold}
chmod 777 /tmp/just_storage/{hot,cold}
```

## Useful Commands

```bash
# View logs
docker-compose logs -f just_storage

# Stop services
docker-compose down

# Reset everything
docker-compose down -v
rm -rf data/

# Run tests
cd rust && cargo test

# Check code quality
cd rust && cargo clippy
```

## What's Next?

Try these examples:

### Upload a Large File

```bash
# Generate 100MB test file
dd if=/dev/urandom of=large.bin bs=1M count=100

# Upload with cold storage (for archives)
curl -X POST \
  'http://localhost:8080/v1/objects?namespace=files&tenant_id=test&storage_class=cold' \
  -H "Content-Type: application/octet-stream" \
  --data-binary @large.bin
```

### Test Deduplication

```bash
# Upload same file twice with different keys
curl -X POST \
  'http://localhost:8080/v1/objects?namespace=demo&tenant_id=test&key=copy1' \
  --data-binary @test.txt

curl -X POST \
  'http://localhost:8080/v1/objects?namespace=demo&tenant_id=test&key=copy2' \
  --data-binary @test.txt

# Both objects share the same content_hash and physical file!
```

### Pagination

```bash
# Upload multiple objects
for i in {1..100}; do
  echo "Object $i" | curl -X POST \
    "http://localhost:8080/v1/objects?namespace=demo&tenant_id=test&key=obj-$i" \
    --data-binary @-
done

# List with pagination
curl 'http://localhost:8080/v1/objects?namespace=demo&tenant_id=test&limit=10&offset=0'
curl 'http://localhost:8080/v1/objects?namespace=demo&tenant_id=test&limit=10&offset=10'
```

## Production Checklist

Before deploying to production:

- [ ] Enable authentication (remove `DISABLE_AUTH=true`)
- [ ] Configure JWT_SECRET with strong random value
- [ ] Set up proper storage volumes (not /tmp)
- [ ] Configure backup for PostgreSQL
- [ ] Set up monitoring and alerts
- [ ] Review [DEPLOYMENT.md](DEPLOYMENT.md) for production setup
- [ ] Load test your expected workload

## Getting Help

- **Documentation**: See [INDEX.md](INDEX.md) for all docs
- **Issues**: Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
- **API Questions**: See [API.md](API.md)
