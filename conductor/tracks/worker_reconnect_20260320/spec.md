# Spec: Worker Reconnection on Connection Close

## Overview

When a server closes the connection (e.g., HTTP/1.0 `Connection: close`, server restart, idle timeout), rewrk-core workers currently call `set_abort()` which shuts down the entire benchmark. This is a regression from the old CLI which reconnected automatically and continued benchmarking.

## Background

The old CLI (`src/http/mod.rs`) had a `try_connect_until` method that retried connections in a loop until the benchmark deadline elapsed. When a connection error occurred during a request, the old CLI would attempt to reconnect and continue. The `error_map` tracked connection errors as non-fatal counts.

After migrating the CLI to use `rewrk-core::ReWrkBenchmark`, servers that close connections (like `python3 -m http.server` which uses HTTP/1.0) produce zero successful requests because `set_abort()` terminates all workers on the first `ConnectionAborted` error.

## Requirements

### Functional Requirements

- **FR-1:** When a worker connection receives a `ConnectionAborted` error (connection closed by server), it must attempt to reconnect using the connector rather than aborting.
- **FR-2:** Reconnection attempts must respect the original `retry_max` and `CONNECT_TIMEOUT` limits.
- **FR-3:** If reconnection fails (all retries exhausted or timeout), only that connection should stop — not all workers.
- **FR-4:** Connection errors (both during requests and during reconnection) must be counted in the Sample's error records, not silently dropped.
- **FR-5:** The benchmark must continue producing correct latency measurements after a reconnection (no stale state carried over).

### Non-Functional Requirements

- **NFR-1:** Reconnection must not add latency to the measurement path — the reconnection time itself must not be recorded as request latency.
- **NFR-2:** All existing tests must continue to pass.
- **NFR-3:** The fix must work for both HTTP/1.0 (Connection: close) and HTTP/1.1 (keep-alive) servers.

## Out of Scope

- Changing the `ProtocolConnector` trait.
- Adding exponential backoff to reconnection (simple retry is sufficient).

## Success Criteria

1. `cargo run -- --host http://127.0.0.1:8080 -c 1 -t 1 -d 2s --pct` against `python3 -m http.server 8080` shows latency stats with >0 requests.
2. `cargo test --all` passes.
3. Existing benchmarks against keep-alive servers continue to work identically.
