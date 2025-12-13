# Contributing to JustStorage

Thank you for your interest in contributing to JustStorage! This document provides guidelines and information for contributors.

## Development Setup

### Prerequisites

- Rust 1.75.0 or later
- PostgreSQL 14+ (for integration tests)
- Git

### Quick Start

1. Fork and clone the repository:
   ```bash
   git clone https://github.com/your-username/just-storage.git
   cd just-storage
   ```

2. Install development tools:
   ```bash
   make install-tools
   ```

3. Set up the database:
   ```bash
   createdb just_storage_dev
   psql just_storage_dev < schema.sql
   ```

4. Run tests and checks:
   ```bash
   make lint
   make test
   ```

## Development Workflow

### 1. Create a Branch

Create a feature branch from `main`:
```bash
git checkout -b feature/your-feature-name
# or for bug fixes:
git checkout -b bugfix/issue-description
```

### 2. Make Changes

- Follow the existing code style
- Add tests for new functionality
- Update documentation as needed
- Run quality checks regularly:
  ```bash
  make lint    # Run all linting checks
  make test    # Run tests
  ```

### 3. Commit Changes

Follow conventional commit format:
```bash
git commit -m "feat: add new feature"
git commit -m "fix: resolve issue with X"
git commit -m "docs: update README"
```

### 4. Create Pull Request

- Push your branch to GitHub
- Create a pull request with a clear description
- Ensure all CI checks pass
- Request review from maintainers

## Code Quality Standards

### Linting and Formatting

All code must pass the following checks:

```bash
# Format code
make fmt

# Run lints
make clippy

# Security checks
make security

# Run all quality checks
make lint
```

### Testing

- Unit tests are required for new functionality
- Integration tests for database operations
- All tests must pass before merging

### Documentation

- Update README.md for new features
- Add doc comments for public APIs
- Update this CONTRIBUTING.md as needed

## Pull Request Process

1. **Template**: Use the provided PR template
2. **Labels**: PRs will be automatically labeled based on branch names and content
3. **Reviews**: At least one maintainer review is required
4. **CI**: All CI checks must pass
5. **Merge**: Squash merge is preferred for clean history

## Security Considerations

- Report security issues privately to maintainers
- Do not commit sensitive information
- Run security audits regularly:
  ```bash
  make audit    # Check for vulnerabilities
  make deny     # Check dependencies
  ```

## Performance Guidelines

- Profile performance-critical code
- Use appropriate data structures
- Optimize database queries
- Monitor binary size with `make bloat`

## Dependency Management

- Keep dependencies up-to-date
- Use `cargo deny` to check licenses and security
- Minimize dependency footprint
- Check for unused dependencies: `make udeps`

## Commit Message Guidelines

Follow conventional commits:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Code style changes
- `refactor`: Code refactoring
- `test`: Test changes
- `chore`: Maintenance

## Issue Reporting

- Use GitHub issues for bugs and feature requests
- Provide clear reproduction steps for bugs
- Include environment information
- Check existing issues first

## License

By contributing to JustStorage, you agree that your contributions will be licensed under the MIT License.

## Questions?

Feel free to ask questions in GitHub discussions or reach out to maintainers directly.

Thank you for contributing to JustStorage! 🎉
