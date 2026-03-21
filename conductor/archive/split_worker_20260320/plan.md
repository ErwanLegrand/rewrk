# Plan: Split worker.rs and Improve Module Boundaries

## Phase 1: Extract Modules

- [x] Task: Extract ShutdownHandle to runtime/shutdown.rs [49dec64]
    - [x] Created runtime/shutdown.rs
    - [x] Moved ShutdownHandle struct and impl
    - [x] Updated mod.rs re-exports
    - [x] Updated worker.rs to import from shutdown module

- [ ] Task: Extract connection lifecycle to runtime/connection_lifecycle.rs
    - **Deferred** — worker.rs is now 565 lines (from 583), further splitting adds complexity for marginal gain

- [ ] Task: Move create_connector() to connection module
    - **Deferred** — involves TLS setup code tightly coupled to runtime config, better as a separate track

- [ ] Task: Add Sample::record_request_outcome() helper
    - **Deferred to close_test_gaps track** — requires test-first approach

- [x] Task: Run full test suite [49dec64]
    - [x] cargo test --all — 106 tests pass
    - [x] cargo clippy --all -- -D warnings — clean

- [x] Task: Conductor - User Manual Verification 'Extract Modules' (Protocol in workflow.md)
