FROM rust:1.92 as builder

WORKDIR /app

# Copy manifests
COPY rust/Cargo.toml rust/Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && \
  echo "fn main() {}" > src/main.rs && \
  cargo build --release && \
  rm -rf src

# Copy source code
COPY rust/src ./src

# Build application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && \
  apt-get install -y ca-certificates && \
  rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/just_storage /app/just_storage

# Create data directories
RUN mkdir -p /data/hot /data/cold

EXPOSE 8080

CMD ["/app/just_storage"]
