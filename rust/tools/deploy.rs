//! JustStorage Deployment CLI Tool
//!
//! This tool helps generate and manage deployment configurations for various PaaS platforms.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "just-storage-deploy")]
#[command(about = "JustStorage deployment configuration generator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate deployment configuration for a specific platform
    Generate {
        /// Platform to generate config for
        #[arg(value_enum)]
        platform: Platform,
        /// Output directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        /// App name (for platforms that require it)
        #[arg(short, long)]
        app_name: Option<String>,
        /// Region (for platforms that support it)
        #[arg(short, long)]
        region: Option<String>,
    },
    /// Validate deployment configuration
    Validate {
        /// Platform to validate config for
        #[arg(value_enum)]
        platform: Platform,
        /// Path to configuration file
        #[arg(short, long)]
        config: PathBuf,
    },
    /// List all supported platforms
    Platforms,
    /// Show environment variables template
    Env {
        /// Output file (default: .env.example)
        #[arg(short, long, default_value = ".env.example")]
        output: PathBuf,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum Platform {
    Caprover,
    Heroku,
    Flyio,
    Railway,
    Render,
    Digitalocean,
    DockerCompose,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            platform,
            output,
            app_name,
            region,
        } => generate_config(&platform, &output, app_name.as_deref(), region.as_deref())?,
        Commands::Validate { platform, config } => validate_config(&platform, &config)?,
        Commands::Platforms => list_platforms(),
        Commands::Env { output } => generate_env_template(&output)?,
    }

    Ok(())
}

fn generate_config(
    platform: &Platform,
    output_dir: &Path,
    app_name: Option<&str>,
    region: Option<&str>,
) -> Result<()> {
    println!("Generating deployment configuration for {:?}...", platform);

    // Ensure output directory exists
    fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {:?}", output_dir))?;

    match platform {
        Platform::Caprover => generate_caprover_config(output_dir)?,
        Platform::Heroku => generate_heroku_config(output_dir)?,
        Platform::Flyio => generate_flyio_config(output_dir, app_name, region)?,
        Platform::Railway => generate_railway_config(output_dir)?,
        Platform::Render => generate_render_config(output_dir, app_name, region)?,
        Platform::Digitalocean => generate_digitalocean_config(output_dir, app_name, region)?,
        Platform::DockerCompose => generate_docker_compose_config(output_dir)?,
    }

    println!(
        "✅ Configuration generated successfully in {:?}",
        output_dir
    );
    println!("\nNext steps:");
    print_next_steps(platform);

    Ok(())
}

