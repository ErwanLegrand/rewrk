# Plan: Comprehensive Code Review — rewrk

## Phase 1: Automated Analysis

- [x] Task: Run `cargo clippy --all -- -D warnings` and document all warnings/errors
    - [x] Capture output for both `rewrk` and `rewrk-core`
    - [x] Categorize findings by severity
    - **Result: CLEAN — 0 warnings, 0 errors across all crates**

- [x] Task: Run `cargo audit` (dependency vulnerability scan) and document findings
    - [x] Install `cargo-audit` if not present
    - [x] Document any known CVEs or advisories
    - **Result: 0 CVEs, 3 warnings (all clap 2.34.0 transitive deps: ansi_term unmaintained, atty unmaintained + potential unsound read). 2 duplicate deps (bitflags, socket2).**

- [x] Task: Run `cargo llvm-cov` to generate coverage report and identify untested code paths
    - [x] Generate per-file coverage percentages
    - [x] Flag files/functions below 80% coverage
    - **Result: 76.53% overall line coverage (below 80% target). Below-80% files: src/bench.rs (0%), src/main.rs (0%), src/runtime.rs (0%), src/results.rs (46.31%), rewrk-core/src/runtime/mod.rs (77.94%)**

- [x] Task: Conductor - User Manual Verification 'Automated Analysis' (Protocol in workflow.md)

## Phase 2: Code Quality & Error Handling Review

- [x] Task: Review `rewrk-core/src/runtime/worker.rs` (575 lines)
    - [x] Flag functions >50 lines, assess complexity
    - [x] Check unwrap/expect usage outside tests
    - [x] Evaluate naming, readability, dead code
    - [x] Document findings with line numbers and severity

- [x] Task: Review `rewrk-core/src/recording/sample.rs` (429 lines)
    - [x] Flag functions >50 lines
    - [x] Check error handling patterns
    - [x] Assess API surface clarity

- [x] Task: Review `rewrk-core/src/connection/conn.rs` (376 lines)
    - [x] Flag functions >50 lines
    - [x] Check error handling in handshake/connection logic

- [x] Task: Review `rewrk-core/src/connection/http1.rs` (344 lines) and `http2.rs` (346 lines)
    - [x] Identify code duplication between HTTP/1 and HTTP/2 implementations
    - [x] Check error handling consistency
    - [x] Document shared logic extraction opportunities

- [x] Task: Review `rewrk-core/src/connection/http3.rs` (289 lines)
    - [x] Verify stub quality (unimplemented!() usage, trait compliance)

- [x] Task: Review `rewrk-core/src/connection/protocol.rs` (277 lines)
    - [x] Evaluate trait design quality
    - [x] Check documentation completeness

- [x] Task: Review `rewrk-core/src/runtime/mod.rs` (284 lines)
    - [x] Flag functions >50 lines
    - [x] Check error handling in DNS resolution, TLS config

- [x] Task: Review remaining `rewrk-core` files (expected_interval.rs, io_usage.rs, timings.rs, collector.rs, producer.rs, validator.rs, lib.rs)
    - [x] Check each for quality, naming, error handling
    - [x] Verify pub(crate) conventions

- [x] Task: Review `src/results.rs` (573 lines)
    - [x] Flag functions >50 lines
    - [x] Check formatting logic complexity
    - [x] Evaluate readability of display code

- [x] Task: Review `src/main.rs` (329 lines)
    - [x] Check CLI argument validation
    - [x] Evaluate parse_duration and parse_header quality
    - [x] Check error messages for user-friendliness

- [x] Task: Review remaining CLI files (bench.rs, cli_producer.rs, cli_collector.rs, utils.rs, runtime.rs)
    - [x] Check each for quality, naming, error handling

- [x] Task: Conductor - User Manual Verification 'Code Quality & Error Handling Review' (Protocol in workflow.md)

## Phase 3: Security Review

- [x] Task: Audit TLS configuration
    - [x] Review certificate validation behavior (especially `--insecure` flag)
    - [x] Check ALPN negotiation correctness
    - [x] Verify no unsafe TLS defaults

- [x] Task: Audit input validation at system boundaries
    - [x] CLI argument parsing edge cases
    - [x] HTTP response handling (malformed responses, oversized headers)
    - [x] URL/host parsing robustness

- [x] Task: Search for unsafe code blocks and evaluate necessity
    - [x] Grep for `unsafe` across codebase
    - [x] Document justification or flag for removal

- [x] Task: Review error messages for information leakage
    - [x] Check that internal details (paths, stack traces) aren't exposed to users

- [x] Task: Conductor - User Manual Verification 'Security Review' (Protocol in workflow.md)

## Phase 4: Modularity Review

- [x] Task: Analyze files >400 lines for splitting opportunities
    - [x] `worker.rs` (575 lines) — identify extractable sub-modules
    - [x] `results.rs` (573 lines) — identify extractable sub-modules
    - [x] `sample.rs` (429 lines) — assess whether splitting is warranted

- [x] Task: Analyze functions >50 lines for extraction opportunities
    - [x] Identify and document each long function with proposed extraction

- [x] Task: Analyze HTTP/1 vs HTTP/2 code duplication
    - [x] Measure duplication percentage
    - [x] Propose shared abstraction or macro approach

- [x] Task: Assess module coupling
    - [x] Map cross-module dependencies
    - [x] Identify circular or excessive coupling
    - [x] Propose decoupling opportunities

- [x] Task: Conductor - User Manual Verification 'Modularity Review' (Protocol in workflow.md)

## Phase 5: Test Coverage Review

- [x] Task: Review unit test quality across all `#[cfg(test)]` modules
    - [x] Assess assertion quality and edge case coverage
    - [x] Check test naming conventions
    - [x] Identify missing negative/boundary tests

- [x] Task: Review integration tests in `rewrk-core/tests/`
    - [x] Assess scenario coverage
    - [x] Identify missing integration scenarios (e.g., error paths, timeouts)

- [x] Task: Identify untested public API surface
    - [x] Cross-reference public API with existing tests
    - [x] Document each untested function/method

- [x] Task: Conductor - User Manual Verification 'Test Coverage Review' (Protocol in workflow.md)

## Phase 6: Findings Report & Follow-up Track Creation

- [x] Task: Compile findings report
    - [x] Organize by dimension (Quality, Security, Modularity, Error Handling, Coverage)
    - [x] Assign severity to each finding (Critical / High / Medium / Low)
    - [x] Include file paths and line numbers
    - **Result: findings_report.md written — 0 Critical, 5 HIGH, 11 MEDIUM, 8 LOW findings**

- [x] Task: Create follow-up conductor tracks for all findings
    - [x] One track per finding or logical group of related findings
    - [x] Each track includes spec referencing the finding details
    - **Result: 10 follow-up tracks created (2 bug, 2 refactor, 1 test, 2 chore, 1 upgrade, 2 fix)**

- [x] Task: Conductor - User Manual Verification 'Findings Report & Follow-up Track Creation' (Protocol in workflow.md)
