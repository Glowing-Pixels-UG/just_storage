# Rust Linting and Static Analysis Setup

This document describes the comprehensive linting and static analysis setup for the Rust projects in this repository, based on 2025 best practices.

## Overview

The project uses a comprehensive set of tools to ensure code quality, security, and performance:

- **Clippy**: Official Rust linter with 750+ lints
- **rustfmt**: Code formatter for consistent style
- **cargo-audit**: Security vulnerability scanner
- **cargo-deny**: Comprehensive dependency checker
- **cargo-udeps**: Unused dependency detector
- **cargo-bloat**: Binary size analyzer
- **cargo-nextest**: Fast parallel test runner
- **Miri**: Undefined behavior detector for unsafe code

## Quick Start

### Install Tools

Run the setup script to install all required tools:

```bash
./scripts/setup-linting.sh
```

Or use the Makefile:

```bash
make install-tools
```

### Run All Checks

```bash
make lint          # Run all linting checks
make all-strict    # Run all checks with strict settings
```

## Tool Configuration

### Clippy

Clippy is configured via command-line flags in the Makefile. The configuration enforces:

- All warnings treated as errors (`-D warnings`)
- Pedantic and nursery lints available via `make clippy-all`

**Usage:**
```bash
make clippy         # Basic clippy checks
make clippy-all     # All clippy lints (pedantic + nursery)
```

### rustfmt

Code formatting is configured via `rustfmt.toml` files in each project root.

**Key settings:**
- Max line width: 100 characters
- Tab spaces: 4
- Format strings and doc comments
- Group and reorder imports

**Usage:**
```bash
make fmt            # Format code
make fmt-check      # Check formatting without modifying files
```

### cargo-audit

Scans dependencies for known security vulnerabilities from the RustSec advisory database.

**Usage:**
```bash
make audit
```

### cargo-deny

Comprehensive dependency checker that validates:
- Security advisories
- License compliance
- Dependency sources
- Banned crates
- Multiple versions

**Configuration:** `deny.toml` in each project root

**Usage:**
```bash
make deny
```

**License Policy:**
- Allowed: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, 0BSD, CC0-1.0, Unlicense
- Denied: GPL, AGPL, LGPL variants
- Copyleft licenses: Warning

### cargo-udeps

Detects unused dependencies to keep `Cargo.toml` clean.

**Usage:**
```bash
make udeps
```

**Note:** Requires nightly Rust toolchain.

### cargo-bloat

Analyzes binary size to identify optimization opportunities.

**Usage:**
```bash
make bloat
```

### cargo-nextest

Fast, parallel test runner that provides better output and performance than standard `cargo test`.

**Usage:**
```bash
make test-nextest
```

### Miri

Interprets Rust code to detect undefined behavior, especially useful for unsafe code.

**Usage:**
```bash
make miri
```

**Note:** Requires nightly Rust toolchain.

## Makefile Commands

| Command | Description |
|---------|-------------|
| `make lint` | Run all linting and static analysis checks |
| `make clippy` | Run Clippy linter (basic) |
| `make clippy-all` | Run Clippy with all lints enabled |
| `make fmt` | Format code |
| `make fmt-check` | Check code formatting |
| `make security` | Run all security checks (audit + deny) |
| `make audit` | Check for security vulnerabilities |
| `make deny` | Run cargo-deny checks |
| `make udeps` | Check for unused dependencies |
| `make bloat` | Analyze binary size |
| `make miri` | Run tests with Miri |
| `make test-nextest` | Run tests with nextest |
| `make install-tools` | Install all required tools |
| `make all-strict` | Run all checks with strict settings |

## CI/CD Integration

These tools should be integrated into your CI/CD pipeline. Example GitHub Actions workflow:

```yaml
name: Linting and Security

on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Install tools
        run: make install-tools
      - name: Format check
        run: make fmt-check
      - name: Clippy
        run: make clippy
      - name: Security audit
        run: make security
      - name: Tests
        run: make test-nextest
```

## Best Practices

1. **Run linting before commits**: Use `make lint` or `make all-strict` before pushing code
2. **Fix warnings immediately**: Don't accumulate warnings - fix them as they appear
3. **Update dependencies regularly**: Run `make audit` and `make deny` regularly
4. **Use nextest for faster feedback**: Prefer `make test-nextest` over `make test` for faster test runs
5. **Check binary size**: Use `make bloat` periodically to monitor binary size
6. **Test unsafe code with Miri**: Always run `make miri` when working with unsafe code

## Troubleshooting

### Tool Installation Issues

If a tool fails to install, try:
1. Update Rust: `rustup update`
2. Update cargo: `cargo update`
3. Clear cargo cache: `cargo clean`

### Clippy Warnings

If Clippy reports warnings you want to allow:
1. Add `#[allow(clippy::lint_name)]` attribute to the specific code
2. Or configure in `clippy.toml` if it's a project-wide decision

### License Issues

If cargo-deny reports license issues:
1. Review the license in `deny.toml`
2. Add exceptions for specific crates if needed
3. Consider alternatives if a crate has an incompatible license

## References

- [Clippy Documentation](https://github.com/rust-lang/rust-clippy)
- [rustfmt Documentation](https://github.com/rust-lang/rustfmt)
- [cargo-audit Documentation](https://github.com/rustsec/rustsec/tree/main/cargo-audit)
- [cargo-deny Documentation](https://github.com/EmbarkStudios/cargo-deny)
- [cargo-udeps Documentation](https://github.com/est31/cargo-udeps)
- [cargo-bloat Documentation](https://github.com/RazrFalcon/cargo-bloat)
- [cargo-nextest Documentation](https://nexte.st/)
- [Miri Documentation](https://github.com/rust-lang/miri)

