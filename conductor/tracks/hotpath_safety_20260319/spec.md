# Spec: Hot-Path Safety — Eliminate `unwrap()` in `execute_req`

## Overview

The `execute_req` method in `Http1Connection` and `Http2Connection` calls `.scheme().unwrap()`, `.authority().unwrap()`, and `.build().unwrap()` on every request. These are the innermost hot-path methods — called once per benchmarked HTTP request. A malformed URI would cause a panic deep inside a worker thread with no error recovery.

This track moves URI validation to connector construction time and stores the extracted components as typed fields, making the hot path infallible.

## Background

Found during code review of the CO Mitigation & HTTP/3 Readiness Audit track. The pattern was inherited from the original `ReWrkConnector` implementation and propagated to `Http1Connection` and `Http2Connection` during the protocol abstraction refactoring.

## Requirements

### Functional Requirements

- **FR-1:** `Http1Connector::new()` and `Http2Connector::new()` must validate that the provided `Uri` has both a scheme and authority. Return an error (or panic at construction) if either is missing.
- **FR-2:** Store the extracted `http::uri::Scheme` and `http::uri::Authority` as typed fields in `Http1Connection` and `Http2Connection`, eliminating the need to unwrap `Option` values during request execution.
- **FR-3:** `execute_req` must not contain any `.unwrap()` calls on URI components. The URI builder should use the pre-validated typed fields directly.
- **FR-4:** The `ReWrkConnector` enum dispatch and `create_connector` in `runtime/mod.rs` must continue to work without API changes to callers.

### Non-Functional Requirements

- **NFR-1:** Zero performance regression — the hot path should be faster (no Option unwrapping) or equivalent.
- **NFR-2:** All existing tests must continue to pass without modification.

## Out of Scope

- Changing the `ProtocolConnector` trait signature.
- Validating request-level URIs (only the base URI stored in the connector/connection).

## Success Criteria

1. `grep -r "\.unwrap()" rewrk-core/src/connection/http1.rs rewrk-core/src/connection/http2.rs` returns zero matches in `execute_req` methods.
2. `cargo test --all` passes.
3. `cargo clippy --all` produces zero warnings.
