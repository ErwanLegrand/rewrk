# Plan: Fix CLI Error Handling

## Phase 1: Fix Error Handling

- [x] Task: Make main() return Result or use process::exit(1) [d8bcbb2]
    - [x] Changed main() to return anyhow::Result<()>
    - [x] All error paths now use ? operator

- [x] Task: Add checked arithmetic to parse_duration [d8bcbb2]
    - [x] Used checked_mul for days/hours/minutes calculations
    - [x] Returns error on overflow

- [x] Task: Guard calculate_rate against zero duration
    - [x] Already fixed in fix_hotpath_expects track [8a6afb5]

- [x] Task: Use LazyLock for regex in parse_duration [d8bcbb2]
    - [x] static DURATION_RE: LazyLock<Regex> compiled once

- [x] Task: Propagate bench.rs errors to main [d8bcbb2]
    - [x] start_benchmark returns anyhow::Result<()>
    - [x] Errors propagated via ? to main

- [x] Task: Run full test suite [d8bcbb2]
    - [x] cargo test --all — all tests pass
    - [x] cargo clippy --all -- -D warnings — clean

- [x] Task: Conductor - User Manual Verification 'Fix Error Handling' (Protocol in workflow.md)
