# Plan: Split results.rs and Deduplicate Display Functions

## Phase 1: Deduplicate and Split

- [x] Task: Deduplicate display_latencies / display_latencies_corrected [c4a1f63]
    - [x] Parameterized with title string → display_latencies_impl(hist, title)
    - [x] Removed duplicate function

- [x] Task: Deduplicate display_percentile_table / display_percentile_table_corrected [c4a1f63]
    - [x] Parameterized with column title → display_percentile_table_impl(hist, column_title)
    - [x] Used loop over percentile slice instead of repeated code
    - [x] Removed duplicate function
    - [x] Net result: -88 lines of duplicate code

- [ ] Task: Refactor display functions to accept impl Write
    - **Deferred to close_test_gaps track** — better addressed alongside test improvements

- [ ] Task: Split into sub-modules
    - **Deferred** — file is now under 400 lines after deduplication, splitting not warranted

- [ ] Task: Update tests to call actual display functions
    - **Deferred to close_test_gaps track**

- [x] Task: Run full test suite [c4a1f63]
    - [x] cargo test --all — all tests pass
    - [x] cargo clippy --all -- -D warnings — clean

- [x] Task: Conductor - User Manual Verification 'Deduplicate and Split' (Protocol in workflow.md)
