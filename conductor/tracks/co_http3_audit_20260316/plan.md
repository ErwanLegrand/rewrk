# Plan: CO Mitigation & HTTP/3 Readiness Audit

## Phase 1: Codebase Quality Baseline [checkpoint: 22ded6c]

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

## Phase 2: Coordinated Omission Mitigation [checkpoint: d59bef0]

- [x] Task: Implement expected interval tracking per connection [ceed6fc]
    - [x] Write tests for expected interval calculation from observed service times
    - [x] Add expected interval tracking to the worker's per-connection request loop in `rewrk-core/src/runtime/worker.rs`
    - [x] Verify tests pass
- [x] Task: Integrate CO-corrected histogram recording [86e9208]
    - [x] Write tests verifying `record_correct()` produces different percentiles than `record()` under simulated latency spikes
    - [x] Modify `Sample` to carry both uncorrected and CO-corrected histograms
    - [x] Use `hdrhistogram::Histogram::record_correct(value, expected_interval)` in the recording path
    - [x] Verify tests pass
- [x] Task: Expose CO-corrected statistics in CLI output [0898433]
    - [x] Write tests for result formatting with both corrected and uncorrected values
    - [x] Update `src/results.rs` to display CO-corrected latency alongside uncorrected
    - [x] Update `--json` output to include `latency_corrected` object
    - [x] Add `--no-co-correction` CLI flag (CO correction on by default)
    - [x] Verify tests pass
- [x] Task: Validate CO correction with synthetic latency spike test [a3c7279]
    - [x] Write an integration test with an axum server that introduces artificial latency spikes
    - [x] Assert that CO-corrected p99 is significantly higher than uncorrected p99
    - [x] Verify the test demonstrates the CO effect clearly
- [x] Task: Conductor - User Manual Verification 'Coordinated Omission Mitigation' (Protocol in workflow.md)

## Phase 3: Protocol-Agnostic Connection Layer [checkpoint: 3cedfc7]

- [x] Task: Define ProtocolConnector trait [d99f3d8]
    - [x] Write tests for trait interface contract (connect, execute, IO tracking)
    - [x] Design `ProtocolConnector` trait in a new module `rewrk-core/src/connection/protocol.rs`
    - [x] Define associated types for connection state, request execution, and IO measurement
    - [x] Verify tests pass
- [x] Task: Implement Http1Connector [cbd9069]
    - [x] Write tests for HTTP/1.1 connection establishment and request execution
    - [x] Extract HTTP/1 logic from `ReWrkConnector` into `Http1Connector` implementing `ProtocolConnector`
    - [x] Verify tests pass and existing integration tests still pass
- [x] Task: Implement Http2Connector [d5ddd55]
    - [x] Write tests for HTTP/2 connection establishment and request execution
    - [x] Extract HTTP/2 logic from `ReWrkConnector` into `Http2Connector` implementing `ProtocolConnector`
    - [x] Verify tests pass and existing integration tests still pass
- [x] Task: Refactor ReWrkConnector to dispatch via ProtocolConnector [bc6c2d0]
    - [x] Write tests for protocol dispatch (HTTP/1 vs HTTP/2 selection)
    - [x] Refactor `ReWrkConnector` to hold a `Box<dyn ProtocolConnector>` (or enum dispatch)
    - [x] Remove duplicated protocol-specific code from the original connector
    - [x] Verify all existing tests still pass (no behavioral change)
- [x] Task: Add Http3Connector stub for compile-time validation [ba99b89]
    - [x] Create a stub `Http3Connector` that implements `ProtocolConnector` with `unimplemented!()` bodies
    - [x] Write a compile-only test that instantiates it to prove the trait is sufficient
    - [x] Document the trait interface with examples for future HTTP/3 implementation
- [x] Task: Conductor - User Manual Verification 'Protocol-Agnostic Connection Layer' (Protocol in workflow.md)

## Phase 4: Test Coverage & Final Hardening

- [x] Task: Add unit tests for latency recording and sample aggregation [ebb1886]
    - [x] Write tests for `Sample` creation, merging, and histogram operations
    - [x] Write tests for IO usage tracking accuracy
    - [x] Write tests for error classification and counting
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
