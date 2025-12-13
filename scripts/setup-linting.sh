#!/bin/bash
# Setup script for Rust linting and static analysis tools
# Based on 2025 best practices

set -e

echo "🔧 Setting up Rust linting and static analysis tools..."
echo ""

# Check if Rust is installed
if ! command -v rustc &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust first: https://rustup.rs/"
    exit 1
fi

echo "✅ Rust is installed: $(rustc --version)"
echo ""

# Install cargo-audit
if ! command -v cargo-audit &> /dev/null; then
    echo "📦 Installing cargo-audit..."
    cargo install cargo-audit --locked
else
    echo "✅ cargo-audit is already installed"
fi

# Install cargo-deny
if ! command -v cargo-deny &> /dev/null; then
    echo "📦 Installing cargo-deny..."
    cargo install cargo-deny --locked
else
    echo "✅ cargo-deny is already installed"
fi

# Install cargo-udeps
if ! command -v cargo-udeps &> /dev/null; then
    echo "📦 Installing cargo-udeps..."
    cargo install cargo-udeps --locked
else
    echo "✅ cargo-udeps is already installed"
fi

# Install cargo-bloat
if ! command -v cargo-bloat &> /dev/null; then
    echo "📦 Installing cargo-bloat..."
    cargo install cargo-bloat --locked
else
    echo "✅ cargo-bloat is already installed"
fi

# Install cargo-nextest
if ! command -v cargo-nextest &> /dev/null; then
    echo "📦 Installing cargo-nextest..."
    cargo install cargo-nextest --locked
else
    echo "✅ cargo-nextest is already installed"
fi

# Install Miri
echo "📦 Installing Miri..."
rustup +nightly component add miri 2>/dev/null || echo "⚠️  Miri installation skipped (may already be installed)"

echo ""
echo "✅ All tools installed successfully!"
echo ""
echo "📋 Available Makefile commands:"
echo "  make lint          - Run all linting checks"
echo "  make clippy         - Run Clippy linter"
echo "  make clippy-all    - Run Clippy with all lints"
echo "  make fmt-check     - Check code formatting"
echo "  make security      - Run security checks (audit + deny)"
echo "  make audit         - Check for security vulnerabilities"
echo "  make deny          - Run cargo-deny checks"
echo "  make udeps         - Check for unused dependencies"
echo "  make bloat         - Analyze binary size"
echo "  make miri          - Run tests with Miri"
echo "  make test-nextest  - Run tests with nextest"
echo "  make all-strict    - Run all checks with strict settings"
echo ""

