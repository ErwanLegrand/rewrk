# Plan: Hot-Path Safety — Eliminate `unwrap()` in `execute_req`

## Phase 1: Refactor Connection Types

- [x] Task: Add URI validation to Http1Connector and Http2Connector constructors [3c07c86]
    - [x] Write tests that Http1Connector::new rejects a URI without scheme
    - [x] Write tests that Http1Connector::new rejects a URI without authority
    - [x] Change Http1Connector::new to validate and store Scheme + Authority as typed fields
    - [x] Apply same changes to Http2Connector
    - [x] Verify all existing tests still pass
- [x] Task: Remove unwrap() from execute_req in Http1Connection and Http2Connection [3c07c86]
    - [x] Write test that execute_req works correctly with pre-validated URI components
    - [x] Refactor Http1Connection to store Scheme and Authority as fields, use them directly in execute_req
    - [x] Refactor Http2Connection identically
    - [x] Verify zero unwrap() calls remain in execute_req methods
    - [x] Verify all existing tests still pass
- [x] Task: Update ReWrkConnector enum and create_connector [3c07c86]
    - [x] Verify create_connector already validates URI (it does via Error::MissingScheme/MissingHost)
    - [x] Ensure ReWrkConnector::http1/http2 pass-through works with updated constructors
    - [x] Run cargo clippy --all and verify zero warnings
- [~] Task: Conductor - User Manual Verification 'Refactor Connection Types' (Protocol in workflow.md)
