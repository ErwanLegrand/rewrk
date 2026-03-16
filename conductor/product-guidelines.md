# Product Guidelines

## Tone & Style

### CLI Output
- **Technical and precise.** Numbers speak for themselves.
- Use exact terminology: "CO-corrected p99 latency", not "adjusted latency".
- No decorative output, emojis, or progress bars. Benchmarking output must be machine-parseable when `--json` is used.
- Human-readable format uses aligned columns and consistent units (microseconds for latency, bytes/sec for transfer).

### Documentation
- Use precise language. Define terms on first use (e.g., Coordinated Omission).
- Assume the reader understands HTTP and basic benchmarking concepts.
- Include concrete examples with expected output.

### Error Messages
- State what went wrong, what was expected, and (when possible) how to fix it.
- Example: `error: connection refused at 127.0.0.1:8080 -- is the target server running?`

## Compatibility

### Versioning
- **Strict semver** for both the CLI binary and `rewrk-core` library.
- Breaking changes only in major versions.
- Deprecation warnings in minor versions before removal.
- Library consumers can depend on `rewrk-core` with version pinning confidence.

### CLI Stability
- Flag names and output formats are part of the public API.
- JSON output schema is versioned and backward-compatible within a major version.

## Error Handling Philosophy

- **Fail loud and early.** Invalid inputs, connection failures, and protocol errors produce clear error messages and non-zero exit codes.
- No silent degradation. If rewrk cannot produce accurate measurements, it must not produce misleading ones.
- Connection errors during a benchmark run are counted and reported, not swallowed.

## Quality Standards

- Measurement correctness is more important than raw performance of the tool itself.
- Every metric displayed must be well-defined and documented.
- CO-corrected and uncorrected values must be clearly labeled when both are shown.