fn generate_caprover_config(output_dir: &Path) -> Result<()> {
    let content = r#"{
  "schemaVersion": 2,
  "dockerfilePath": "./Dockerfile",
  "serviceName": "just-storage",
  "appName": "just-storage",
  "appVersion": "1.0.0",
  "description": "Content-addressable object storage service with strong consistency guarantees",
  "ports": [
    {
      "protocol": "http",
      "containerPort": 8080,
      "hostPort": 80,
      "httpOptions": {
        "redirectToHttps": true,
        "baseUrl": "/"
      }
    },
    {
      "protocol": "https",
      "containerPort": 8080,
      "hostPort": 443,
      "httpOptions": {
        "baseUrl": "/"
      }
    }
  ],
  "environments": [
    {
      "key": "RUST_LOG",
      "value": "info",
      "description": "Log level for the application"
    },
    {
      "key": "GC_INTERVAL_SECS",
      "value": "60",
      "description": "Garbage collection interval in seconds"
    },
    {
      "key": "GC_BATCH_SIZE",
      "value": "100",
      "description": "Number of blobs to process per GC cycle"
    },
    {
      "key": "HOT_STORAGE_ROOT",
      "value": "/data/hot",
      "description": "Path for hot storage"
    },
    {
      "key": "COLD_STORAGE_ROOT",
      "value": "/data/cold",
      "description": "Path for cold storage"
    },
    {
      "key": "DB_MAX_CONNECTIONS",
      "value": "20",
      "description": "Maximum database connections"
    },
    {
      "key": "DB_MIN_CONNECTIONS",
      "value": "5",
      "description": "Minimum database connections"
    },
    {
      "key": "DB_ACQUIRE_TIMEOUT_SECS",
      "value": "30",
      "description": "Database connection acquire timeout"
    },
    {
      "key": "DB_IDLE_TIMEOUT_SECS",
      "value": "600",
      "description": "Database connection idle timeout"
    },
    {
      "key": "DB_MAX_LIFETIME_SECS",
      "value": "1800",
      "description": "Maximum database connection lifetime"
    },
    {
      "key": "JWT_SECRET",
      "value": "",
      "description": "Secret key for JWT validation (REQUIRED - set in CapRover dashboard)",
      "required": true
    },
    {
      "key": "API_KEYS",
      "value": "",
      "description": "Comma-separated API keys (REQUIRED - set in CapRover dashboard)",
      "required": true
    }
  ],
  "volumes": [
    {
      "volumeName": "just_storage_hot_data",
      "containerPath": "/data/hot",
      "volumeType": "local",
      "description": "Hot storage volume for frequently accessed data"
    },
    {
      "volumeName": "just_storage_cold_data",
      "containerPath": "/data/cold",
      "volumeType": "local",
      "description": "Cold storage volume for infrequently accessed data"
    }
  ],
  "captainsLog": {
    "enabled": true,
    "maxSize": "10m"
  },
  "healthcheck": {
    "enabled": true,
    "path": "/health",
    "interval": "30s",
    "timeout": "5s",
    "retries": 3
  }
}
"#;

    let file_path = output_dir.join("captain-definition");
    fs::write(&file_path, content)
        .with_context(|| format!("Failed to write captain-definition to {:?}", file_path))?;

    println!("Created: captain-definition");
    Ok(())
}

fn generate_heroku_config(output_dir: &Path) -> Result<()> {
    // Procfile
    let procfile_content = "web: ./target/release/just_storage\n";
    let procfile_path = output_dir.join("Procfile");
    fs::write(&procfile_path, procfile_content)
        .with_context(|| format!("Failed to write Procfile to {:?}", procfile_path))?;
    println!("Created: Procfile");

    // app.json
    let app_json_content = r#"{
  "name": "JustStorage",
  "description": "Content-addressable object storage service with strong consistency guarantees",
  "repository": "https://github.com/yourusername/just_storage",
  "keywords": ["rust", "storage", "object-storage", "content-addressable", "api"],
  "success_url": "/health",
  "website": "https://github.com/yourusername/just_storage",
  "buildpacks": [
    {
      "url": "https://github.com/emk/heroku-buildpack-rust.git"
    }
  ],
  "formation": {
    "web": {
      "quantity": 1,
      "size": "standard-1x"
    }
  },
  "addons": [
    {
      "plan": "heroku-postgresql:mini",
      "options": {
        "version": "15"
      }
    }
  ],
  "env": {
    "RUST_LOG": {
      "description": "Log level for the application",
      "value": "info"
    },
    "GC_INTERVAL_SECS": {
      "description": "Garbage collection interval in seconds",
      "value": "60"
    },
    "GC_BATCH_SIZE": {
      "description": "Number of blobs to process per GC cycle",
      "value": "100"
    },
    "HOT_STORAGE_ROOT": {
      "description": "Path for hot storage (ephemeral on Heroku)",
      "value": "/tmp/hot"
    },
    "COLD_STORAGE_ROOT": {
      "description": "Path for cold storage (ephemeral on Heroku)",
      "value": "/tmp/cold"
    },
    "DB_MAX_CONNECTIONS": {
      "description": "Maximum database connections",
      "value": "10"
    },
    "DB_MIN_CONNECTIONS": {
      "description": "Minimum database connections",
      "value": "2"
    },
    "DB_ACQUIRE_TIMEOUT_SECS": {
      "description": "Database connection acquire timeout",
      "value": "30"
    },
    "DB_IDLE_TIMEOUT_SECS": {
      "description": "Database connection idle timeout",
      "value": "600"
    },
    "DB_MAX_LIFETIME_SECS": {
      "description": "Maximum database connection lifetime",
      "value": "1800"
    },
    "JWT_SECRET": {
      "description": "Secret key for JWT validation (REQUIRED for production)",
      "generator": "secret"
    },
    "API_KEYS": {
      "description": "Comma-separated API keys (REQUIRED for production)"
    },
    "DISABLE_AUTH": {
      "description": "Disable authentication (development only, DO NOT USE IN PRODUCTION)",
      "value": "false"
    }
  },
  "scripts": {
    "postdeploy": "echo 'JustStorage deployed successfully! Visit /health to verify.' && echo 'Note: Heroku uses ephemeral storage. For persistent storage, consider using S3 or another cloud storage service.'"
  },
  "stack": "heroku-22"
}
"#;
    let app_json_path = output_dir.join("app.json");
    fs::write(&app_json_path, app_json_content)
        .with_context(|| format!("Failed to write app.json to {:?}", app_json_path))?;
    println!("Created: app.json");

    // rust-toolchain
    let toolchain_content = "1.85\n";
    let toolchain_path = output_dir.join("rust-toolchain");
    fs::write(&toolchain_path, toolchain_content)
        .with_context(|| format!("Failed to write rust-toolchain to {:?}", toolchain_path))?;
    println!("Created: rust-toolchain");

    Ok(())
}

