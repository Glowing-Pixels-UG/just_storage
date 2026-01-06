# JustStorage Docker Compose Setup

This directory contains a Docker Compose configuration for running JustStorage locally with PostgreSQL database.

## Quick Start

1. **Prerequisites:**
   - Docker and Docker Compose installed
   - At least 2GB free RAM
   - At least 5GB free disk space

2. **Start the services:**
   ```bash
   cd deployments/docker-compose
   docker-compose up -d
   ```

3. **Check the status:**
   ```bash
   docker-compose ps
   ```

4. **View logs:**
   ```bash
   docker-compose logs -f just-storage
   ```

5. **Access the application:**
   - JustStorage API: http://localhost:8080
   - Health check: http://localhost:8080/health
   - pgAdmin (optional): http://localhost:8081

## Services

### just-storage
- **Port:** 8080
- **Health Check:** `/health`
- **Persistent Volumes:**
  - `hot_storage`: Frequently accessed data
  - `cold_storage`: Infrequently accessed data

### db (PostgreSQL)
- **Port:** 5432
- **Database:** just_storage
- **User:** postgres
- **Password:** password
- **Persistent Volume:** `postgres_data`

### pgadmin (Optional)
- **Port:** 8081
- **Email:** admin@juststorage.com
- **Password:** password
- **Purpose:** Database administration interface

## Configuration

### Environment Variables

Copy `docker-compose.env` to `.env` and modify as needed:

```bash
cp docker-compose.env .env
```

Key configuration options:
- `JWT_SECRET`: Change for production use
- `API_KEYS`: Configure API keys for authentication
- `DISABLE_AUTH`: Set to `false` for production
- `RUST_LOG`: Set to `debug` for more detailed logging

### Scaling

To run multiple instances of JustStorage:

```bash
docker-compose up -d --scale just-storage=3
```

Note: Load balancing is not included in this setup. Use a reverse proxy like nginx or traefik for production scaling.

## Database Management

### Using pgAdmin

1. Open http://localhost:8081
2. Login with admin@juststorage.com / password
3. Add server:
   - Host: db
   - Port: 5432
   - Username: postgres
   - Password: password
   - Database: just_storage

### Direct PostgreSQL Access

```bash
docker-compose exec db psql -U postgres -d just_storage
```

## Development Workflow

### Building Custom Images

To build with local changes:

```bash
# From the project root
docker-compose -f deployments/docker-compose/docker-compose.yml build
```

### Database Migrations

The application automatically runs database migrations on startup.

### Logs and Debugging

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f just-storage

# Database logs
docker-compose logs -f db
```

## Production Considerations

This Docker Compose setup is intended for development and testing. For production:

1. **Security:**
   - Change all default passwords
   - Use strong JWT secrets
   - Configure proper authentication
   - Enable SSL/TLS

2. **Persistence:**
   - Use named volumes or bind mounts for data persistence
   - Consider backup strategies for the database

3. **Monitoring:**
   - Add monitoring services (Prometheus, Grafana)
   - Configure log aggregation

4. **Scaling:**
   - Use Docker Swarm or Kubernetes for orchestration
   - Implement load balancing
   - Configure session affinity if needed

## Troubleshooting

### Common Issues

1. **Port conflicts:**
   - Change ports in docker-compose.yml if 8080/5432/8081 are in use

2. **Permission issues:**
   - Ensure Docker has access to project files
   - Check file permissions on mounted volumes

3. **Database connection issues:**
   - Wait for database to fully start (check logs)
   - Verify DATABASE_URL configuration

4. **Out of memory:**
   - Increase Docker memory limit
   - Reduce DB_MAX_CONNECTIONS if needed

### Reset Everything

To completely reset the environment:

```bash
docker-compose down -v
docker-compose up -d
```

This will remove all data and restart fresh.