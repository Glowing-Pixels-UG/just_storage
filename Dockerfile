# syntax=docker/dockerfile:1
#
# Multi-stage build for JustStorage.
#   base    — pinned toolchain + full crate source (shared by ci/builder)
#   ci      — format check, clippy (-D warnings), unit + doc tests
#   builder — release binary
#   runtime — minimal Debian image with just the binary
#
# Toolchain is pinned to 1.96 (the project MSRV; sqlx 0.9 requires >= 1.94).
# BuildKit cache mounts keep recompiles fast across builds on the (possibly
# remote) daemon. Build context is trimmed by .dockerignore.

# Pin the builder to the bookworm variant so the produced binary links against
# the same glibc (2.36) as the debian:bookworm-slim runtime. The default
# rust:1.96 image tracks Debian trixie (glibc 2.38+), which is NOT present in
# bookworm and makes the binary fail at runtime with "GLIBC_2.38 not found".
FROM docker.io/library/rust:1.96-bookworm AS base
WORKDIR /app
ENV SQLX_OFFLINE=true
ENV CARGO_TERM_COLOR=never
# clippy/rustfmt are needed by the ci stage (idempotent if already present)
RUN rustup component add clippy rustfmt
# Manifests first for layer caching, then the full crate (src, tests, benches,
# tools, migrations, templates, static assets, .sqlx, lint configs).
COPY rust/Cargo.toml rust/Cargo.lock ./
COPY rust/ ./

# ---- CI: the quality gate (mirrors .github/workflows/ci.yml) ----
# Integration/e2e tests use testcontainers (need a Docker daemon) and run in
# GitHub CI, not here.
FROM base AS ci
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    cargo fmt --all -- --check
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=target-debug,target=/app/target \
    cargo clippy --all-targets --all-features -- -D warnings
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=target-debug,target=/app/target \
    cargo test --lib --bins
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=target-debug,target=/app/target \
    cargo test --doc

# ---- Release builder ----
# Distinct target cache id from the ci (debug) stage: mixing debug and release
# artifacts in one cache dir causes churn and can serve a stale binary.
FROM base AS builder
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=target-release,target=/app/target \
    cargo build --release --bin just_storage && \
    cp target/release/just_storage /app/just_storage

# ---- Runtime ----
FROM docker.io/library/debian:bookworm-slim AS runtime
RUN apt-get update && \
  apt-get install -y ca-certificates && \
  rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/just_storage /app/just_storage
COPY --from=base /app/internal_static /app/internal_static
RUN mkdir -p /data/hot /data/cold
EXPOSE 8080
CMD ["/app/just_storage"]
