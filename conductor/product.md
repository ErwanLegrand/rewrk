# Product Guide

## Vision

rewrk is a modern, accurate HTTP benchmarking tool written in Rust. It exists to give developers and the open-source community a synthetic benchmark utility that produces correct, unbiased latency measurements -- something most benchmarking tools fail at due to Coordinated Omission.

## Goals

### 1. Measurement Accuracy (Primary)
rewrk's most important quality is producing correct latency numbers. This means:
- Detecting and correcting for Coordinated Omission so that latency percentiles reflect real-world service behavior under load, not just the fast-path responses.
- Using HDR Histograms with microsecond precision for percentile calculations.
- Clearly distinguishing between uncorrected and CO-corrected latency in output.

### 2. HTTP/3 Future-Proofing
The connection layer will be refactored into a protocol-agnostic abstraction so that HTTP/3 (via hyper + h3/quinn when ready) can be integrated with minimal changes. This means:
- A trait-based connection interface that HTTP/1.1, HTTP/2, and eventually HTTP/3 all implement.
- TLS/ALPN negotiation abstracted behind the same interface.
- No tight coupling between the benchmarking engine and a specific HTTP protocol version.

## Non-Goals
- **Realistic traffic simulation:** rewrk is a synthetic benchmark tool. It does not simulate user behavior, session patterns, or variable request rates.
- **Load testing orchestration:** rewrk runs on a single machine. Distributed load generation is out of scope.

## Target Users
- **Backend developers** benchmarking their HTTP services during development.
- **Open-source community** users and contributors who choose rewrk as an alternative to wrk, wrk2, hey, or k6.

## Architecture Overview
- **Workspace:** Two crates -- `rewrk` (CLI binary) and `rewrk-core` (library).
- **Runtime:** Tokio multi-threaded async runtime.
- **Extensibility:** Producer, SampleCollector, and ResponseValidator traits allow library consumers to customize request generation, metrics collection, and response validation.
