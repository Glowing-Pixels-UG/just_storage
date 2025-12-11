# ActiveStorage Development Guide

## Quick Start

### 1. Using Docker Compose (Recommended)

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f activestorage

# Stop services
docker-compose down
```

The service will be available at `http://localhost:8080`

### 2. Local Development

#### Prerequisites

- Rust 1.75+
- PostgreSQL 14+
- Git

#### Setup

```bash
# Clone repository
git clone <repo-url>
cd just_storage

# Copy environment file
cp .env.example .env

# Start PostgreSQL (if not using Docker)
# e.g., using Homebrew on macOS:
brew services start postgresql@15

# Create database
createdb activestorage

# Run migrations
psql activestorage < schema.sql

# Create storage directories
mkdir -p /data/hot /data/cold
# Or use local directories:
mkdir -p ./data/hot ./data/cold
# And update .env:
# HOT_STORAGE_ROOT=./data/hot
# COLD_STORAGE_ROOT=./data/cold

# Build and run
cd rust
cargo build --release
cargo run --release
```

## API Examples

### Upload Object

```bash
curl -X POST "http://localhost:8080/v1/objects?namespace=models&tenant_id=$(uuidgen)&storage_class=hot" \
  --data-binary @model.bin
```

### Download Object

```bash
curl -X GET "http://localhost:8080/v1/objects/{object_id}" \
  -o downloaded.bin
```

### List Objects

```bash
curl "http://localhost:8080/v1/objects?namespace=models&tenant_id={tenant_id}&limit=10&offset=0"
```

### Delete Object

```bash
curl -X DELETE "http://localhost:8080/v1/objects/{object_id}"
```

### Health Check

```bash
curl http://localhost:8080/health
```

## Development Workflow

### Running Tests

```bash
cd rust
cargo test
```

### Code Formatting

```bash
cargo fmt
```

### Linting

```bash
cargo clippy
```

### Check Compilation

```bash
cargo check
```

## Architecture Overview

The project follows Clean Architecture:

```
Domain Layer (Business Logic)
    ↑
Application Layer (Use Cases)
    ↑
Infrastructure Layer (DB, Storage)
    ↑
API Layer (HTTP)
```

See [CLEAN_ARCHITECTURE.md](docs/CLEAN_ARCHITECTURE.md) for details.

## Configuration

All configuration is done via environment variables. See `.env.example` for available options.

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://postgres:password@localhost/activestorage` |
| `HOT_STORAGE_ROOT` | Path for hot storage | `/data/hot` |
| `COLD_STORAGE_ROOT` | Path for cold storage | `/data/cold` |
| `LISTEN_ADDR` | Server bind address | `0.0.0.0:8080` |
| `GC_INTERVAL_SECS` | GC run interval | `60` |
| `GC_BATCH_SIZE` | Blobs per GC cycle | `100` |
| `RUST_LOG` | Log level | `info` |

## Troubleshooting

### Database Connection Issues

```bash
# Check PostgreSQL is running
psql -U postgres -c "SELECT 1"

# Check database exists
psql -U postgres -l | grep activestorage
```

### Storage Permission Issues

```bash
# Ensure directories exist and are writable
ls -la /data/hot /data/cold

# Fix permissions
sudo chown -R $USER:$USER /data/hot /data/cold
```

### Port Already in Use

```bash
# Find process using port 8080
lsof -i :8080

# Change port in .env
LISTEN_ADDR=0.0.0.0:8081
```

## Production Deployment

See [DEPLOYMENT.md](docs/DEPLOYMENT.md) for production deployment guide including:

- Kubernetes manifests
- Longhorn configuration
- Monitoring setup
- Backup strategies
