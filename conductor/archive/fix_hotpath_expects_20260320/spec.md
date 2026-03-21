# Spec: Fix Hot-path Expects and Add Body Size Limit

## Overview
Several `.expect()` calls in the benchmarking hot path can panic under edge conditions, and response bodies are read without size limits.

## Issues

### 1. Histogram recording expects (sample.rs:156,174,187,200)
`.expect("Record value")` panics if histogram cannot record the value. Should handle gracefully.
**Fix:** Use `if let Err(e)` and log a warning, or use saturating record.

### 2. cli_producer.rs:56 protocol violation panic
`.expect("ready() must be called before create_batch()")` panics on misuse.
**Fix:** Return `Result` instead of panicking.

### 3. Unbounded response body reads (http1.rs:131, http2.rs:132)
`hyper::body::to_bytes(body)` reads entire response with no size limit.
**Fix:** Add a configurable body size limit (e.g., 10MB default) using `http_body::Limited`.

## Acceptance Criteria
- [ ] No .expect() in recording hot path
- [ ] cli_producer returns Result on protocol violation
- [ ] Response body reads have a configurable size limit
- [ ] All existing tests pass
