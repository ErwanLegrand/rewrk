# Spec: Unify CLI onto `rewrk-core` Benchmarking Engine

## Overview

The `rewrk` CLI binary currently has its own HTTP client implementation (`src/http/`) that is entirely separate from the `rewrk-core` library's benchmarking engine (`ReWrkBenchmark`, workers, `Sample`). This means:

1. **Dual CO-correction:** The CLI computes CO-corrected stats by rebuilding an HDR histogram from raw `Vec<Duration>` at the end (`build_corrected_stats` in `results.rs`), while `rewrk-core` records per-request into `corrected_latency_hist` using the per-connection running average. These produce different numbers.
2. **Dead code:** `results.rs` contains display functions, percentile calculations, and a `WorkerResult` type that duplicate functionality available through `rewrk-core`'s `Sample` type.
3. **Maintenance burden:** Bug fixes and improvements must be applied in two places.

This track migrates the CLI to use `rewrk-core`'s `ReWrkBenchmark` API as its benchmarking engine, eliminating the separate `src/http/` module and unifying metrics onto a single path.

## Background

Found during code review of the CO Mitigation & HTTP/3 Readiness Audit track. The `rewrk-core` library was designed to be the canonical benchmarking engine, but the CLI predates it and was never migrated.

## Requirements

### Functional Requirements

- **FR-1:** Replace `src/http/` with a CLI-specific `Producer` implementation that generates requests based on CLI flags (method, headers, body, host).
- **FR-2:** Replace `src/results.rs` `WorkerResult` with a CLI-specific `SampleCollector` that aggregates `Sample` objects from `rewrk-core` and formats output.
- **FR-3:** CO-corrected statistics must come from `rewrk-core`'s `corrected_latency_hist` (per-request `record_correct`), not a post-hoc rebuild.
- **FR-4:** Remove `build_corrected_stats` and the raw `Vec<Duration>` collection path from `results.rs`.
- **FR-5:** Preserve all existing CLI flags and output formats (backward compatibility): `--host`, `--connections`, `--threads`, `--duration`, `--http2`, `--pct`, `--json`, `--no-co-correction`, `--insecure`, `--rounds`, `--method`, `--header`, `--body`.
- **FR-6:** `--json` output must include both uncorrected latency stats and `latency_corrected` object, sourced from `Sample` histograms.
- **FR-7:** `--no-co-correction` flag must suppress CO-corrected output (skip `corrected_latency()` histogram in display).
- **FR-8:** Document the `ExpectedIntervalTracker`'s CMA-based approach as an approximation in module-level docs, noting it may underestimate correction under sustained load spikes.

### Non-Functional Requirements

- **NFR-1:** CLI output format must be identical for a given benchmark run (same columns, units, alignment).
- **NFR-2:** `--json` schema must remain backward-compatible.
- **NFR-3:** The `src/http/` module should be fully removed (no dead code left behind).
- **NFR-4:** Test coverage for the CLI crate should increase (previously 35% line coverage).

## Out of Scope

- Adding a target-RPS mode (open-loop benchmarking).
- Changing the `rewrk-core` public API.
- HTTP/3 implementation.

## Success Criteria

1. `src/http/` directory no longer exists.
2. `build_corrected_stats` function no longer exists in `results.rs`.
3. CLI output matches the current format for `--pct`, `--json`, and default modes.
4. `cargo test --all` passes.
5. `cargo clippy --all` produces zero warnings.
6. Running `rewrk --host http://... --pct` shows CO-corrected percentiles sourced from `rewrk-core` Sample histograms.