fn generate_flyio_config(
    output_dir: &Path,
    app_name: Option<&str>,
    region: Option<&str>,
) -> Result<()> {
    let app_name = app_name.unwrap_or("just-storage");
    let region = region.unwrap_or("iad");

    let content = format!(
        r#"# Fly.io configuration for JustStorage
# See: https://fly.io/docs/reference/configuration/

app = "{}"
primary_region = "{}"

[build]
  dockerfile = "Dockerfile"

[env]
  RUST_LOG = "info"
  GC_INTERVAL_SECS = "60"
  GC_BATCH_SIZE = "100"
  DB_MAX_CONNECTIONS = "20"
  DB_MIN_CONNECTIONS = "5"
  DB_ACQUIRE_TIMEOUT_SECS = "30"
  DB_IDLE_TIMEOUT_SECS = "600"
  DB_MAX_LIFETIME_SECS = "1800"

[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = true
  auto_start_machines = true
  min_machines_running = 1
  max_machines_running = 3
  processes = ["app"]

  # Concurrency settings
  concurrency = {{ type = "connections", hard_limit = 1000, soft_limit = 500 }}

  # HTTP/2 and other optimizations
  http_options = {{ h2_backend = true, compress = true }}

  # TLS configuration
  tls_options = {{ alpn = ["h2", "http/1.1"], versions = ["TLSv1.3", "TLSv1.2"] }}

[[http_service.checks]]
  grace_period = "30s"
  interval = "30s"
  method = "GET"
  timeout = "10s"
  path = "/health"
  headers = {{ Host = "{}.fly.dev" }}

# Readiness check
[[http_service.checks]]
  grace_period = "10s"
  interval = "15s"
  method = "GET"
  timeout = "5s"
  path = "/api/health"
  headers = {{ Host = "{}.fly.dev" }}

[[vm]]
  cpu_kind = "shared"
  cpus = 1
  memory_mb = 1024

# Persistent storage for data
[mounts]
  source = "just_storage_data"
  destination = "/data"
  initial_size = "10gb"

# Database (optional - can use managed PostgreSQL)
# [[fly_postgres.attachments]]
#   database_name = "just_storage"
#   database_user = "just_storage"
#   name = "just_storage_db"

# Metrics and monitoring
[metrics]
  port = 9090
  path = "/metrics"

# Processes configuration
[processes]
  app = "/app/just_storage"

# Health checks for the app process
[checks]
  [checks.app_health]
    port = 8080
    type = "http"
    interval = "30s"
    timeout = "10s"
    method = "GET"
    path = "/health"
    http_headers = {{ "Host" = "{}.fly.dev" }}

# Restart policy
[restart]
  policy = "on-failure"
  max_retries = 3
"#,
        app_name, region, app_name, app_name, app_name
    );

    let file_path = output_dir.join("fly.toml");
    fs::write(&file_path, content)
        .with_context(|| format!("Failed to write fly.toml to {:?}", file_path))?;

    println!("Created: fly.toml");
    Ok(())
}

fn generate_railway_config(output_dir: &Path) -> Result<()> {
    let content = r#"# Railway configuration for JustStorage
# Railway auto-detects Dockerfile, but this file can be used for advanced configuration
# See: https://docs.railway.app/reference/config

[build]
builder = "DOCKERFILE"
dockerfilePath = "Dockerfile"
buildCommand = "docker build -t just-storage ."
dockerfileTarget = ""

[deploy]
startCommand = "./target/release/just_storage"
restartPolicyType = "ON_FAILURE"
restartPolicyMaxRetries = 10
healthcheckPath = "/health"
healthcheckTimeout = 300
healthcheckInterval = 30
healthcheckStartPeriod = 10

# Environment variables
[environments]
  RUST_LOG = "info"
  GC_INTERVAL_SECS = "60"
  GC_BATCH_SIZE = "100"
  DB_MAX_CONNECTIONS = "20"
  DB_MIN_CONNECTIONS = "5"
  DB_ACQUIRE_TIMEOUT_SECS = "30"
  DB_IDLE_TIMEOUT_SECS = "600"
  DB_MAX_LIFETIME_SECS = "1800"
  HOT_STORAGE_ROOT = "/app/data/hot"
  COLD_STORAGE_ROOT = "/app/data/cold"

# Resource allocation
[resources]
  cpu = 1000
  memory = 1024

# Volumes for persistent storage
[volumes]
  hot_storage = "/app/data/hot"
  cold_storage = "/app/data/cold"

# Network configuration
[network]
  tcpProxyApplicationPort = 8080
  customDomain = ""

# Monitoring and logging
[monitoring]
  enableMetrics = true
  enableLogs = true
"#;

    let file_path = output_dir.join("railway.toml");
    fs::write(&file_path, content)
        .with_context(|| format!("Failed to write railway.toml to {:?}", file_path))?;

    println!("Created: railway.toml");
    Ok(())
}

fn generate_render_config(
    output_dir: &Path,
    app_name: Option<&str>,
    region: Option<&str>,
) -> Result<()> {
    let app_name = app_name.unwrap_or("just-storage");
    let region = region.unwrap_or("oregon");

    let content = format!(
        r#"# Render.com configuration for JustStorage
# Generated by just-storage-deploy
# See: https://render.com/docs/yaml-spec

services:
  - type: web
    name: {}
    env: docker
    dockerfilePath: Dockerfile
    dockerContext: .
    plan: starter
    region: {}
    runtime: docker

    # Environment variables
    envVars:
      - key: RUST_LOG
        value: info
      - key: GC_INTERVAL_SECS
        value: "60"
      - key: GC_BATCH_SIZE
        value: "100"
      - key: DB_MAX_CONNECTIONS
        value: "20"
      - key: DB_MIN_CONNECTIONS
        value: "5"
      - key: DB_ACQUIRE_TIMEOUT_SECS
        value: "30"
      - key: DB_IDLE_TIMEOUT_SECS
        value: "600"
      - key: DB_MAX_LIFETIME_SECS
        value: "1800"
      - key: HOT_STORAGE_ROOT
        value: /app/data/hot
      - key: COLD_STORAGE_ROOT
        value: /app/data/cold
      - key: JWT_SECRET
        sync: false
      - key: API_KEYS
        sync: false

    # Health checks
    healthCheckPath: /health
    healthCheckTimeout: 30

    # Resource limits
    disk:
      name: {}-disk
      mountPath: /app/data
      sizeGB: 10

    # Auto-scaling (paid plans only)
    # minInstances: 1
    # maxInstances: 3

    # Build settings
    buildCommand: docker build -t just-storage .
    dockerCommand: ./target/release/just_storage

    # Networking
    httpHeaders:
      - name: X-Frame-Options
        value: DENY
      - name: X-Content-Type-Options
        value: nosniff
      - name: Referrer-Policy
        value: strict-origin-when-cross-origin
      - name: Permissions-Policy
        value: geolocation=(), microphone=(), camera=()

    autoDeploy: true

  # PostgreSQL database
  - type: pgsql
    name: {}-db
    plan: starter
    region: {}
    databaseName: just_storage
    databaseUser: just_storage
    postgresMajorVersion: 15

    # Database configuration
    disk:
      name: {}-db-disk
      sizeGB: 10

    # Environment variables for the web service to connect
    envVars:
      - key: DATABASE_URL
        fromService:
          type: pgsql
          name: {}-db
          property: connectionString

# Environment groups for different deployment environments
environments:
  - name: production
    envVars:
      - key: RUST_LOG
        value: warn
      - key: GC_INTERVAL_SECS
        value: "30"
  - name: staging
    envVars:
      - key: RUST_LOG
        value: info
      - key: GC_INTERVAL_SECS
        value: "60"
"#,
        app_name, region, app_name, app_name, region, app_name, app_name
    );

    let file_path = output_dir.join("render.yaml");
    fs::write(&file_path, content)
        .with_context(|| format!("Failed to write render.yaml to {:?}", file_path))?;

    println!("Created: render.yaml");
    Ok(())
}

fn generate_digitalocean_config(
    output_dir: &Path,
    app_name: Option<&str>,
    region: Option<&str>,
) -> Result<()> {
    let app_name = app_name.unwrap_or("just-storage");
    let region = region.unwrap_or("nyc");

    // Create .do directory if it doesn't exist
    let do_dir = output_dir.join(".do");
    fs::create_dir_all(&do_dir)
        .with_context(|| format!("Failed to create .do directory: {:?}", do_dir))?;

    let content = format!(
        r#"name: {}
region: {}
services:
  - name: {}
    github:
      repo: yourusername/just_storage
      branch: main
      deploy_on_push: true
    dockerfile_path: Dockerfile
    docker_build_context: .
    http_port: 8080
    instance_count: 1
    instance_size_slug: basic-xxs
    health_check:
      http_path: /health
      initial_delay_seconds: 30
      period_seconds: 10
      timeout_seconds: 5
      failure_threshold: 3
    routes:
      - path: /
    envs:
      - key: RUST_LOG
        value: info
      - key: GC_INTERVAL_SECS
        value: "60"
      - key: GC_BATCH_SIZE
        value: "100"
      - key: DB_MAX_CONNECTIONS
        value: "20"
      - key: DB_MIN_CONNECTIONS
        value: "5"
      - key: DB_ACQUIRE_TIMEOUT_SECS
        value: "30"
      - key: DB_IDLE_TIMEOUT_SECS
        value: "600"
      - key: DB_MAX_LIFETIME_SECS
        value: "1800"
      - key: HOT_STORAGE_ROOT
        value: /app/data/hot
      - key: COLD_STORAGE_ROOT
        value: /app/data/cold
      - key: PORT
        value: "8080"
      - key: JWT_SECRET
        scope: RUN_TIME
        type: SECRET
      - key: API_KEYS
        scope: RUN_TIME
        type: SECRET
      - key: DATABASE_URL
        scope: RUN_TIME
        type: SECRET

databases:
  - name: {}-db
    engine: PG
    production: false
    version: "15"
    db_name: just_storage
    db_user: just_storage
    size: professional-xs
"#,
        app_name, region, app_name, app_name
    );

    let file_path = do_dir.join("app.yaml");
    fs::write(&file_path, content)
        .with_context(|| format!("Failed to write app.yaml to {:?}", file_path))?;

    println!("Created: .do/app.yaml");
    println!("Note: Update the 'repo' field with your GitHub repository path");
    Ok(())
}

fn validate_config(platform: &Platform, config_path: &Path) -> Result<()> {
    println!(
        "Validating {:?} configuration at {:?}...",
        platform, config_path
    );

    if !config_path.exists() {
        anyhow::bail!("Configuration file not found: {:?}", config_path);
    }

    let content = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

    match platform {
        Platform::Caprover => validate_caprover(&content)?,
        Platform::Heroku => validate_heroku(&content, config_path)?,
        Platform::Flyio => validate_flyio(&content)?,
        Platform::Railway => validate_railway(&content)?,
        Platform::Render => validate_render(&content)?,
        Platform::Digitalocean => validate_digitalocean(&content)?,
        Platform::DockerCompose => validate_docker_compose(&content)?,
    }

    println!("✅ Configuration is valid!");
    Ok(())
}

fn validate_caprover(content: &str) -> Result<()> {
    let _: serde_json::Value =
        serde_json::from_str(content).context("Invalid JSON format for captain-definition")?;
    Ok(())
}

fn validate_heroku(content: &str, path: &Path) -> Result<()> {
    if path.file_name().and_then(|n| n.to_str()) == Some("Procfile") && !content.contains("web:") {
        anyhow::bail!("Procfile must contain a 'web:' process type");
    }
    Ok(())
}

fn validate_flyio(content: &str) -> Result<()> {
    let _: toml::Value = toml::from_str(content).context("Invalid TOML format for fly.toml")?;
    Ok(())
}

fn validate_railway(content: &str) -> Result<()> {
    let _: toml::Value = toml::from_str(content).context("Invalid TOML format for railway.toml")?;
    Ok(())
}

fn validate_render(content: &str) -> Result<()> {
    // Basic YAML validation
    let _: serde_yaml::Value =
        serde_yaml::from_str(content).context("Invalid YAML format for render.yaml")?;
    Ok(())
}

fn validate_digitalocean(content: &str) -> Result<()> {
    // Basic YAML validation
    let _: serde_yaml::Value =
        serde_yaml::from_str(content).context("Invalid YAML format for app.yaml")?;
    Ok(())
}

fn generate_docker_compose_config(output_dir: &Path) -> Result<()> {
    let content = r#"version: '3.8'

services:
  just-storage:
    build:
      context: ../..
      dockerfile: Dockerfile
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgres://postgres:password@db:5432/just_storage
      - HOT_STORAGE_ROOT=/data/hot
      - COLD_STORAGE_ROOT=/data/cold
      - LISTEN_ADDR=0.0.0.0:8080
      - GC_INTERVAL_SECS=60
      - GC_BATCH_SIZE=100
      - DB_MAX_CONNECTIONS=20
      - DB_MIN_CONNECTIONS=5
      - DB_ACQUIRE_TIMEOUT_SECS=30
      - DB_IDLE_TIMEOUT_SECS=600
      - DB_MAX_LIFETIME_SECS=1800
      - RUST_LOG=info
      - JWT_SECRET=development-secret-key-change-in-production
      - API_KEYS=key1,key2,key3
      - DISABLE_AUTH=false
      - REQUEST_TIMEOUT_SECS=30
      - MAX_BODY_SIZE=104857600
      - RATE_LIMIT_RPM=60
      - CORS_ALLOWED_ORIGINS=*
      - SECURITY_HEADERS_ENABLED=true
      - METRICS_ENABLED=true
      - METRICS_PORT=9090
    volumes:
      - hot_storage:/data/hot
      - cold_storage:/data/cold
    depends_on:
      - db
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    restart: unless-stopped

  db:
    image: postgres:15
    environment:
      - POSTGRES_DB=just_storage
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=password
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 30s
      timeout: 10s
      retries: 3
    restart: unless-stopped

volumes:
  hot_storage:
    driver: local
  cold_storage:
    driver: local
  postgres_data:
    driver: local

networks:
  default:
    name: just_storage_network
"#;

    let file_path = output_dir.join("docker-compose.yml");
    fs::write(&file_path, content)
        .with_context(|| format!("Failed to write docker-compose.yml to {:?}", file_path))?;

    // Generate environment file
    let env_content = r#"# Docker Compose Environment Configuration for JustStorage
# Copy this file to .env and configure for your environment

# Database
POSTGRES_DB=just_storage
POSTGRES_USER=postgres
POSTGRES_PASSWORD=password
DATABASE_URL=postgres://postgres:password@db:5432/just_storage

# Application
HOT_STORAGE_ROOT=/data/hot
COLD_STORAGE_ROOT=/data/cold
LISTEN_ADDR=0.0.0.0:8080
GC_INTERVAL_SECS=60
GC_BATCH_SIZE=100

# Database Connection Pool Settings
DB_MAX_CONNECTIONS=20
DB_MIN_CONNECTIONS=5
DB_ACQUIRE_TIMEOUT_SECS=30
DB_IDLE_TIMEOUT_SECS=600
DB_MAX_LIFETIME_SECS=1800

# Logging
RUST_LOG=info

# Authentication (REQUIRED for production)
JWT_SECRET=development-secret-key-change-in-production
API_KEYS=key1,key2,key3

# Development only - disable auth (DO NOT USE IN PRODUCTION)
DISABLE_AUTH=false

# Performance and Security Settings
REQUEST_TIMEOUT_SECS=30
MAX_BODY_SIZE=104857600
RATE_LIMIT_RPM=60
CORS_ALLOWED_ORIGINS=*
SECURITY_HEADERS_ENABLED=true

# Metrics and monitoring
METRICS_ENABLED=true
METRICS_PORT=9090
"#;

    let env_path = output_dir.join("docker-compose.env");
    fs::write(&env_path, env_content)
        .with_context(|| format!("Failed to write docker-compose.env to {:?}", env_path))?;

    println!("Created: docker-compose.yml");
    println!("Created: docker-compose.env");
    Ok(())
}

fn validate_docker_compose(content: &str) -> Result<()> {
    // Basic YAML validation
    let _: serde_yaml::Value =
        serde_yaml::from_str(content).context("Invalid YAML format for docker-compose.yml")?;
    Ok(())
}

fn list_platforms() {
    println!("Supported deployment platforms:\n");
    println!("  caprover     - CapRover (self-hosted PaaS)");
    println!("  heroku       - Heroku");
    println!("  flyio        - Fly.io");
    println!("  railway      - Railway");
    println!("  render       - Render.com");
    println!("  digitalocean - DigitalOcean App Platform");
    println!("  docker-compose - Local development with Docker Compose");
    println!("\nUse 'just-storage-deploy generate <platform>' to create configuration files.");
}

fn generate_env_template(output_path: &Path) -> Result<()> {
    let content = r#"# Environment Configuration Template for JustStorage
# Copy this file to .env and configure for your environment

# Database
DATABASE_URL=postgres://postgres:password@localhost:5432/just_storage

# Storage
HOT_STORAGE_ROOT=/data/hot
COLD_STORAGE_ROOT=/data/cold

# Server
LISTEN_ADDR=0.0.0.0:8080
# PORT is automatically detected by PaaS platforms (Heroku, Fly.io, etc.)

# Garbage Collection
GC_INTERVAL_SECS=60
GC_BATCH_SIZE=100

# Database Connection Pool Settings
DB_MAX_CONNECTIONS=20
DB_MIN_CONNECTIONS=5
DB_ACQUIRE_TIMEOUT_SECS=30
DB_IDLE_TIMEOUT_SECS=600
DB_MAX_LIFETIME_SECS=1800

# Logging
RUST_LOG=info

# Authentication (REQUIRED for production)
JWT_SECRET=your-secret-key-change-this-in-production
API_KEYS=key1,key2,key3

# Development only - disable auth (DO NOT USE IN PRODUCTION)
# DISABLE_AUTH=false

# Performance and Security Settings
# Request timeout in seconds
REQUEST_TIMEOUT_SECS=30

# Maximum request body size in bytes (100MB)
MAX_BODY_SIZE=104857600

# Rate limiting (requests per minute per IP)
RATE_LIMIT_RPM=60

# CORS settings
CORS_ALLOWED_ORIGINS=*

# Security headers
SECURITY_HEADERS_ENABLED=true

# Metrics and monitoring
METRICS_ENABLED=true
METRICS_PORT=9090

# Backup settings (if using external storage)
BACKUP_ENABLED=false
BACKUP_INTERVAL_HOURS=24
BACKUP_RETENTION_DAYS=30

# External storage (optional - for S3, GCS, etc.)
# STORAGE_BACKEND=s3
# S3_BUCKET=your-bucket-name
# S3_REGION=us-east-1
# AWS_ACCESS_KEY_ID=your-access-key
# AWS_SECRET_ACCESS_KEY=your-secret-key
"#;

    fs::write(output_path, content)
        .with_context(|| format!("Failed to write env template to {:?}", output_path))?;

    println!("✅ Environment template created at {:?}", output_path);
    Ok(())
}

fn print_next_steps(platform: &Platform) {
    match platform {
        Platform::DockerCompose => {
            println!("  1. Install Docker and Docker Compose");
            println!("  2. Run: docker-compose up -d");
            println!("  3. Access at: http://localhost:8080");
            println!("  4. Optional: pgAdmin at http://localhost:8081");
        }
        Platform::Caprover => {
            println!("  1. Ensure Dockerfile is in project root");
            println!("  2. Deploy: caprover deploy");
            println!("  3. Or upload via CapRover web dashboard");
        }
        Platform::Heroku => {
            println!("  1. Install Heroku CLI: https://devcenter.heroku.com/articles/heroku-cli");
            println!("  2. Login: heroku login");
            println!("  3. Create app: heroku create --buildpack emk/rust");
            println!("  4. Add PostgreSQL: heroku addons:create heroku-postgresql:mini");
            println!("  5. Set secrets: heroku config:set JWT_SECRET=... API_KEYS=...");
            println!("  6. Deploy: git push heroku main");
        }
        Platform::Flyio => {
            println!("  1. Install Fly CLI: curl -L https://fly.io/install.sh | sh");
            println!("  2. Login: fly auth login");
            println!("  3. Create app: fly apps create");
            println!("  4. Create volume: fly volumes create just_storage_data --size 10");
            println!("  5. Set secrets: fly secrets set JWT_SECRET=... API_KEYS=...");
            println!("  6. Deploy: fly deploy");
        }
        Platform::Railway => {
            println!("  1. Install Railway CLI: npm i -g @railway/cli");
            println!("  2. Login: railway login");
            println!("  3. Initialize: railway init");
            println!("  4. Add PostgreSQL: railway add postgresql");
            println!("  5. Set environment variables in Railway dashboard");
            println!("  6. Deploy: railway up");
        }
        Platform::Render => {
            println!("  1. Sign up at https://render.com");
            println!("  2. Create new Blueprint from your repository");
            println!("  3. Render will auto-detect render.yaml");
            println!("  4. Customize environment variables in dashboard");
            println!("  5. Deploy automatically on git push");
        }
        Platform::Digitalocean => {
            println!("  1. Sign up at https://cloud.digitalocean.com");
            println!("  2. Click 'Deploy to DigitalOcean' button in README");
            println!("  3. Or create app manually: doctl apps create --spec .do/app.yaml");
            println!("  4. Set secrets in DigitalOcean dashboard:");
            println!("     - JWT_SECRET");
            println!("     - API_KEYS");
            println!("     - DATABASE_URL (auto-created if using managed DB)");
            println!("  5. Deploy automatically on git push");
        }
    }
    println!("\nFor detailed instructions, see: deployments/README.md");
}
