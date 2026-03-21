# Spec: Fix CLI Error Handling

## Overview
Multiple error handling issues in the CLI binary crate.

## Issues

### 1. main() exits 0 on error (src/main.rs:30-145)
All error paths use `eprintln!` + `return`. Process exits with code 0 on error.
**Fix:** Return `Result` from `main()` or use `std::process::exit(1)`.

### 2. Duration arithmetic overflow (src/main.rs:160)
`days * 24 * 60 * 60` can overflow u64 with extreme values.
**Fix:** Use `checked_mul` and return error on overflow.

### 3. calculate_rate division by zero (sample.rs:206)
Produces infinity on `Duration::ZERO`.
**Fix:** Guard against zero duration.

### 4. Regex recompilation (src/main.rs:155)
`Regex::new()` called every invocation of `parse_duration`.
**Fix:** Use `std::sync::LazyLock` or `once_cell::Lazy`.

### 5. bench.rs error doesn't set exit code (src/bench.rs:70-74)
**Fix:** Propagate error to main() which should exit non-zero.

## Acceptance Criteria
- [ ] Process exits non-zero on all error paths
- [ ] Duration parsing uses checked arithmetic
- [ ] calculate_rate handles zero duration
- [ ] Regex compiled once
- [ ] All existing tests pass
