.PHONY: help build test run clean docker-build docker-up docker-down fmt clippy check

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

build: ## Build release binary
	cd rust && cargo build --release

test: ## Run tests
	cd rust && cargo test

run: ## Run service locally
	cd rust && cargo run --release

clean: ## Clean build artifacts
	cd rust && cargo clean
	rm -rf data/hot/* data/cold/*

fmt: ## Format code
	cd rust && cargo fmt

clippy: ## Run clippy lints
	cd rust && cargo clippy -- -D warnings

check: ## Check compilation
	cd rust && cargo check

docker-build: ## Build Docker image
	docker-compose build

docker-up: ## Start services with Docker Compose
	docker-compose up -d

docker-down: ## Stop Docker Compose services
	docker-compose down

docker-logs: ## Show Docker Compose logs
	docker-compose logs -f activestorage

db-setup: ## Create database and run migrations
	createdb activestorage || true
	psql activestorage < schema.sql

db-reset: ## Drop and recreate database
	dropdb activestorage || true
	make db-setup

dev: ## Start development environment
	@echo "Starting PostgreSQL..."
	docker-compose up -d postgres
	@echo "Waiting for database..."
	@sleep 3
	@echo "Starting service..."
	cd rust && cargo run

all: fmt clippy test build ## Format, lint, test, and build
