# Spec: Upgrade clap 2.x to 4.x

## Overview
clap 2.34.0 pulls in 3 problematic transitive dependencies:
- RUSTSEC-2021-0139: ansi_term 0.12.1 unmaintained
- RUSTSEC-2024-0375: atty 0.2.14 unmaintained
- RUSTSEC-2021-0145: atty 0.2.14 potential unsound unaligned read

Upgrading to clap 4.x eliminates all three advisories and enables derive-based argument parsing.

## Scope
- `src/main.rs` — rewrite CLI argument parsing from builder to derive API
- `Cargo.toml` — update clap dependency
- `src/bench.rs` — update BenchmarkSettings construction from new arg types

## Acceptance Criteria
- [ ] clap 4.x in Cargo.toml
- [ ] cargo audit shows 0 warnings
- [ ] All CLI arguments work identically (same names, same defaults)
- [ ] Duration parsing preserved
- [ ] All existing tests pass
