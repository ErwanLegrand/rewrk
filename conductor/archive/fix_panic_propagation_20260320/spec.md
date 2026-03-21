# Spec: Fix Panic Propagation on Task Join

## Overview
`worker.rs:195` and `runtime/mod.rs:169` use `.expect()` on tokio task join results. If a subtask panics, the `.expect()` propagates the panic to the parent thread, crashing the entire worker instead of shutting down gracefully.

## Locations
1. `rewrk-core/src/runtime/worker.rs:195` — `.expect("Join tasks")` on worker task joins
2. `rewrk-core/src/runtime/mod.rs:169` — `.expect("Join task")` on collector join

## Fix
Replace `.expect()` with proper `JoinError` handling:
- Log the error with `tracing::error!`
- Set the shutdown/abort flag
- Continue graceful shutdown

## Acceptance Criteria
- [ ] No `.expect()` on JoinHandle results outside tests
- [ ] JoinError is logged and triggers graceful shutdown
- [ ] Existing tests pass
- [ ] New test verifies graceful handling of panicked subtask
