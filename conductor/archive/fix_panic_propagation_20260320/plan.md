# Plan: Fix Panic Propagation on Task Join

## Phase 1: Fix and Test

- [x] Task: Write test for graceful JoinError handling in worker [7ac80df]
    - [x] Simulate a panicking subtask
    - [x] Verify worker shuts down without propagating the panic

- [x] Task: Replace .expect() with match on JoinError in worker.rs:195 [7ac80df]
    - [x] Log error via tracing::error!
    - [x] Return default RuntimeTimings on JoinError

- [x] Task: Replace .expect() with match on JoinError in runtime/mod.rs:169 [7ac80df]
    - [x] Log error via tracing::error!
    - [x] Return Result<C, Error> — new Error::CollectorJoinFailed variant
    - [x] Updated all call sites (bench.rs, integration tests, lib.rs doc example)

- [x] Task: Run full test suite [7ac80df]
    - [x] cargo test --all — 116/116 pass
    - [x] cargo clippy --all -- -D warnings — clean

- [x] Task: Conductor - User Manual Verification 'Fix and Test' (Protocol in workflow.md)
