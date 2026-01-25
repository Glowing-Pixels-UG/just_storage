Phase 2 — Reorganize tests by type

Goals:
- Create directory structure: tests/unit, tests/integration, tests/e2e, tests/property, tests/performance
- Split `api_endpoint_tests.rs` into focused e2e files
- Update test imports to use `tests/common`

This PR is a plan + small scaffolding; subsequent PRs will move files incrementally.

