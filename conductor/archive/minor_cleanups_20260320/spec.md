# Spec: Minor Cleanups

## Overview
Low-severity findings from the code review that are quick to fix.

## Issues

### 1. Typos in error messages
- `rewrk-core/src/validator.rs:22` — "serve." → "server."
- `rewrk-core/src/validator.rs:25` — "to long" → "too long"
- `rewrk-core/src/runtime/mod.rs:65` — "overriden" → "overridden"

### 2. HttpProtocol naming convention
- `rewrk-core/src/connection/mod.rs:19-22` — `HTTP1`/`HTTP2` should be `Http1`/`Http2` per Rust convention

### 3. pub(crate) visibility
- `rewrk-core/src/connection/conn.rs:125` — `HttpStream::send` should be `pub(crate)`
- `rewrk-core/src/runtime/worker.rs:348` — `ShutdownHandle` should be `pub(crate)`
- `rewrk-core/src/runtime/worker.rs:365` — `WorkerConnection` should be `pub(crate)`

### 4. Commented-out code
- `src/main.rs:317-327` — Commented-out `random` CLI arg, should be removed

## Acceptance Criteria
- [ ] All typos fixed
- [ ] HttpProtocol variants renamed to Http1/Http2
- [ ] Internal types marked pub(crate)
- [ ] Commented-out code removed
- [ ] All existing tests pass
- [ ] cargo clippy clean
