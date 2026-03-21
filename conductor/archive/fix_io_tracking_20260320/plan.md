# Plan: Fix IO Usage Tracking Bugs

## Phase 1: Fix and Test

- [x] Task: Write failing tests for poll_read over-counting [2e9a7f4]
    - [x] Test that pre-filled buffer bytes are not double-counted
    - [x] Test that only newly-read bytes are counted

- [x] Task: Fix poll_read in io_usage.rs [2e9a7f4]
    - [x] Capture buf.filled().len() before poll
    - [x] Add only (after - before) to counter

- [x] Task: Write failing tests for poll_write partial write [2e9a7f4]
    - [x] Test that only actually-written bytes are counted
    - [x] Test that Pending polls don't increment counter

- [x] Task: Fix poll_write in io_usage.rs [2e9a7f4]
    - [x] Only count on Poll::Ready(Ok(n))
    - [x] Use n (actual written), not buf.len()

- [x] Task: Run full test suite and verify coverage [2e9a7f4]
    - [x] cargo test --all — 7/7 io_usage tests pass
    - [x] All 116 tests pass

- [x] Task: Conductor - User Manual Verification 'Fix and Test' (Protocol in workflow.md)
