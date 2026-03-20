# Plan: Worker Reconnection on Connection Close

## Phase 1: Implement Reconnection Logic

- [x] Task: Refactor WorkerConnection to reconnect on ConnectionAborted [333d0be]
    - [x] Write test: worker continues after connection close (mock connector that fails every N requests)
    - [x] Modify `WorkerConnection::send()` to return a reconnection signal instead of `Ok(false)` on ConnectionAborted
    - [x] Add reconnection logic in the worker task loop (re-call `connect_with_timeout` via the connector)
    - [x] Carry over the existing Sample state across reconnections (no data loss)
    - [x] Verify existing keep-alive tests still pass unchanged
- [x] Task: Add integration test with connection-closing server [ddc8f0d]
    - [x] Write test with an axum server that sends `Connection: close` header on every response
    - [x] Assert that the benchmark completes with >0 latency recordings despite connection closes
    - [x] Assert that error counts reflect the connection close events
- [x] Task: Conductor - User Manual Verification 'Implement Reconnection Logic' (Protocol in workflow.md)
