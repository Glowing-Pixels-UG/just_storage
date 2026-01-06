# JustStorage - Content-Addressable Object Storage

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](#testing)
[![License](https://img.shields.io/badge/license-MIT-blue)](#license)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](#prerequisites)

[![Deploy to Heroku](https://www.herokucdn.com/deploy/button.svg)](https://heroku.com/deploy?template=https://github.com/yourusername/just_storage)
[![Deploy to DigitalOcean](https://www.deploytodo.com/do-btn-blue.svg)](https://cloud.digitalocean.com/apps/new?repo=https://github.com/yourusername/just_storage/tree/main)

A content-addressable object storage service with strong consistency guarantees, automatic deduplication, and crash-safe operations.

---

## 📋 Table of Contents

- [Overview](#overview)
- [Key Features](#key-features)
- [Quick Start](#quick-start)
- [API Reference](#api-reference)
- [Architecture](#architecture)
- [Deployment](#deployment)
- [Operations](#operations)
- [Development](#development)
- [Documentation Index](#documentation-index)
- [Status & Roadmap](#status--roadmap)

---

## Overview

**JustStorage** is an internal object storage service designed to replace generic S3/GCS usage with a domain-specific, streamlined interface:

### Design Approach

Replace generic S3/GCS usage with a domain-specific, streamlined interface designed for internal services:

- Simple API with 10-15 well-defined operations
- Strong consistency with read-after-write guarantees
- Clear cost model without complex pricing tiers
- Optimized for Model Hub, knowledge base, and file storage workloads

## Key Features

### Architecture

- **Longhorn + ZFS foundation**: Node-level HA, disk redundancy, volume snapshots
- **Content-addressable storage**: Automatic deduplication, integrity verification
- **Two-phase writes**: Crash-safe, atomic operations
- **Background GC**: Deferred cleanup, no read/delete races

### Storage Classes

- **Hot**: NVMe-backed for models, active data
- **Cold**: HDD-backed for archives, bulk storage

### API

- `POST /v1/objects` - Upload
- `GET /v1/objects/{id}` - Download by ID
- `GET /v1/objects/by-key/{namespace}/{tenant}/{key}` - Download by key
- `DELETE /v1/objects/{id}` - Delete (async GC)
- `GET /v1/objects` - List with pagination

## Architecture

This project follows **Clean Architecture** with clear separation of concerns:

```
Domain Layer (Core Business Logic)
    ↑
Application Layer (Use Cases)
    ↑
Infrastructure Layer (DB, Storage)
    ↑
API Layer (HTTP Handlers)
```

**Key Principles:**

- **SRP**: Each module has one responsibility
- **Dependency Inversion**: Domain depends on nothing, infra depends on domain
- **Testability**: Easy to mock ports for unit testing
- **Extensibility**: Add new storage backends without touching business logic

See [CLEAN_ARCHITECTURE.md](docs/CLEAN_ARCHITECTURE.md) for detailed architecture documentation.

## Quick Start

### Using Docker Compose (Easiest)

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f just_storage

# Test API
curl http://localhost:8080/health
```

### Local Development

#### Prerequisites

- **Postgres 14+** with uuid extension
- **Rust 1.75+** with tokio runtime

#### Setup

```bash
# 1. Copy environment file
cp .env.example .env

# 2. Start PostgreSQL
docker-compose up -d postgres
# OR use local PostgreSQL

# 3. Create database and run migrations
make db-setup
# OR manually:
# createdb just_storage
# psql just_storage < schema.sql

# 4. Build and run
cd rust
cargo run --release
```

See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed development guide.

## Example Usage

### Upload a model

```bash
curl -X POST http://localhost:8080/v1/objects \
  -H "X-Namespace: models" \
  -H "X-Tenant: acme" \
  -H "X-Key: gpt-4-turbo" \
  -H "X-Storage-Class: hot" \
  -H "Content-Type: application/octet-stream" \
  --data-binary @model.bin
```

Response:

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "namespace": "models",
  "tenant_id": "acme",
  "key": "gpt-4-turbo",
  "content_hash": "sha256:abcdef123...",
  "size_bytes": 1073741824,
  "storage_class": "hot",
  "created_at": "2025-12-11T10:30:00Z"
}
```

### Download

```bash
# By ID
curl http://localhost:8080/v1/objects/550e8400-e29b-41d4-a716-446655440000 \
  -H "X-Tenant: acme" -o model.bin

# By key
curl http://localhost:8080/v1/objects/by-key/models/acme/gpt-4-turbo -o model.bin
```

### List objects

```bash
curl "http://localhost:8080/v1/objects?namespace=models&tenant=acme&limit=50"
```

### Delete

```bash
curl -X DELETE http://localhost:8080/v1/objects/550e8400-e29b-41d4-a716-446655440000 \
  -H "X-Tenant: acme"
```

---

## Documentation Index

Comprehensive documentation is available in the `/docs` directory:

### 📖 Getting Started

- **[Quick Start Guide](docs/QUICKSTART.md)** - Get running in 5 minutes
- **[API Reference](docs/API.md)** - Complete API documentation with examples
- **[Architecture Overview](docs/ARCHITECTURE.md)** - System design and components

### 🏗️ Architecture & Design

- **[Clean Architecture](docs/CLEAN_ARCHITECTURE.md)** - Layer separation and patterns
- **[Design Decisions](DESIGN.md)** - State machine and consistency model
- **[Database Schema](docs/DATABASE.md)** - Schema, indexes, and migrations
- **[Responsibility Boundaries](docs/LONGHORN_VS_SERVICE.md)** - What we handle vs infrastructure

### 💻 Development

- **[Development Guide](DEVELOPMENT.md)** - Setup and workflow
- **[Linting & Static Analysis](LINTING_SETUP.md)** - Code quality tools and setup
- **[Rust Best Practices](docs/RUST_BEST_PRACTICES.md)** - Coding standards
- **[Testing Guide](docs/TESTING.md)** - Testing strategy and examples
- **[Contributing](docs/CONTRIBUTING.md)** - How to contribute

### 🚀 Operations

- **[Deployment Guide](docs/DEPLOYMENT.md)** - Production deployment
- **[Operations Manual](docs/OPERATIONS.md)** - Day-to-day operations
- **[Monitoring Setup](docs/MONITORING.md)** - Metrics and alerting
- **[Troubleshooting](docs/TROUBLESHOOTING.md)** - Common issues and solutions

### 📚 Reference

- **[Implementation Details](IMPLEMENTATION.md)** - Code examples and patterns
- **[Completion Summary](COMPLETION_SUMMARY.md)** - Implementation checklist
- **[Documentation Index](docs/INDEX.md)** - Complete documentation map

---

## Key Design Decisions

### What Longhorn + ZFS Solve (We Don't Re-Implement)

- Node failures & replica management
- Disk failures & checksums
- Volume snapshots & backups to remote storage
- Block-level integrity

### What JustStorage Handles

- Object semantics (tenants, namespaces, keys)
- Concurrency control (writes, deletes, GC)
- Content-addressable storage with deduplication
- Metadata indexing & fast listing
- Object-level integrity (SHA-256 hashing)
- API, auth, metrics

### S3/GCS Mistakes We Avoid

| S3/GCS Problem | Our Solution |
|----------------|--------------|
| Fake filesystem semantics | No directories, opaque keys |
| Slow listing (millions of objects) | DB-backed indexes |
| Eventual consistency | Strong read-after-write |
| Mysterious throttling | Explicit 429 + metrics |
| Versioning tombstone hell | Explicit versions in DB |
| Complex pricing | Simple $/GB model |

## State Machine

Every object transitions through explicit states:

```
    (none)
       ↓ POST /objects
   WRITING (temp file, DB txn #1)
       ↓ file written, fsync, rename, DB txn #2
  COMMITTED (visible to reads)
       ↓ DELETE /objects
  DELETING (metadata marked, ref_count--)
       ↓ background GC
   DELETED (file removed)
```

**Crash safety**: If crash occurs before `COMMITTED`, object is not visible and will be GC'd as orphan.

## Deployment

### One-Click Deploy

Deploy JustStorage instantly to popular platforms with a single click:

[![Deploy to Heroku](https://www.herokucdn.com/deploy/button.svg)](https://heroku.com/deploy?template=https://github.com/yourusername/just_storage)
[![Deploy to DigitalOcean](https://www.deploytodo.com/do-btn-blue.svg)](https://cloud.digitalocean.com/apps/new?repo=https://github.com/yourusername/just_storage/tree/main)

**Note:** 
- Replace `yourusername` in the URLs above with your actual GitHub username/organization
- For Heroku: The button will prompt you to set `JWT_SECRET` and `API_KEYS` during deployment
- For DigitalOcean: Set `JWT_SECRET` and `API_KEYS` as secrets in the dashboard after deployment

### Quick Deployment Setup

JustStorage includes a CLI tool for generating deployment configurations:

```bash
# Build the deployment CLI
cd rust && cargo build --release --bin just-storage-deploy

# Generate configuration for your platform
cargo run --release --bin just-storage-deploy -- generate <platform>

# Supported platforms: caprover, heroku, flyio, railway, render, digitalocean
```

**Example:**
```bash
# Generate Heroku configuration
just-storage-deploy generate heroku

# Generate Fly.io config with custom settings
just-storage-deploy generate flyio --app-name my-app --region ord
```

See [Deployment Guide](docs/DEPLOYMENT.md) for detailed platform-specific instructions.

### Kubernetes (StatefulSet)

```yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: just-storage
spec:
  serviceName: just-storage
  replicas: 1
  selector:
    matchLabels:
      app: just-storage
  template:
    metadata:
      labels:
        app: just-storage
    spec:
      containers:
      - name: just-storage
        image: just-storage:latest
        ports:
        - containerPort: 8080
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: just-storage-secrets
              key: database-url
        volumeMounts:
        - name: hot-storage
          mountPath: /data/hot
        - name: cold-storage
          mountPath: /data/cold
  volumeClaimTemplates:
  - metadata:
      name: hot-storage
    spec:
      storageClassName: longhorn-nvme
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 1Ti
  - metadata:
      name: cold-storage
    spec:
      storageClassName: longhorn-standard
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 10Ti
```

## Observability

### Health Checks

- `GET /health` - Liveness
- `GET /ready` - Readiness (DB + filesystem checks)

### Metrics (Prometheus)

```
just_storage_requests_total{method, status, namespace}
just_storage_request_duration_seconds{method, namespace}
just_storage_objects_total{namespace, tenant, status}
just_storage_storage_bytes{storage_class, namespace}
just_storage_gc_runs_total
just_storage_gc_deleted_blobs_total
```

## Testing

```bash
# Unit tests
cargo test

# Integration tests (requires test DB)
cargo test --test integration

# With coverage
cargo tarpaulin --out Html
```

## Contributing

1. Read `DESIGN.md` for architecture context
2. Check `docs/LONGHORN_VS_SERVICE.md` for responsibility boundaries
3. Follow patterns in `IMPLEMENTATION.md`
4. Write tests for new features
5. Update metrics for new operations

## License

This project is licensed under the MIT License — see the [LICENSE](./LICENSE) file for details.

## Status & Roadmap

### Implementation Status (v0.1.0)

**Core Implementation Complete:**

- Clean Architecture with domain/application/infrastructure/api layers
- Database schema with state machine and migrations
- All CRUD operations (upload, download, delete, list)
- Content-addressable storage with automatic deduplication
- Two-phase commit for crash safety
- Background garbage collection with tested worker
- JWT and API key authentication
- Health check endpoints
- Error handling without unsafe unwrap/expect
- Unit test coverage with in-memory mocks
- Database validation CLI tool
- Clippy-clean, formatted code
- API documentation and examples

**Deployment Ready:**

- Docker and docker-compose configurations
- Kubernetes StatefulSet manifests
- Environment variable configuration
- Migration scripts

### 🎯 Next Steps

- [ ] Integration tests with real database
- [ ] Prometheus metrics implementation
- [ ] Production deployment to dev cluster
- [ ] Performance benchmarks
- [ ] Monitoring dashboards
- [ ] Load testing

### Tech Stack

| Component | Technology |
|-----------|------------|
| **Language** | Rust 1.75+ (2021 edition) |
| **Runtime** | Tokio async |
| **HTTP Framework** | Axum 0.8 |
| **Database** | PostgreSQL 14+ with SQLx |
| **Storage** | Content-addressable filesystem |
| **Authentication** | JWT + API Keys |
| **Logging** | tracing + tracing-subscriber |
| **Testing** | cargo test, mockall |

**Next Milestone:** Deploy to production and migrate first service (Model Hub).
