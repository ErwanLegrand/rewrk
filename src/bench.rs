use std::fmt::Display;
use std::time::Duration;

use ::http::{HeaderMap, Method, Uri};
use anyhow::{anyhow, Result};
use colored::*;
use hyper::body::Bytes;
use rewrk_core::{HttpProtocol, ReWrkBenchmark};

use crate::cli_collector::CliCollector;
use crate::cli_producer::CliProducer;
use crate::results;
use crate::runtime;
use crate::utils::div_mod;

/// The customisable settings that build the benchmark's behaviour.
#[derive(Clone, Debug)]
pub struct BenchmarkSettings {
    /// The number of worker threads given to Tokio's runtime.
    pub threads: usize,

    /// The amount of concurrent connections when connecting to the
    /// framework.
    pub connections: usize,

    /// The host connection / url.
    pub host: String,

    /// The HTTP protocol to use (HTTP/1 or HTTP/2).
    pub protocol: HttpProtocol,

    /// The duration of the benchmark.
    pub duration: Duration,

    /// Display the percentile table.
    pub display_percentile: bool,

    /// Display the result data as a json.
    pub display_json: bool,

    /// The number of rounds to repeat.
    pub rounds: usize,

    /// The request method.
    pub method: Method,

    /// Additional request headers.
    pub headers: HeaderMap,

    /// Request body.
    pub body: Bytes,

    /// Disable TLS certificate verification (for self-signed certs).
    pub insecure: bool,

    /// Enable coordinated omission correction for latency stats.
    pub co_correction: bool,
}

/// Builds the runtime with the given settings and blocks on the main future.
pub fn start_benchmark(settings: BenchmarkSettings) -> anyhow::Result<()> {
    let rt = runtime::get_rt(settings.threads);
    let rounds = settings.rounds;
    let is_json = settings.display_json;
    for i in 0..rounds {
        if !is_json {
            println!("Beginning round {}...", i + 1);
        }

        rt.block_on(run(settings.clone()))?;

        // Adds a line separator between rounds unless it's formatting
        // as a json, for readability.
        if !is_json {
            println!();
        };
    }
    Ok(())
}

/// Controls the benchmark itself using the rewrk-core engine.
///
/// A `ReWrkBenchmark` is created with a `CliProducer` that generates
/// request batches and a `CliCollector` that aggregates Sample histograms.
///
/// Once the benchmark completes the collector's aggregated histograms
/// are used to display results.
async fn run(settings: BenchmarkSettings) -> Result<()> {
    // Parse URI
    let uri: Uri = settings
        .host
        .trim()
        .parse()
        .map_err(|e| anyhow!("error parsing uri: {}", e))?;

    // Build producer with path-only URI
    let path = uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let path_uri: Uri = path.parse()?;

    let producer = CliProducer::new(
        path_uri,
        settings.method.clone(),
        settings.headers.clone(),
        settings.body.clone(),
        settings.duration,
    );

    let collector = CliCollector::new();

    // Create benchmark using rewrk-core
    let mut benchmarker = ReWrkBenchmark::create_with_tls(
        uri,
        settings.connections,
        settings.protocol,
        producer,
        collector,
        settings.insecure,
    )
    .await
    .map_err(|e| anyhow!("benchmark setup error: {}", e))?;

    benchmarker.set_num_workers(settings.threads);
    // Use a sample window larger than the duration so all data
    // lands in a single sample for accurate aggregation.
    benchmarker.set_sample_window(settings.duration + Duration::from_secs(60));

    if !settings.display_json {
        println!(
            "Benchmarking {} connections @ {} for {}",
            string(settings.connections).cyan(),
            settings.host,
            humanize(settings.duration),
        );
    }

    benchmarker.run().await;
    let collector = benchmarker
        .consume_collector()
        .await
        .map_err(|e| anyhow!("collector error: {}", e))?;

    // Display results from aggregated histograms
    results::display_results(&collector, &settings);

    Ok(())
}

/// Uber lazy way of just stringing everything and limiting it to 2 d.p
pub(crate) fn string<T: Display>(value: T) -> String {
    format!("{:.2}", value)
}

/// Turns a fairly un-readable float in seconds / Duration into a human
/// friendly string.
///
/// E.g.
/// 10,000 seconds -> '2 hours, 46 minutes, 40 seconds'
pub(crate) fn humanize(time: Duration) -> String {
    let seconds = time.as_secs();

    let (minutes, seconds) = div_mod(seconds, 60);
    let (hours, minutes) = div_mod(minutes, 60);
    let (days, hours) = div_mod(hours, 24);

    let mut human = Vec::new();

    if days != 0 {
        human.push(format!("{} day(s)", days));
    };

    if hours != 0 {
        human.push(format!("{} hour(s)", hours));
    };

    if minutes != 0 {
        human.push(format!("{} minute(s)", minutes));
    };

    if seconds != 0 {
        human.push(format!("{} second(s)", seconds));
    };

    human.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- humanize tests ---

    #[test]
    fn humanize_seconds_only() {
        assert_eq!(humanize(Duration::from_secs(10)), "10 second(s)");
    }

    #[test]
    fn humanize_minutes_and_seconds() {
        assert_eq!(humanize(Duration::from_secs(90)), "1 minute(s), 30 second(s)");
    }

    #[test]
    fn humanize_hours_minutes_seconds() {
        assert_eq!(humanize(Duration::from_secs(3661)), "1 hour(s), 1 minute(s), 1 second(s)");
    }

    #[test]
    fn humanize_one_day() {
        assert_eq!(humanize(Duration::from_secs(86400)), "1 day(s)");
    }

    // --- string tests ---

    #[test]
    fn string_rounds_to_two_decimal_places() {
        assert_eq!(string(3.14159_f64), "3.14");
    }

    #[test]
    fn string_integer_formats_with_two_decimals() {
        assert_eq!(string(100_f64), "100.00");
    }
}
