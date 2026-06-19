# Test-improvement roadmap (salvaged)

These three documents were the only thing of value on the deleted
`phase-5-add-coverage` / `phase-6-modern-testing` branches (their code was stale
and would have regressed OIDC config + validation). They are kept here as the
**spec** for the test-hardening sprint that implements them *fresh* on the
current `main` — not by merging the old branches.

Status (tracked in the test-hardening sprint):
- Phase 4 — tooling: nextest config, coverage gate, insta/rstest dev-deps.
- Phase 5 — coverage: middleware unit tests, repository contract tests, error-propagation tests.
- Phase 6 — modern: insta OpenAPI snapshots, cargo-mutants, tracing-test.

Note: builders + custom assertions (a Phase 4 goal) already exist in
`rust/tests/common/{builders,assertions}.rs`.
