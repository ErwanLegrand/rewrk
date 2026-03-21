# Spec: Fix IO Usage Tracking Bugs

## Overview
`rewrk-core/src/utils/io_usage.rs` has two bugs in the `RecordStream` implementation that cause inaccurate byte transfer metrics.

## Bug 1: poll_read over-counts (line 62-64)
`buf.filled().len()` counts ALL filled bytes in the buffer, not just newly-read bytes. If the buffer already had data, this over-counts.

**Fix:** Capture `buf.filled().len()` before and after the poll, add only the difference.

## Bug 2: poll_write ignores partial writes (line 78)
`buf.len()` is added unconditionally before checking if the write succeeded. On `Poll::Pending` or partial writes, the count is wrong.

**Fix:** Only count bytes on `Poll::Ready(Ok(n))`, and count `n` (actual bytes written), not `buf.len()`.

## Acceptance Criteria
- [ ] poll_read counts only newly-read bytes
- [ ] poll_write counts only successfully-written bytes
- [ ] Existing io_usage tests pass
- [ ] New tests cover partial read/write scenarios
