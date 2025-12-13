.PHONY: help build test test-nextest run clean docker-build docker-up docker-down fmt fmt-check clippy clippy-all check lint security audit deny udeps bloat miri install-tools

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

build: ## Build release binary
	cd rust && cargo build --release

test: ## Run tests
	cd rust && cargo test

test-nextest: ## Run tests with nextest (faster parallel test runner)
	@command -v cargo-nextest >/dev/null 2>&1 || { echo "Installing cargo-nextest..."; cargo install cargo-nextest --locked; }
	cd rust && cargo nextest run

run: ## Run service locally
	cd rust && cargo run --release

clean: ## Clean build artifacts
	cd rust && cargo clean
	rm -rf data/hot/* data/cold/*

fmt: ## Format code
	cd rust && cargo fmt
	cd binary-container-poc && cargo fmt

fmt-check: ## Check code formatting without modifying files
	cd rust && cargo fmt -- --check
	cd binary-container-poc && cargo fmt -- --check

clippy: ## Run clippy lints (basic)
	cd rust && cargo clippy -- -D warnings
	cd binary-container-poc && cargo clippy -- -D warnings

clippy-all: ## Run clippy with all lints enabled
	cd rust && cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic -W clippy::nursery
	cd binary-container-poc && cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic -W clippy::nursery

check: ## Check compilation
	cd rust && cargo check
	cd binary-container-poc && cargo check

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

# Security and dependency analysis
security: audit deny ## Run all security checks (audit + deny)

audit: ## Run cargo-audit for security vulnerabilities
	@command -v cargo-audit >/dev/null 2>&1 || { echo "Installing cargo-audit..."; cargo install cargo-audit --locked; }
	cd rust && cargo audit
	cd binary-container-poc && cargo audit

deny: ## Run cargo-deny for comprehensive dependency checks
	@command -v cargo-deny >/dev/null 2>&1 || { echo "Installing cargo-deny..."; cargo install cargo-deny --locked; }
	cd rust && cargo deny check
	cd binary-container-poc && cargo deny check

udeps: ## Check for unused dependencies
	@command -v cargo-udeps >/dev/null 2>&1 || { echo "Installing cargo-udeps..."; cargo install cargo-udeps --locked; }
	cd rust && cargo +nightly udeps
	cd binary-container-poc && cargo +nightly udeps

bloat: ## Analyze binary size with cargo-bloat
	@command -v cargo-bloat >/dev/null 2>&1 || { echo "Installing cargo-bloat..."; cargo install cargo-bloat --locked; }
	cd rust && cargo bloat --release
	cd binary-container-poc && cargo bloat --release

miri: ## Run tests with Miri (undefined behavior detection)
	@rustup +nightly component add miri 2>/dev/null || true
	cd rust && cargo +nightly miri test
	cd binary-container-poc && cargo +nightly miri test

# Comprehensive linting target
lint: fmt-check clippy security ## Run all linting and static analysis checks

# Install all required tools
install-tools: ## Install all recommended Rust development tools
	@echo "Installing Rust development tools..."
	@command -v cargo-audit >/dev/null 2>&1 || cargo install cargo-audit --locked
	@command -v cargo-deny >/dev/null 2>&1 || cargo install cargo-deny --locked
	@command -v cargo-udeps >/dev/null 2>&1 || cargo install cargo-udeps --locked
	@command -v cargo-bloat >/dev/null 2>&1 || cargo install cargo-bloat --locked
	@command -v cargo-nextest >/dev/null 2>&1 || cargo install cargo-nextest --locked
	@rustup +nightly component add miri 2>/dev/null || true
	@echo "All tools installed successfully!"

all: fmt clippy test build ## Format, lint, test, and build

all-strict: fmt-check clippy-all security test-nextest build ## Run all checks with strict settings
