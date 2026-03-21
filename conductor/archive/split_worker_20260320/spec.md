# Spec: Split worker.rs and Improve Module Boundaries

## Overview
`rewrk-core/src/runtime/worker.rs` is 575 lines and acts as a coupling hub importing from 5 modules. This track extracts cohesive sub-modules and moves connector construction to the connection module.

## Proposed Changes

### 1. Extract shutdown.rs (~17 lines)
Move `ShutdownHandle` to `rewrk-core/src/runtime/shutdown.rs`. It's a cross-cutting synchronization primitive independent of worker logic.

### 2. Extract connection_lifecycle.rs (~125 lines)
Move `connect_with_timeout()` and `create_worker_connection()` to their own module. These handle connection establishment and retry logic.

### 3. Move create_connector() to connection module (~70 lines)
Add `ReWrkConnector::from_config()` factory method in `connection/conn.rs`. Move TLS setup, address resolution, and protocol selection out of `runtime/mod.rs`.

### 4. Add record_request_outcome() to Sample
Encapsulate the success/error branching that currently lives in `WorkerConnection::send()` into a higher-level method on `Sample`.

## Acceptance Criteria
- [ ] worker.rs reduced to <450 lines
- [ ] runtime/mod.rs reduced by ~70 lines
- [ ] No public API changes
- [ ] All existing tests pass
