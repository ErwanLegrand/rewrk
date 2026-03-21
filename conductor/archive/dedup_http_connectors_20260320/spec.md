# Spec: Deduplicate HTTP/1 and HTTP/2 Connectors

## Overview
`rewrk-core/src/connection/http1.rs` (344 lines) and `http2.rs` (346 lines) are ~85% structurally identical. The only meaningful difference is that HTTP/2 adds `conn_builder.http2_only(true)`.

## Proposed Design
Generic base connector with a configuration closure:

```rust
pub(crate) struct BaseConnector<F> {
    addr: SocketAddr,
    scheme: Scheme,
    host: String,
    uri_scheme: HttpScheme,
    uri_authority: Authority,
    host_header: HeaderValue,
    configure_builder: F,
}
```

Type aliases preserve the public API:
```rust
pub type Http1Connector = BaseConnector<Http1Config>;
pub type Http2Connector = BaseConnector<Http2Config>;
```

Single `BaseConnection` struct for the identical `execute_req()` and `usage()` implementations.

## Acceptance Criteria
- [ ] Single implementation of connector logic
- [ ] Single implementation of connection logic (execute_req, usage)
- [ ] Http1Connector and Http2Connector type aliases preserved
- [ ] All existing tests pass without modification
- [ ] HTTP/3 stub updated to use new base types
