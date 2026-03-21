# Spec: Comprehensive Code Review — rewrk

## Overview

A thorough, full-codebase review of the `rewrk` workspace (both `rewrk` CLI binary and `rewrk-core` library crate). The review covers code quality, security, modularity, error handling, and test coverage. Findings are documented in a structured report, and each finding produces a follow-up conductor track for remediation.

## Scope

All Rust source files in:
- `rewrk-core/src/` (~3,900 lines across 18 files) — benchmarking engine, connection layer, recording, runtime, utilities
- `src/` (~1,500 lines across 7 files) — CLI entry point, results display, producer, collector
- `rewrk-core/tests/` (~960 lines across 6 files) — integration tests (review for quality/coverage gaps only)

## Review Dimensions

### 1. Code Quality
- Naming consistency and readability
- Function complexity (flag functions >50 lines)
- Dead code, unused imports, redundant logic
- Clippy lint compliance (`-D warnings`)
- Adherence to project Rust conventions (pub(crate), thiserror/anyhow split, workspace patterns)

### 2. Security
- Unsafe code blocks (if any)
- TLS configuration correctness (certificate validation, ALPN)
- Input validation at system boundaries (CLI args, HTTP responses)
- Error messages that could leak sensitive information
- Dependency audit (known vulnerabilities in Cargo.lock)

### 3. Modularity
- Files exceeding 400 lines — candidates for splitting
- Functions exceeding 50 lines — candidates for extraction
- Module coupling — identify tight coupling between components
- Opportunities to extract shared abstractions (e.g., HTTP/1 and HTTP/2 code duplication)

### 4. Error Handling
- `unwrap()` / `expect()` usage outside test code
- Error propagation patterns (proper use of `?`, `thiserror`, `anyhow`)
- User-facing error message quality from the CLI
- Silent error swallowing

### 5. Test Coverage
- Identify untested public API surface
- Missing edge cases in existing tests
- Test quality (assertions, isolation, naming)
- Integration test coverage gaps

## Deliverables

1. **Findings report** — structured by dimension, with severity ratings (Critical / High / Medium / Low)
2. **Follow-up tracks** — one conductor track per finding (or per logical group of related findings), created via `/conductor` new track workflow

## Acceptance Criteria

- [ ] Every source file in both crates has been reviewed
- [ ] Findings are documented with file path, line numbers, and severity
- [ ] `cargo clippy --all -- -D warnings` passes (or violations are documented)
- [ ] `cargo audit` run and results documented (or noted if not installed)
- [ ] Each finding has a corresponding follow-up conductor track
- [ ] No dimension is skipped — all five areas are covered for every file
