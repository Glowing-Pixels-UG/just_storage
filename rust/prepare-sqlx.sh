#!/bin/bash
set -e

echo "Starting testcontainer for SQLx offline preparation..."

# Start a temporary PostgreSQL container
CONTAINER_ID=$(docker run -d \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_DB=postgres \
  -p 54321:5432 \
  postgres:17-alpine)

echo "Waiting for PostgreSQL to be ready..."
sleep 8

# Set database URL
export DATABASE_URL="postgres://postgres:postgres@localhost:54321/postgres"

# Run migrations (this will create all tables)
echo "Running migrations..."
cargo sqlx migrate run --database-url "$DATABASE_URL"

# Prepare SQLx offline data
echo "Preparing SQLx offline query cache..."
cargo sqlx prepare --database-url "$DATABASE_URL"

# Cleanup
echo "Cleaning up..."
docker stop $CONTAINER_ID
docker rm $CONTAINER_ID

echo "SQLx offline cache prepared successfully!"
