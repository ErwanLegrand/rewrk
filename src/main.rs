use std::str::FromStr;

use ::http::{HeaderMap, Method};
use anyhow::{Context, Result};
use clap::Parser;
use hyper::body::Bytes;

mod bench;
mod cli_collector;
mod cli_producer;
mod parsing;
mod results;
mod runtime;
mod utils;

use parsing::{parse_duration, parse_header};
use rewrk_core::HttpProtocol;

#[derive(Parser)]
#[command(
    name = "ReWrk",
    version,
    about = "Benchmark HTTP/1 and HTTP/2 frameworks without pipelining bias."
)]
struct Args {
    /// Number of threads to use
    #[arg(short = 't', long, default_value_t = 1)]
    threads: usize,

    /// Number of concurrent connections
    #[arg(short = 'c', long, default_value_t = 1)]
    connections: usize,

    /// Host to benchmark (e.g., http://127.0.0.1:5050/path)
    #[arg(short = 'h', long)]
    host: String,

    /// Use HTTP/2 instead of HTTP/1.1
    #[arg(long)]
    http2: bool,

    /// Duration of the benchmark (e.g., "10s", "1h30m")
    #[arg(short = 'd', long)]
    duration: String,

    /// Display percentile table
    #[arg(long)]
    pct: bool,

    /// Display results as JSON
    #[arg(long)]
    json: bool,

    /// Number of benchmark rounds
    #[arg(short = 'r', long, default_value_t = 1)]
    rounds: usize,

    /// HTTP method (default: GET)
    #[arg(short = 'm', long)]
    method: Option<String>,

    /// Add headers (repeatable, format: "key: value")
    #[arg(short = 'H', long = "header")]
    headers: Vec<String>,

    /// Request body
    #[arg(short = 'b', long)]
    body: Option<String>,

    /// Disable TLS certificate verification
    #[arg(short = 'k', long)]
    insecure: bool,

    /// Disable coordinated omission correction
    #[arg(long = "no-co-correction")]
    no_co_correction: bool,
}

/// ReWrk
///
/// Captures CLI arguments and builds benchmarking settings and runtime to
/// suit the arguments and options.
fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    anyhow::ensure!(args.threads >= 1, "invalid parameter for 'threads': must be at least 1.");
    anyhow::ensure!(
        args.connections >= 1,
        "invalid parameter for 'connections': must be at least 1."
    );
    anyhow::ensure!(args.rounds >= 1, "invalid parameter for 'rounds': must be at least 1.");

    let protocol = if args.http2 {
        HttpProtocol::Http2
    } else {
        HttpProtocol::Http1
    };

    let duration =
        parse_duration(&args.duration).context("failed to parse duration parameter")?;

    let co_correction = !args.no_co_correction;

    let method = args
        .method
        .as_deref()
        .map(|m| Method::from_str(&m.to_uppercase()))
        .transpose()
        .context("failed to parse method")?
        .unwrap_or(Method::GET);

    let headers = args
        .headers
        .iter()
        .map(|h| parse_header(h))
        .collect::<Result<HeaderMap<_>>>()
        .context("failed to parse header")?;

    let body = Bytes::copy_from_slice(args.body.as_deref().unwrap_or_default().as_bytes());

    let settings = bench::BenchmarkSettings {
        threads: args.threads,
        connections: args.connections,
        host: args.host,
        protocol,
        duration,
        display_percentile: args.pct,
        display_json: args.json,
        rounds: args.rounds,
        method,
        headers,
        body,
        insecure: args.insecure,
        co_correction,
    };

    bench::start_benchmark(settings)
}

