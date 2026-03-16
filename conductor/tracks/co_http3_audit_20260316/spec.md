# Spec: CO Mitigation & HTTP/3 Readiness Audit

## Overview

This track audits and improves rewrk's benchmarking accuracy by implementing Coordinated Omission (CO) detection and correction, refactors the connection layer into a protocol-agnostic abstraction for HTTP/3 readiness, and hardens the codebase through quality, security, and test coverage improvements.

## Background

### Coordinated Omission Problem

Most HTTP benchmarking tools (wrk, hey, ab) suffer from Coordinated Omission: when the server slows down, the client also slows down its request rate, causing latency measurements to exclude the time requests *would have been waiting* if they had been sent on schedule. This makes tail latencies (p99, p99.9) appear much lower than reality.

rewrk currently measures latency per-request from send to response, but does not account for requests that were delayed because the previous request on that connection hadn't completed yet. The HDR Histogram library already supports CO correction via `record_correct()` -- rewrk needs to use it.

### HTTP/3 Readiness

hyper's HTTP/3 support (via h3 crate + quinn QUIC transport) is under active development. rewrk currently hardcodes hyper's HTTP/1 and HTTP/2 connection handling directly in `ReWrkConnector`. To adopt HTTP/3 when available, the connection layer must be abstracted behind a protocol-agnostic trait.

## Requirements

### Functional Requirements

#### FR-1: Coordinated Omission Detection & Correction
- **FR-1.1:** Track the expected inter-request interval per connection (based on observed average service time).
- **FR-1.2:** Use `hdrhistogram::Histogram::record_correct(value, expected_interval)` to produce CO-corrected latency histograms.
- **FR-1.3:** Report both uncorrected and CO-corrected latency statistics in CLI output.
- **FR-1.4:** In `--json` mode, include both `latency` and `latency_corrected` objects.
- **FR-1.5:** Add a `--co-correction` flag (enabled by default) that can be disabled with `--no-co-correction`.

#### FR-2: Protocol-Agnostic Connection Layer
- **FR-2.1:** Define a `ProtocolConnector` trait abstracting connection establishment, request execution, and IO tracking.
- **FR-2.2:** Implement `Http1Connector` and `Http2Connector` as concrete implementations.
- **FR-2.3:** Refactor `ReWrkConnector` to dispatch to the appropriate connector based on protocol configuration.
- **FR-2.4:** Ensure the trait is designed so a future `Http3Connector` (using quinn + h3) can be added without changing the benchmarking engine.
- **FR-2.5:** Document the trait interface with examples for implementing a custom connector.

#### FR-3: Quality & Security Hardening
- **FR-3.1:** Achieve >80% test coverage across both `rewrk` and `rewrk-core` crates.
- **FR-3.2:** Add unit tests for latency recording, CO correction logic, and result formatting.
- **FR-3.3:** Add integration tests for the new protocol abstraction layer.
- **FR-3.4:** Run and fix all `cargo clippy` warnings.
- **FR-3.5:** Audit `unsafe` code blocks (if any) and TLS certificate handling.
- **FR-3.6:** Validate all CLI inputs (duration format, connection count, thread count bounds).

### Non-Functional Requirements

- **NFR-1:** CO correction must not add measurable overhead to the hot path (< 1% throughput impact).
- **NFR-2:** The protocol abstraction must not add latency to request execution (zero-cost abstraction where possible).
- **NFR-3:** All changes must maintain backward compatibility with existing CLI flags and `rewrk-core` public API.
- **NFR-4:** CI must pass on all three platforms (Linux, macOS, Windows).

## Out of Scope
- Implementing HTTP/3 transport (only the abstraction layer).
- Rate-limited benchmarking mode (fixed request rate).
- Distributed load generation.

## Success Criteria
1. Running `rewrk --pct` against a server with artificial latency spikes shows meaningfully different CO-corrected vs uncorrected p99/p99.9 values.
2. A new `Http3Connector` stub can be added that compiles and integrates without modifying the worker or recording code.
3. `cargo test --all` passes with >80% coverage.
4. `cargo clippy --all -- -D warnings` produces zero warnings.
