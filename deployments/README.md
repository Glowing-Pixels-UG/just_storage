# JustStorage Deployment Templates

This directory contains deployment configurations for various Platform-as-a-Service (PaaS) providers.

## Quick Start

### Local Development with Docker Compose

For local development and testing:

```bash
cd deployments/docker-compose
docker-compose up -d
```

The application will be available at http://localhost:8080

### Cloud Deployment

Use the Rust CLI tool to generate deployment configurations for cloud platforms:

```bash
# Build the CLI tool
cd rust && cargo build --release --bin just-storage-deploy

# Generate deployment configuration
cargo run --release --bin just-storage-deploy -- generate <platform>

# Or use the binary directly after building
./target/release/just-storage-deploy generate <platform>
```

Supported platforms: `caprover`, `heroku`, `flyio`, `railway`, `render`, `digitalocean`

### CLI Commands

```bash
# Generate configuration for a platform
just-storage-deploy generate <platform> [--output <dir>] [--app-name <name>] [--region <region>]

# Validate a configuration file
just-storage-deploy validate <platform> --config <path>

# List all supported platforms
just-storage-deploy platforms

# Generate environment variables template
just-storage-deploy env [--output <path>]
```

**Examples:**

```bash
# Generate Heroku configuration
just-storage-deploy generate heroku

# Generate Fly.io config with custom app name and region
just-storage-deploy generate flyio --app-name my-app --region ord

# Validate a fly.toml file
just-storage-deploy validate flyio --config fly.toml

# Generate .env.example file
just-storage-deploy env --output .env.example
```

## Platform-Specific Guides

### Docker Compose (Local Development)

**Directory:** `docker-compose/`

**Files:**
- `docker-compose/docker-compose.yml` - Main compose configuration
- `docker-compose/docker-compose.env` - Environment variables template
- `docker-compose/README.md` - Detailed setup instructions

**Deployment Steps:**

1. Ensure Docker and Docker Compose are installed

2. Start the services:
   ```bash
   cd deployments/docker-compose
   docker-compose up -d
   ```

3. Access the application:
   - JustStorage API: http://localhost:8080
   - Health check: http://localhost:8080/health
   - pgAdmin (optional): http://localhost:8081

**Environment Variables:**
All variables are pre-configured in `docker-compose.env`. For production use:
- Set strong `JWT_SECRET` and `API_KEYS`
- Set `DISABLE_AUTH=false`
- Configure database credentials

**Documentation:** See `docker-compose/README.md`

---

### CapRover

**Files:**
- `caprover/captain-definition` - CapRover deployment definition

**Deployment Steps:**

1. Generate configuration using the CLI:
   ```bash
   just-storage-deploy generate caprover
   ```

2. Ensure your Dockerfile is in the project root

3. Deploy using CapRover CLI:
   ```bash
   caprover deploy
   ```

   Or upload via the CapRover web dashboard.

**Environment Variables:**
Set these in CapRover dashboard:
- `DATABASE_URL` - PostgreSQL connection string
- `HOT_STORAGE_ROOT` - Path for hot storage (default: `/data/hot`)
- `COLD_STORAGE_ROOT` - Path for cold storage (default: `/data/cold`)
- `JWT_SECRET` - Secret for JWT validation
- `API_KEYS` - Comma-separated API keys

**Documentation:** https://caprover.com/docs/

---

### Heroku

**Files:**
- `heroku/Procfile` - Process definition
- `heroku/rust-toolchain` - Rust version specification

**Deployment Steps:**

1. Generate configuration using the CLI:
   ```bash
   just-storage-deploy generate heroku
   ```

2. Install Heroku CLI and login:
   ```bash
   heroku login
   ```

3. Create app with Rust buildpack:
   ```bash
   heroku create your-app-name --buildpack emk/rust
   ```

4. Add PostgreSQL addon:
   ```bash
   heroku addons:create heroku-postgresql:mini
   ```

5. Set environment variables:
   ```bash
   heroku config:set JWT_SECRET=your-secret-key
   heroku config:set API_KEYS=key1,key2,key3
   heroku config:set HOT_STORAGE_ROOT=/app/data/hot
   heroku config:set COLD_STORAGE_ROOT=/app/data/cold
   ```

6. Deploy:
   ```bash
   git push heroku main
   ```

**Note:** Heroku uses ephemeral filesystem. For persistent storage, consider using S3 or another storage service.

**Documentation:** https://devcenter.heroku.com/articles/getting-started-with-rust

---

### Fly.io

**Files:**
- `flyio/fly.toml` - Fly.io configuration

**Deployment Steps:**

1. Install Fly CLI:
   ```bash
   curl -L https://fly.io/install.sh | sh
   ```

2. Login to Fly.io:
   ```bash
   fly auth login
   ```

3. Generate configuration using the CLI:
   ```bash
   just-storage-deploy generate flyio --app-name just-storage --region iad
   ```

4. Create app (if not exists):
   ```bash
   fly apps create just-storage
   ```

5. Create volume for persistent storage:
   ```bash
   fly volumes create just_storage_data --size 10 --region iad
   ```

6. Set secrets:
   ```bash
   fly secrets set JWT_SECRET=your-secret-key
   fly secrets set API_KEYS=key1,key2,key3
   fly secrets set DATABASE_URL=your-database-url
   ```

7. Deploy:
   ```bash
   fly deploy
   ```

**Documentation:** https://fly.io/docs/

---

### Railway

**Files:**
- `railway/railway.json` - Railway configuration (optional)

