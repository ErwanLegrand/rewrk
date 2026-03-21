# Code Review Findings Report — rewrk

**Date:** 2026-03-20
**Scope:** Full codebase — `rewrk` CLI binary + `rewrk-core` library crate
**Overall Assessment:** No critical issues. 5 HIGH, 11 MEDIUM, 8 LOW findings.

---

## 1. Code Quality

### HIGH

| # | Finding | File:Line | Description |
|---|---------|-----------|-------------|
| Q1 | IO read tracking bug | `rewrk-core/src/utils/io_usage.rs:62-64` | `poll_read` counts `buf.filled().len()` (total filled bytes) instead of newly-read bytes. Over-counts receive metrics when buffer already has data. |
| Q2 | IO write tracking bug | `rewrk-core/src/utils/io_usage.rs:78` | `poll_write` counts `buf.len()` unconditionally, but actual bytes written is in `Poll::Ready(Ok(n))`. Wrong count on partial writes or `Pending`. |
| Q3 | Panic propagation from JoinError | `rewrk-core/src/runtime/worker.rs:195`, `rewrk-core/src/runtime/mod.rs:169` | `.expect("Join tasks")` on task join results. A panicked subtask crashes the parent thread instead of graceful shutdown. |

### MEDIUM

| # | Finding | File:Line | Description |
|---|---------|-----------|-------------|
| Q4 | Near-duplicate display functions | `src/results.rs:45-94`, `src/results.rs:146-239` | Two pairs of copy-paste functions (`display_latencies` / `display_latencies_corrected`, `display_percentile_table` / `display_percentile_table_corrected`) differing only by title string. |
| Q5 | `HttpProtocol` SCREAMING_CASE variants | `rewrk-core/src/connection/mod.rs:19-22` | `HTTP1`/`HTTP2` instead of Rust convention `Http1`/`Http2`. |
| Q6 | `main()` exits 0 on error | `src/main.rs:30-145` | All error paths use `eprintln!` + `return` instead of `std::process::exit(1)`. |
| Q7 | Hot-path histogram `.expect()` | `rewrk-core/src/recording/sample.rs:156,174,187,200` | Panics if histogram cannot record value, in the benchmarking hot path. |
| Q8 | `calculate_rate` division by zero | `rewrk-core/src/recording/sample.rs:206` | Produces infinity/undefined on `Duration::ZERO`. |

### LOW

| # | Finding | File:Line | Description |
|---|---------|-----------|-------------|
| Q9 | Typos in error messages | `rewrk-core/src/validator.rs:22,25`, `rewrk-core/src/runtime/mod.rs:65` | "serve" → "server", "to long" → "too long", "overriden" → "overridden" |
| Q10 | `pub` vs `pub(crate)` inconsistencies | `rewrk-core/src/runtime/worker.rs:348,365`, `rewrk-core/src/connection/conn.rs:125` | Internal types marked `pub` instead of `pub(crate)`. |
| Q11 | Regex recompilation | `src/main.rs:155` | `Regex::new()` called on every `parse_duration` invocation instead of using `LazyLock`. |

---

## 2. Security

### MEDIUM

| # | Finding | File:Line | Description |
|---|---------|-----------|-------------|
| S1 | Duration arithmetic overflow | `src/main.rs:160` | `days * 24 * 60 * 60` can overflow u64. Should use `checked_mul`. |
| S2 | Unbounded response body reads | `rewrk-core/src/connection/http1.rs:131`, `http2.rs:132` | `hyper::body::to_bytes(body)` reads entire response with no size limit. Malicious target could cause OOM. |

### LOW

| # | Finding | File:Line | Description |
|---|---------|-----------|-------------|
| S3 | No minimum TLS version enforced | `rewrk-core/src/runtime/mod.rs:222` | Defaults to TLS 1.0+. Acceptable for benchmarking tool. |
| S4 | Zero `unsafe` code | N/A | Positive finding — no unsafe blocks in entire codebase. |
| S5 | TLS defaults secure | `rewrk-core/src/runtime/mod.rs:222-235` | `--insecure` documented with WARNING. ALPN correct. |
| S6 | No information leakage | N/A | Error messages appropriate for CLI tool. No secrets logged. |

---

## 3. Modularity

### MEDIUM

