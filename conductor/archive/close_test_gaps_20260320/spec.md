# Spec: Close Test Coverage Gaps

## Overview
Overall coverage is 76.53%, below the 80% target. The CLI binary crate is the main gap.

## Coverage Gaps

### Critical (0% coverage)
- `src/bench.rs` (0%) — orchestration module, 87 lines
- `src/main.rs` (0%) — CLI entry, 211 lines. Contains testable pure functions: parse_duration, parse_header
- `src/runtime.rs` (0%) — 7 lines, trivial

### Significant (<50% coverage)
- `src/results.rs` (46%) — display functions never called in tests

### Below threshold (<80%)
- `rewrk-core/src/runtime/mod.rs` (78%) — create_connector error paths

### Test Quality Issues
- Integration tests use hardcoded ports (port collision risk)
- No TLS/HTTPS path tested
- validator.rs only tests 200/404/500
- SampleFactory untested outside integration

## Acceptance Criteria
- [ ] Overall line coverage >= 80%
- [ ] parse_duration and parse_header have unit tests
- [ ] Integration tests use random ports (no collisions)
- [ ] At least one TLS/HTTPS test exists
- [ ] validator.rs covers 2xx range and 3xx codes