**Deployment Steps:**

1. Install Railway CLI:
   ```bash
   npm i -g @railway/cli
   ```

2. Login:
   ```bash
   railway login
   ```

3. Initialize project:
   ```bash
   railway init
   ```

4. Add PostgreSQL service:
   ```bash
   railway add postgresql
   ```

5. Set environment variables in Railway dashboard:
   - `JWT_SECRET`
   - `API_KEYS`
   - `HOT_STORAGE_ROOT=/app/data/hot`
   - `COLD_STORAGE_ROOT=/app/data/cold`

6. Deploy:
   ```bash
   railway up
   ```

**Note:** Railway automatically detects Dockerfile. The `railway.json` is optional for advanced configuration.

**Documentation:** https://docs.railway.app/

---

### Render

**Files:**
- `render/render.yaml` - Render service configuration

**Deployment Steps:**

1. Generate configuration using the CLI:
   ```bash
   just-storage-deploy generate render --app-name just-storage --region oregon
   ```

2. Sign up/login to Render: https://render.com

3. Create a new Blueprint from your repository

4. Render will automatically:
   - Detect the `render.yaml` file
   - Create web service
   - Create PostgreSQL database
   - Set up environment variables

5. Customize environment variables in Render dashboard:
   - `JWT_SECRET`
   - `API_KEYS`

**Documentation:** https://render.com/docs

---

### DigitalOcean App Platform

**Files:**
- `.do/app.yaml` - DigitalOcean App Platform configuration

**Deployment Steps:**

1. Generate configuration using the CLI:
   ```bash
   just-storage-deploy generate digitalocean --app-name just-storage --region nyc
   ```

2. Update `.do/app.yaml` with your GitHub repository:
   ```yaml
   github:
     repo: yourusername/just_storage
     branch: main
   ```

3. **Option A: One-Click Deploy Button**
   - Click the "Deploy to DigitalOcean" button in the README
   - Connect your GitHub repository
   - DigitalOcean will auto-detect `.do/app.yaml`

4. **Option B: Manual Deploy**
   ```bash
   # Install DigitalOcean CLI
   doctl apps create --spec .do/app.yaml
   ```

5. Set secrets in DigitalOcean dashboard:
   - `JWT_SECRET`
   - `API_KEYS`
   - `DATABASE_URL` (auto-created if using managed database)

**Environment Variables:**
- Managed PostgreSQL is automatically created
- Set `JWT_SECRET` and `API_KEYS` as secrets in dashboard
- Other variables are set in `app.yaml`

**Documentation:** https://docs.digitalocean.com/products/app-platform/

---

## Environment Variables Reference

All platforms require these environment variables:

| Variable | Description | Required | Default |
|----------|-------------|----------|----------|
| `DATABASE_URL` | PostgreSQL connection string | Yes | - |
| `JWT_SECRET` | Secret for JWT validation | Yes (production) | - |
| `API_KEYS` | Comma-separated API keys | Yes (production) | - |
| `HOT_STORAGE_ROOT` | Hot storage path | No | `/data/hot` |
| `COLD_STORAGE_ROOT` | Cold storage path | No | `/data/cold` |
| `PORT` | Server port (auto-set by PaaS) | No | `8080` |
| `LISTEN_ADDR` | Server bind address | No | `0.0.0.0:8080` |
| `GC_INTERVAL_SECS` | GC interval | No | `60` |
| `GC_BATCH_SIZE` | GC batch size | No | `100` |
| `RUST_LOG` | Log level | No | `info` |
| `DISABLE_AUTH` | Disable auth (dev only) | No | `false` |

## Storage Considerations

### Ephemeral Storage (Heroku, some Fly.io plans)
- Files are lost on restart
- Use external storage (S3, GCS) for production
- Good for development/testing

### Persistent Volumes (Fly.io, Railway, Render, DigitalOcean)
- Files persist across restarts
- Suitable for production workloads
- Configure volume mounts in platform settings

## Database Setup

All platforms support PostgreSQL. Options:

1. **Managed PostgreSQL** (Recommended)
   - Heroku Postgres
   - Fly.io Postgres
   - Railway PostgreSQL
   - Render PostgreSQL
   - DigitalOcean Managed Databases

2. **External Database**
   - Set `DATABASE_URL` to your external database
   - Ensure network access is configured

## Security Checklist

Before deploying to production:

- [ ] Set strong `JWT_SECRET` (use random generator)
- [ ] Configure `API_KEYS` with secure keys
- [ ] Remove `DISABLE_AUTH` or set to `false`
- [ ] Use HTTPS (most PaaS providers enable by default)
- [ ] Configure database backups
- [ ] Set up monitoring and alerts
- [ ] Review platform security best practices

## Troubleshooting

### Port Issues
- Most PaaS platforms set `PORT` automatically
- JustStorage reads `PORT` or falls back to `LISTEN_ADDR`

### Database Connection
- Verify `DATABASE_URL` is correct
- Check database is accessible from your app
- Ensure SSL is configured if required

### Storage Issues
- Verify volume mounts are configured
- Check file permissions
- Ensure sufficient disk space

### Build Failures
- Check Rust version compatibility
- Verify all dependencies in `Cargo.toml`
- Review build logs for specific errors

## Getting Help

- **Documentation**: See main [README.md](../README.md)
- **Issues**: Check [TROUBLESHOOTING.md](../docs/TROUBLESHOOTING.md)
- **API Reference**: See [API.md](../docs/API.md)
