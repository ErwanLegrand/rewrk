# Plan: Fix Hot-path Expects and Add Body Size Limit

## Phase 1: Fix Panics

- [x] Task: Replace histogram .expect() with graceful handling in sample.rs [8a6afb5]
    - [x] Used if let Err(e) pattern with tracing::warn!
    - [x] All 4 recording methods now log instead of panic
    - [x] Added test for calculate_rate edge values

- [x] Task: Replace cli_producer.rs .expect() with Result [8a6afb5]
    - [x] Changed to .ok_or_else(|| anyhow!(...))?
    - [x] Protocol violation now returns error instead of panic

- [ ] Task: Add response body size limit
    - **Deferred to dedup_http_connectors track** — requires http-body dependency and trait interface changes, better addressed during connector refactor

- [x] Task: Run full test suite [8a6afb5]
    - [x] cargo test --all — all tests pass
    - [x] cargo clippy --all -- -D warnings — clean

- [x] Task: Conductor - User Manual Verification 'Fix Panics' (Protocol in workflow.md)
