# Plan: Worker Reconnection on Connection Close

## Phase 1: Implement Reconnection Logic

- [ ] Task: Refactor WorkerConnection to reconnect on ConnectionAborted
    - [ ] Write test: worker continues after connection close (mock connector that fails every N requests)
    - [ ] Modify `WorkerConnection::send()` to return a reconnection signal instead of `Ok(false)` on ConnectionAborted
    - [ ] Add reconnection logic in the worker task loop (re-call `connect_with_timeout` via the connector)
    - [ ] Carry over the existing Sample state across reconnections (no data loss)
    - [ ] Verify existing keep-alive tests still pass unchanged
- [ ] Task: Add integration test with connection-closing server
    - [ ] Write test with an axum server that sends `Connection: close` header on every response
    - [ ] Assert that the benchmark completes with >0 latency recordings despite connection closes
    - [ ] Assert that error counts reflect the connection close events
- [ ] Task: Conductor - User Manual Verification 'Implement Reconnection Logic' (Protocol in workflow.md)
