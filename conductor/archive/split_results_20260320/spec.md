# Spec: Split results.rs and Deduplicate Display Functions

## Overview
`src/results.rs` is 573 lines with two pairs of near-duplicate functions:
- `display_latencies` / `display_latencies_corrected` (lines 45-94)
- `display_percentile_table` / `display_percentile_table_corrected` (lines 146-239)

## Proposed Changes

### 1. Deduplicate display functions
Parameterize with a title string or boolean flag instead of duplicating entire functions.

### 2. Split into sub-modules
- `results/display.rs` (~145 lines) — text-mode terminal display functions
- `results/percentile.rs` (~50 lines after dedup) — percentile table renderer
- `results/json.rs` (~80 lines) — JSON output format
- `results/mod.rs` — re-exports `display_results` as the public API

## Acceptance Criteria
- [ ] No duplicated display functions
- [ ] results.rs split into 3+ sub-modules
- [ ] Output is identical before and after refactor
- [ ] All existing tests pass
- [ ] Tests refactored to call actual display functions (write to impl Write)
