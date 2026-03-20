use std::str::FromStr;
use std::sync::LazyLock;

use ::http::header::HeaderName;
use ::http::{HeaderMap, HeaderValue, Method};
use anyhow::{Context, Error, Result};
use clap::Parser;
use hyper::body::Bytes;
use regex::Regex;
use tokio::time::Duration;

mod bench;
mod cli_collector;
mod cli_producer;
mod results;
mod runtime;
mod utils;

use rewrk_core::HttpProtocol;

/// Matches a string like '12d 24h 5m 45s' to a regex capture.
static DURATION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        "(?P<days>[0-9]+)d|(?P<hours>[0-9]+)h|(?P<minutes>[0-9]+)m|(?P<seconds>[0-9]+)s",
    )
    .unwrap()
});

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

/// Parses a duration string from the CLI to a Duration.
/// '11d 3h 32m 4s' -> Duration
///
/// If no matches are found for the string or an invalid match
/// is captured an error message is returned.
fn parse_duration(duration: &str) -> Result<Duration> {
    let mut dur = Duration::default();

    for cap in DURATION_RE.captures_iter(duration) {
        let add_to = if let Some(days) = cap.name("days") {
            let days = days.as_str().parse::<u64>()?;
            let seconds = days
                .checked_mul(24 * 60 * 60)
                .ok_or_else(|| anyhow::anyhow!("duration overflow: {} days", days))?;
            Duration::from_secs(seconds)
        } else if let Some(hours) = cap.name("hours") {
            let hours = hours.as_str().parse::<u64>()?;
            let seconds = hours
                .checked_mul(60 * 60)
                .ok_or_else(|| anyhow::anyhow!("duration overflow: {} hours", hours))?;
            Duration::from_secs(seconds)
        } else if let Some(minutes) = cap.name("minutes") {
            let minutes = minutes.as_str().parse::<u64>()?;
            let seconds = minutes
                .checked_mul(60)
                .ok_or_else(|| anyhow::anyhow!("duration overflow: {} minutes", minutes))?;
            Duration::from_secs(seconds)
        } else if let Some(seconds) = cap.name("seconds") {
            let seconds = seconds.as_str().parse::<u64>()?;
            Duration::from_secs(seconds)
        } else {
            return Err(Error::msg(format!("invalid match: {:?}", cap)));
        };

        dur += add_to;
    }

    if dur.as_secs() == 0 {
        return Err(Error::msg(format!(
            "failed to extract any valid duration from {}",
            duration
        )));
    }

    Ok(dur)
}

fn parse_header(value: &str) -> Result<(HeaderName, HeaderValue)> {
    let (key, value) = value
        .split_once(": ")
        .context("Header value missing colon (\": \")")?;
    let key = HeaderName::from_str(key).context("Invalid header name")?;
    let value = HeaderValue::from_str(value).context("Invalid header value")?;
    Ok((key, value))
}
