# Plan: CO Mitigation & HTTP/3 Readiness Audit

## Phase 1: Codebase Quality Baseline

- [x] Task: Run cargo clippy and fix all warnings [d8c3e86]
    - [x] Run `cargo clippy --all -- -D warnings` and catalog issues
    - [x] Fix all clippy warnings across both crates
    - [x] Verify clean clippy run
- [x] Task: Establish test coverage baseline (44.27% -- rewrk-core ~65%, CLI 0%)
    - [x] Install cargo-tarpaulin or cargo-llvm-cov
    - [x] Run coverage report and document current percentage
    - [x] Identify uncovered modules and critical paths
- [x] Task: Audit unsafe code and TLS handling [3b8dc29]
    - [x] Search for `unsafe` blocks and review each for soundness (none found)
    - [x] Review TLS certificate validation (accept_invalid_certs usage) -- fixed: gated behind --insecure flag
    - [x] Review input validation for CLI arguments (duration, connections, threads) -- fixed: reject 0 values
    - [x] Document findings and fix critical issues
- [x] Task: Conductor - User Manual Verification 'Codebase Quality Baseline' (Protocol in workflow.md)

## Phase 2: Coordinated Omission Mitigation

- [ ] Task: Implement expected interval tracking per connection
    - [ ] Write tests for expected interval calculation from observed service times
    - [ ] Add expected interval tracking to the worker's per-connection request loop in `rewrk-core/src/runtime/worker.rs`
    - [ ] Verify tests pass
- [ ] Task: Integrate CO-corrected histogram recording
    - [ ] Write tests verifying `record_correct()` produces different percentiles than `record()` under simulated latency spikes
    - [ ] Modify `Sample` to carry both uncorrected and CO-corrected histograms
    - [ ] Use `hdrhistogram::Histogram::record_correct(value, expected_interval)` in the recording path
    - [ ] Verify tests pass
- [ ] Task: Expose CO-corrected statistics in CLI output
    - [ ] Write tests for result formatting with both corrected and uncorrected values
    - [ ] Update `src/results.rs` to display CO-corrected latency alongside uncorrected
    - [ ] Update `--json` output to include `latency_corrected` object
    - [ ] Add `--no-co-correction` CLI flag (CO correction on by default)
    - [ ] Verify tests pass
- [ ] Task: Validate CO correction with synthetic latency spike test
    - [ ] Write an integration test with an axum server that introduces artificial latency spikes
    - [ ] Assert that CO-corrected p99 is significantly higher than uncorrected p99
    - [ ] Verify the test demonstrates the CO effect clearly
- [ ] Task: Conductor - User Manual Verification 'Coordinated Omission Mitigation' (Protocol in workflow.md)

## Phase 3: Protocol-Agnostic Connection Layer

- [ ] Task: Define ProtocolConnector trait
    - [ ] Write tests for trait interface contract (connect, execute, IO tracking)
    - [ ] Design `ProtocolConnector` trait in a new module `rewrk-core/src/connection/protocol.rs`
    - [ ] Define associated types for connection state, request execution, and IO measurement
    - [ ] Verify tests pass
- [ ] Task: Implement Http1Connector
    - [ ] Write tests for HTTP/1.1 connection establishment and request execution
    - [ ] Extract HTTP/1 logic from `ReWrkConnector` into `Http1Connector` implementing `ProtocolConnector`
    - [ ] Verify tests pass and existing integration tests still pass
- [ ] Task: Implement Http2Connector
    - [ ] Write tests for HTTP/2 connection establishment and request execution
    - [ ] Extract HTTP/2 logic from `ReWrkConnector` into `Http2Connector` implementing `ProtocolConnector`
    - [ ] Verify tests pass and existing integration tests still pass
- [ ] Task: Refactor ReWrkConnector to dispatch via ProtocolConnector
    - [ ] Write tests for protocol dispatch (HTTP/1 vs HTTP/2 selection)
    - [ ] Refactor `ReWrkConnector` to hold a `Box<dyn ProtocolConnector>` (or enum dispatch)
    - [ ] Remove duplicated protocol-specific code from the original connector
    - [ ] Verify all existing tests still pass (no behavioral change)
- [ ] Task: Add Http3Connector stub for compile-time validation
    - [ ] Create a stub `Http3Connector` that implements `ProtocolConnector` with `unimplemented!()` bodies
    - [ ] Write a compile-only test that instantiates it to prove the trait is sufficient
    - [ ] Document the trait interface with examples for future HTTP/3 implementation
- [ ] Task: Conductor - User Manual Verification 'Protocol-Agnostic Connection Layer' (Protocol in workflow.md)

## Phase 4: Test Coverage & Final Hardening

- [ ] Task: Add unit tests for latency recording and sample aggregation
    - [ ] Write tests for `Sample` creation, merging, and histogram operations
    - [ ] Write tests for IO usage tracking accuracy
    - [ ] Write tests for error classification and counting
- [ ] Task: Add unit tests for CLI result formatting
    - [ ] Write tests for human-readable output formatting
    - [ ] Write tests for JSON output schema correctness
    - [ ] Write tests for percentile table generation
- [ ] Task: Add integration tests for multi-worker benchmarks
    - [ ] Write test for multi-threaded benchmark with >1 worker
    - [ ] Write test for multi-connection concurrency per worker
    - [ ] Write test for benchmark with custom producer and collector
- [ ] Task: Verify >80% test coverage and fix gaps
    - [ ] Run coverage report
    - [ ] Identify remaining uncovered paths
    - [ ] Add targeted tests to reach >80% threshold
    - [ ] Document final coverage numbers
- [ ] Task: Conductor - User Manual Verification 'Test Coverage & Final Hardening' (Protocol in workflow.md)
