# Plan: Close Test Coverage Gaps

## Phase 1: CLI Binary Tests

- [x] Task: Extract parse_duration and parse_header to testable module [ba73006]
    - [x] Moved to src/parsing.rs as pub(crate)
    - [x] 8 tests for parse_duration (valid, invalid, edge cases, overflow)
    - [x] 3 tests for parse_header (valid, missing colon, empty)

- [x] Task: Add unit tests for bench.rs pure functions [ba73006]
    - [x] 4 tests for humanize() with various durations
    - [x] 2 tests for string() formatting

- [x] Task: Improve results.rs test coverage [ba73006]
    - [x] Added 5 smoke tests for display_results (json/non-json, empty/data, percentile)
    - [x] results.rs coverage: 46% → 96%

- [x] Task: Conductor - User Manual Verification 'CLI Binary Tests' (Protocol in workflow.md)

## Phase 2: Library Tests

- [x] Task: Fix integration test port collisions [f287af0]
    - [x] Updated basic_benchmark.rs to use port 0 + spawn_server()
    - [x] Updated timed_benchmark.rs to use port 0 + spawn_server()
    - [x] Tests pass when run in parallel

- [ ] Task: Add TLS/HTTPS integration test
    - **Deferred** — requires self-signed cert infrastructure, complex setup

- [x] Task: Expand validator.rs tests [f287af0]
    - [x] test_default_validator_accepts_201
    - [x] test_default_validator_accepts_204
    - [x] test_default_validator_rejects_301
    - [x] test_default_validator_rejects_302

- [x] Task: Add Scheme::Https default_port test [f287af0]
    - [x] test_scheme_default_port_https — verifies 443

- [ ] Task: Add SampleFactory unit tests
    - **Deferred** — requires complex collector channel setup

- [ ] Task: Add create_connector error path tests
    - **Deferred** — requires TLS/network mocking

- [x] Task: Verify overall coverage >= 80% [f287af0]
    - [x] **Overall: 89.65% line coverage** (was 76.53%)
    - [x] Per-file highlights:
        - results.rs: 46% → 96%
        - parsing.rs (new): 97%
        - validator.rs: 94% → 94%
        - connection/mod.rs: 96% → 100%
        - Still below 80%: main.rs (0%, expected — binary entry), bench.rs (39%, orchestration), runtime/mod.rs (76%, TLS setup)

- [x] Task: Conductor - User Manual Verification 'Library Tests' (Protocol in workflow.md)
