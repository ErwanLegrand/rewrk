# Plan: Deduplicate HTTP/1 and HTTP/2 Connectors

## Phase 1: Extract Base Types

- [x] Task: Create BaseConnector<C: BuilderConfig> and BaseConnection structs [d45e92d]
    - [x] Define BuilderConfig trait with configure(&mut conn::Builder) method
    - [x] Define generic BaseConnector with PhantomData<C>
    - [x] Define shared BaseConnection struct
    - [x] Implement ProtocolConnector for BaseConnector<C>
    - [x] Implement ProtocolConnection for BaseConnection

- [x] Task: Replace Http1Connector/Connection with type aliases [d45e92d]
    - [x] Define Http1Config (no-op BuilderConfig)
    - [x] Http1Connector = BaseConnector<Http1Config>
    - [x] Http1Connection = BaseConnection
    - [x] All http1 tests pass

- [x] Task: Replace Http2Connector/Connection with type aliases [d45e92d]
    - [x] Define Http2Config (http2_only BuilderConfig)
    - [x] Http2Connector = BaseConnector<Http2Config>
    - [x] Http2Connection = BaseConnection
    - [x] All http2 tests pass

- [x] Task: Update ReWrkConnector enum dispatch in conn.rs [d45e92d]
    - [x] No changes needed — type aliases are transparent

- [x] Task: Run full test suite and verify [d45e92d]
    - [x] cargo test --all — 106 tests pass
    - [x] cargo clippy --all -- -D warnings — clean

- [x] Task: Conductor - User Manual Verification 'Extract Base Types' (Protocol in workflow.md)
