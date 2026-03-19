# Plan: Hot-Path Safety — Eliminate `unwrap()` in `execute_req`

## Phase 1: Refactor Connection Types

- [ ] Task: Add URI validation to Http1Connector and Http2Connector constructors
    - [ ] Write tests that Http1Connector::new rejects a URI without scheme
    - [ ] Write tests that Http1Connector::new rejects a URI without authority
    - [ ] Change Http1Connector::new to validate and store Scheme + Authority as typed fields
    - [ ] Apply same changes to Http2Connector
    - [ ] Verify all existing tests still pass
- [ ] Task: Remove unwrap() from execute_req in Http1Connection and Http2Connection
    - [ ] Write test that execute_req works correctly with pre-validated URI components
    - [ ] Refactor Http1Connection to store Scheme and Authority as fields, use them directly in execute_req
    - [ ] Refactor Http2Connection identically
    - [ ] Verify zero unwrap() calls remain in execute_req methods
    - [ ] Verify all existing tests still pass
- [ ] Task: Update ReWrkConnector enum and create_connector
    - [ ] Verify create_connector already validates URI (it does via Error::MissingScheme/MissingHost)
    - [ ] Ensure ReWrkConnector::http1/http2 pass-through works with updated constructors
    - [ ] Run cargo clippy --all and verify zero warnings
- [ ] Task: Conductor - User Manual Verification 'Refactor Connection Types' (Protocol in workflow.md)
