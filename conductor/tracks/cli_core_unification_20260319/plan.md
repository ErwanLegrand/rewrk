# Plan: Unify CLI onto `rewrk-core` Benchmarking Engine

## Phase 1: Build CLI Producer and Collector [checkpoint: d17ee25]

- [x] Task: Implement CliProducer wrapping CLI flags into rewrk-core Producer trait [aff185d]
    - [x] Write tests for CliProducer batch generation from CLI settings
    - [x] Create `src/cli_producer.rs` implementing `rewrk_core::Producer`
    - [x] Handle --method, --header, --body, --rounds, --duration flags
    - [x] Verify tests pass
- [x] Task: Implement CliCollector wrapping Sample aggregation into SampleCollector [6cd6f06]
    - [x] Write tests for CliCollector merging multiple Samples
    - [x] Create `src/cli_collector.rs` implementing `rewrk_core::SampleCollector`
    - [x] Aggregate histograms (latency, corrected_latency, read/write transfer) across samples
    - [x] Track error counts from samples
    - [x] Verify tests pass
- [x] Task: Conductor - User Manual Verification 'Build CLI Producer and Collector' (Protocol in workflow.md)

## Phase 2: Migrate bench.rs to rewrk-core Engine [checkpoint: ddc8f0d]

- [x] Task: Replace http::start_tasks with ReWrkBenchmark in bench.rs [4d8b164]
    - [x] Write integration test: CLI benchmark with CliProducer + CliCollector produces expected output
    - [x] Rewrite `bench.rs::run()` to use `ReWrkBenchmark::create_with_tls()`
    - [x] Wire CliProducer and CliCollector
    - [x] Verify benchmark runs end-to-end
- [x] Task: Rewrite results display to use Sample histograms [4d8b164]
    - [x] Write tests for latency display from Sample histograms (uncorrected + corrected)
    - [x] Write tests for JSON output from Sample histograms
    - [x] Refactor `results.rs` to accept aggregated histograms instead of raw Vec<Duration>
    - [x] Remove `build_corrected_stats`, `WorkerResult`, and raw duration collection
    - [x] Verify --pct, --json, and default output formats match previous behavior
- [x] Task: Conductor - User Manual Verification 'Migrate bench.rs to rewrk-core Engine' (Protocol in workflow.md)

## Phase 3: Cleanup and Hardening [checkpoint: 3c1e97d]

- [x] Task: Remove src/http/ module [fdd8af7]
    - [x] Delete src/http/mod.rs, src/http/usage.rs, src/http/user_input.rs
    - [x] Move URI parsing from src/http/user_input.rs to src/main.rs or a small utility
    - [x] Update src/main.rs imports
    - [x] Verify cargo build succeeds
- [x] Task: Document ExpectedIntervalTracker CMA approximation [fdd8af7]
    - [x] Add module-level doc comment explaining CMA approach and its limitations
    - [x] Note that under sustained load spikes, the CMA inflates, reducing correction strength
    - [x] Suggest fixed interval as alternative when target RPS is known
- [x] Task: Verify test coverage improvement
    - [x] Run cargo llvm-cov and document final numbers (rewrk-core: 92.8%, cli_collector: 97%, cli_producer: 98%)
    - [x] Verify CLI crate coverage improved from 35% baseline
    - [x] Add targeted tests if needed for uncovered paths
- [x] Task: Final clippy and test verification
    - [x] Run cargo clippy --all -- -D warnings and verify zero warnings
    - [x] Run cargo test --all and verify zero failures (117 tests)
    - [x] Verify --json output schema backward compatibility
- [x] Task: Conductor - User Manual Verification 'Cleanup and Hardening' (Protocol in workflow.md)
