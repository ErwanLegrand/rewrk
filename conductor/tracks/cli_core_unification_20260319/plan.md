# Plan: Unify CLI onto `rewrk-core` Benchmarking Engine

## Phase 1: Build CLI Producer and Collector

- [ ] Task: Implement CliProducer wrapping CLI flags into rewrk-core Producer trait
    - [ ] Write tests for CliProducer batch generation from CLI settings
    - [ ] Create `src/cli_producer.rs` implementing `rewrk_core::Producer`
    - [ ] Handle --method, --header, --body, --rounds, --duration flags
    - [ ] Verify tests pass
- [ ] Task: Implement CliCollector wrapping Sample aggregation into SampleCollector
    - [ ] Write tests for CliCollector merging multiple Samples
    - [ ] Create `src/cli_collector.rs` implementing `rewrk_core::SampleCollector`
    - [ ] Aggregate histograms (latency, corrected_latency, read/write transfer) across samples
    - [ ] Track error counts from samples
    - [ ] Verify tests pass
- [ ] Task: Conductor - User Manual Verification 'Build CLI Producer and Collector' (Protocol in workflow.md)

## Phase 2: Migrate bench.rs to rewrk-core Engine

- [ ] Task: Replace http::start_tasks with ReWrkBenchmark in bench.rs
    - [ ] Write integration test: CLI benchmark with CliProducer + CliCollector produces expected output
    - [ ] Rewrite `bench.rs::run()` to use `ReWrkBenchmark::create_with_tls()`
    - [ ] Wire CliProducer and CliCollector
    - [ ] Verify benchmark runs end-to-end
- [ ] Task: Rewrite results display to use Sample histograms
    - [ ] Write tests for latency display from Sample histograms (uncorrected + corrected)
    - [ ] Write tests for JSON output from Sample histograms
    - [ ] Refactor `results.rs` to accept aggregated histograms instead of raw Vec<Duration>
    - [ ] Remove `build_corrected_stats`, `WorkerResult`, and raw duration collection
    - [ ] Verify --pct, --json, and default output formats match previous behavior
- [ ] Task: Conductor - User Manual Verification 'Migrate bench.rs to rewrk-core Engine' (Protocol in workflow.md)

## Phase 3: Cleanup and Hardening

- [ ] Task: Remove src/http/ module
    - [ ] Delete src/http/mod.rs, src/http/usage.rs, src/http/user_input.rs
    - [ ] Move URI parsing from src/http/user_input.rs to src/main.rs or a small utility
    - [ ] Update src/main.rs imports
    - [ ] Verify cargo build succeeds
- [ ] Task: Document ExpectedIntervalTracker CMA approximation
    - [ ] Add module-level doc comment explaining CMA approach and its limitations
    - [ ] Note that under sustained load spikes, the CMA inflates, reducing correction strength
    - [ ] Suggest fixed interval as alternative when target RPS is known
- [ ] Task: Verify test coverage improvement
    - [ ] Run cargo llvm-cov and document final numbers
    - [ ] Verify CLI crate coverage improved from 35% baseline
    - [ ] Add targeted tests if needed for uncovered paths
- [ ] Task: Final clippy and test verification
    - [ ] Run cargo clippy --all -- -D warnings and verify zero warnings
    - [ ] Run cargo test --all and verify zero failures
    - [ ] Verify --json output schema backward compatibility
- [ ] Task: Conductor - User Manual Verification 'Cleanup and Hardening' (Protocol in workflow.md)
