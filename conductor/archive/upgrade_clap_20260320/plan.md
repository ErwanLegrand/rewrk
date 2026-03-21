# Plan: Upgrade clap 2.x to 4.x

## Phase 1: Migration

- [x] Task: Update Cargo.toml to clap 4.x with derive feature [d8bcbb2]
    - [x] Remove clap 2.x dependency
    - [x] Add clap 4.x with derive feature
    - [x] Updated edition from 2018 to 2021

- [x] Task: Rewrite src/main.rs CLI parsing with derive macros [d8bcbb2]
    - [x] Define Args struct with #[derive(Parser)]
    - [x] Map all existing arguments to struct fields
    - [x] Preserve parse_duration and parse_header logic
    - [x] Ensure exit code 1 on invalid args (clap 4.x does this by default)

- [x] Task: Update src/bench.rs for new arg types [d8bcbb2]
    - [x] start_benchmark now returns anyhow::Result<()>

- [x] Task: Verify cargo audit clean [d8bcbb2]
    - [x] cargo audit shows 0 warnings, 0 vulnerabilities

- [x] Task: Run full test suite [d8bcbb2]
    - [x] cargo test --all — all tests pass
    - [x] cargo clippy --all -- -D warnings — clean

- [x] Task: Conductor - User Manual Verification 'Migration' (Protocol in workflow.md)
