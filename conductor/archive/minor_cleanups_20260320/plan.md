# Plan: Minor Cleanups

## Phase 1: Quick Fixes

- [x] Task: Fix typos in error messages [1680f0f]
    - [x] validator.rs:22 "serve." → "server."
    - [x] validator.rs:25 "to long" → "too long"
    - [x] runtime/mod.rs:65 "overriden" → "overridden"

- [x] Task: Rename HttpProtocol variants [1680f0f]
    - [x] HTTP1 → Http1, HTTP2 → Http2
    - [x] Updated all 14 files with references

- [x] Task: Fix pub(crate) visibility [1680f0f]
    - [x] conn.rs HttpStream::send → pub(crate)
    - [x] worker.rs ShutdownHandle → pub(crate)
    - [x] worker.rs WorkerConnection → pub(crate)

- [x] Task: Remove commented-out code [1680f0f]
    - [x] main.rs:317-327 removed dead random arg

- [x] Task: Run full test suite [1680f0f]
    - [x] cargo test --all — all tests pass
    - [x] cargo clippy --all -- -D warnings — clean

- [x] Task: Conductor - User Manual Verification 'Quick Fixes' (Protocol in workflow.md)
