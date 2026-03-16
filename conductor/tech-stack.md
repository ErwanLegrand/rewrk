# Technology Stack

## Language
- **Rust** (Edition 2021, nightly toolchain)
- Workspace layout: `rewrk` (CLI binary) + `rewrk-core` (library crate)

## Async Runtime
- **Tokio 1.x** -- multi-threaded runtime with configurable worker threads
- Used for all async I/O, timers, and task spawning

## HTTP
- **hyper 0.14** -- HTTP/1.1 and HTTP/2 client implementation
- No HTTP pipelining (intentional design choice for realistic benchmarking)
- HTTP/2 via `http2_only(true)` on hyper's connection builder

## TLS
- **native-tls 0.2** -- platform-native TLS implementation
- **tokio-native-tls 0.3** -- async TLS stream wrapper
- ALPN negotiation for HTTP version selection (`http/1.1`, `h2`)

## Service Abstraction
- **tower 0.4** -- service trait abstraction layer

## Metrics & Recording
- **hdrhistogram 7.x** -- High Dynamic Range Histograms for latency percentile calculations (microsecond precision)

## Concurrency Primitives
- **flume 0.10.14** -- fast MPMC channels for inter-thread communication (producer/worker coordination)

## CLI
- **clap 2.x** -- command-line argument parsing

## Error Handling
- **anyhow 1.x** -- application-level error handling (CLI binary)
- **thiserror 1.x** -- library error type derivation (rewrk-core)

## Logging
- **tracing 0.1** -- structured logging and diagnostics

## Utilities
- **pin-project-lite 0.2** -- lightweight pin projection
- **async-trait 0.1** -- async trait support
- **regex 1.x** -- duration string parsing

## Testing
- **tokio::test** -- async test runtime
- **axum 0.6.5** -- lightweight HTTP server for integration tests
- **tracing-subscriber 0.3.16** -- log output in tests

## Build & CI
- **GitHub Actions** -- CI pipeline (`rust.yml`)
- Multi-platform: Ubuntu, macOS, Windows
- Runs `cargo test --all`
