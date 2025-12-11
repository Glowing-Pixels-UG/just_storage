# JustStorage Documentation Index

Complete guide to JustStorage - a production-ready content-addressable object storage service.

---

## 📚 Documentation Structure

### Getting Started

1. **[README.md](../README.md)** - Project overview, quick start, and feature summary
2. **[QUICKSTART.md](QUICKSTART.md)** - 5-minute setup guide with examples
3. **[API.md](API.md)** - Complete API reference with request/response examples

### Architecture & Design

4. **[ARCHITECTURE.md](ARCHITECTURE.md)** - System architecture overview
5. **[CLEAN_ARCHITECTURE.md](CLEAN_ARCHITECTURE.md)** - Clean Architecture implementation
6. **[DESIGN.md](../DESIGN.md)** - Detailed design decisions and state machine
7. **[DATABASE.md](DATABASE.md)** - Database schema, indexes, and migrations

### Development

8. **[DEVELOPMENT.md](../DEVELOPMENT.md)** - Development setup and workflow
9. **[RUST_BEST_PRACTICES.md](RUST_BEST_PRACTICES.md)** - Rust coding standards
10. **[TESTING.md](TESTING.md)** - Testing strategy and examples
11. **[CONTRIBUTING.md](CONTRIBUTING.md)** - How to contribute

### Operations

12. **[DEPLOYMENT.md](DEPLOYMENT.md)** - Production deployment guide
13. **[OPERATIONS.md](OPERATIONS.md)** - Day-to-day operations manual
14. **[MONITORING.md](MONITORING.md)** - Observability and alerting
15. **[TROUBLESHOOTING.md](TROUBLESHOOTING.md)** - Common issues and solutions

### Reference

16. **[IMPLEMENTATION.md](../IMPLEMENTATION.md)** - Implementation details
17. **[COMPLETION_SUMMARY.md](../COMPLETION_SUMMARY.md)** - Implementation checklist
18. **[LONGHORN_VS_SERVICE.md](LONGHORN_VS_SERVICE.md)** - Responsibility boundaries

---

## 🎯 Quick Navigation

### For New Users

Start here: **README.md** → **QUICKSTART.md** → **API.md**

### For Developers

1. Read **CLEAN_ARCHITECTURE.md** for code organization
2. Review **RUST_BEST_PRACTICES.md** for coding standards
3. Follow **DEVELOPMENT.md** for setup
4. Check **TESTING.md** before writing tests

### For DevOps/SRE

1. Review **ARCHITECTURE.md** for system design
2. Follow **DEPLOYMENT.md** for production setup
3. Use **OPERATIONS.md** for daily tasks
4. Reference **TROUBLESHOOTING.md** when issues arise

### For Architects

- **DESIGN.md** - Core design decisions
- **LONGHORN_VS_SERVICE.md** - Infrastructure boundaries
- **DATABASE.md** - Data model and consistency

---

## 📖 Document Summaries

### README.md

- Project overview and value proposition
- Key features and benefits
- Quick start with Docker Compose
- Example API calls
- Current status and roadmap

### CLEAN_ARCHITECTURE.md

- Layer-by-layer architecture breakdown
- Domain entities and value objects
- Ports and adapters pattern
- Dependency inversion principles
- Testing strategies

### DESIGN.md

- State machine (WRITING → COMMITTED → DELETING → DELETED)
- Content-addressable storage
- Two-phase commit protocol
- Garbage collection design
- Consistency guarantees

### DEVELOPMENT.md

- Local development environment setup
- Database migrations
- Running tests
- Code formatting and linting
- Common development tasks

### RUST_BEST_PRACTICES.md

- Error handling patterns
- Async/await best practices
- Type safety guidelines
- Performance optimization
- Security considerations

---

## 🔍 Search by Topic

### API Operations

- **Upload**: API.md, QUICKSTART.md, DESIGN.md (two-phase commit)
- **Download**: API.md, QUICKSTART.md
- **Delete**: API.md, DESIGN.md (garbage collection)
- **List**: API.md, DATABASE.md (pagination)

### Storage

- **Content Addressing**: DESIGN.md, IMPLEMENTATION.md
- **Deduplication**: DESIGN.md, DATABASE.md (ref counting)
- **Storage Classes**: API.md, DATABASE.md

### Reliability

- **Crash Safety**: DESIGN.md (two-phase commit)
- **Consistency**: DESIGN.md, DATABASE.md
- **Garbage Collection**: DESIGN.md, OPERATIONS.md

### Authentication

- **JWT Tokens**: API.md, OPERATIONS.md
- **API Keys**: API.md, OPERATIONS.md

### Deployment

- **Docker**: DEPLOYMENT.md, README.md
- **Kubernetes**: DEPLOYMENT.md, README.md
- **Configuration**: DEPLOYMENT.md, OPERATIONS.md

---

## 📝 Recent Updates

- **2025-12-11**: Documentation reorganization complete
  - Archived outdated summaries (PRODUCTION_READY, ARCHITECTURE_SUMMARY, PROJECT_SUMMARY)
  - Created comprehensive new docs (API, QUICKSTART, OPERATIONS, ARCHITECTURE)
  - Added centralized documentation index
- **2025-12-11**: Completed core implementation (v0.1.0)
- **2025-12-11**: Added database validation CLI tool
- **2025-12-11**: Improved error handling across all layers

## 📁 Documentation Organization

### Root Directory (`/`)

- **README.md** - Project overview, quick links, status
- **DESIGN.md** - Core design decisions and state machine
- **DEVELOPMENT.md** - Development setup and workflow
- **IMPLEMENTATION.md** - Implementation details and code examples
- **COMPLETION_SUMMARY.md** - Implementation checklist and status
- **DOCUMENTATION_SUMMARY.md** - This documentation overhaul summary

### Documentation Directory (`/docs`)

#### Core Documentation

- **INDEX.md** - This file, central documentation hub
- **API.md** - Complete API reference
- **QUICKSTART.md** - 5-minute getting started guide

#### Architecture & Design

- **ARCHITECTURE.md** - System architecture with diagrams
- **CLEAN_ARCHITECTURE.md** - Layer-by-layer implementation guide
- **LONGHORN_VS_SERVICE.md** - Responsibility boundaries

#### Development & Operations

- **RUST_BEST_PRACTICES.md** - Rust coding standards
- **OPERATIONS.md** - Operations manual

#### Archive (`/docs/archive`)

- Historical documentation superseded by current docs
- Preserved for reference and design evolution context

---

## 🤝 Contributing to Documentation

See [CONTRIBUTING.md](CONTRIBUTING.md) for:

- Documentation standards
- How to add new documents
- Review process
- Style guide

---

## 📧 Need Help?

- **Bug reports**: Open an issue with reproduction steps
- **Feature requests**: Open an issue with use case description
- **Questions**: Check TROUBLESHOOTING.md first, then open a discussion