| # | Finding | File:Line | Description |
|---|---------|-----------|-------------|
| M1 | HTTP/1 vs HTTP/2 ~85% duplication | `rewrk-core/src/connection/http1.rs`, `http2.rs` | 344 vs 346 lines, structurally identical. Only difference: `http2_only(true)` on builder. Propose generic `BaseConnector<F>` with config closure. |
| M2 | `worker.rs` should be split | `rewrk-core/src/runtime/worker.rs` (575 lines) | Extract `shutdown.rs` (~17 lines) + `connection_lifecycle.rs` (~125 lines). |
| M3 | `results.rs` should be split | `src/results.rs` (573 lines) | Extract `display.rs` + `percentile.rs` + `json.rs` sub-modules. |

### LOW

| # | Finding | Description |
|---|---------|-------------|
| M4 | `sample.rs` splitting NOT warranted | 429 lines but ~208 production code. Tightly coupled types. |
| M5 | No circular dependencies | Clean dependency graph. |
| M6 | `worker.rs` coupling hub | Inherent to its role as benchmarking engine core. Propose `record_request_outcome()` on Sample to reduce surface. |
| M7 | Connector construction in wrong module | `runtime/mod.rs:create_connector()` should be a factory in `connection` module. |

---

## 4. Error Handling

### HIGH

| # | Finding | File:Line | Description |
|---|---------|-----------|-------------|
| E1 | (Same as Q3) Panic propagation from JoinError | `worker.rs:195`, `runtime/mod.rs:169` | See Q3. |

### MEDIUM

| # | Finding | File:Line | Description |
|---|---------|-----------|-------------|
| E2 | (Same as Q7) Hot-path histogram `.expect()` | `sample.rs:156,174,187,200` | See Q7. |
| E3 | `cli_producer.rs:56` `.expect()` on protocol violation | `src/cli_producer.rs:56` | Could return `Err` instead of panicking. |
| E4 | `bench.rs:70-74` error doesn't set exit code | `src/bench.rs:70-74` | Benchmark failure prints to stderr but exits 0. |

---

## 5. Test Coverage

### HIGH

| # | Finding | File:Line | Description |
|---|---------|-----------|-------------|
| T1 | Integration test port collision | `rewrk-core/tests/basic_benchmark.rs:16`, `timed_benchmark.rs:18` | Both use port 19999. Will fail when run in parallel. |

### MEDIUM

| # | Finding | File:Line | Description |
|---|---------|-----------|-------------|
| T2 | `results.rs` tests don't call actual display functions | `src/results.rs:322-573` | Tests reconstruct JSON manually instead of calling `display_json()`. |
| T3 | `parse_duration`/`parse_header` untested | `src/main.rs:152-200` | Pure functions with 0% coverage. Should be extracted and tested. |
| T4 | No TLS/HTTPS integration tests | N/A | `Scheme::Https` path entirely untested. |
| T5 | Overall coverage below 80% | N/A | 76.53% line coverage. CLI binary crate is the main gap (bench.rs 0%, main.rs 0%, results.rs 46%). |

### LOW

| # | Finding | Description |
|---|---------|-------------|
| T6 | `validator.rs` only tests 200/404/500 | Missing 2xx range (201, 204), 3xx codes. |
| T7 | `Scheme::Https` default_port untested | Only Http variant tested. |
| T8 | `SampleFactory` untested outside integration | No unit tests for factory methods. |

---

## Follow-up Tracks Created

| Track | Findings | Priority |
|-------|----------|----------|
| `fix_io_tracking_20260320` | Q1, Q2 | HIGH |
| `fix_panic_propagation_20260320` | Q3/E1 | HIGH |
| `upgrade_clap_20260320` | S1, clap supply chain | MEDIUM |
| `dedup_http_connectors_20260320` | M1 | MEDIUM |
| `split_worker_20260320` | M2, M6, M7 | MEDIUM |
| `split_results_20260320` | M3, Q4 | MEDIUM |
| `fix_cli_error_handling_20260320` | Q6, E4, S1, Q8, Q11 | MEDIUM |
| `fix_hotpath_expects_20260320` | Q7/E2, S2, E3 | MEDIUM |
| `close_test_gaps_20260320` | T1-T8 | MEDIUM |
| `minor_cleanups_20260320` | Q5, Q9, Q10 | LOW |
