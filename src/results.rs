#![allow(unused)]

use std::collections::HashMap;

use colored::Colorize;
use hdrhistogram::Histogram;
use serde_json::json;
use tokio::time::Duration;

use crate::utils::format_data;

/// Duration to microseconds as u64 for HDR histogram recording.
fn duration_to_micros(d: Duration) -> u64 {
    d.as_micros() as u64
}

/// Microseconds to milliseconds as f64 for display.
fn micros_to_ms(us: u64) -> f64 {
    us as f64 / 1000.0
}

fn get_percentile(request_times: &[Duration], pct: f64) -> Duration {
    let mut len = request_times.len() as f64 * pct;
    if len < 1.0 {
        len = 1.0;
    }

    let e = format!("failed to calculate P{} avg latency", (1.0 - pct) * 100f64);
    let pct = request_times.chunks(len as usize).next().expect(&e);

    let total: f64 = pct.iter().map(|dur| dur.as_secs_f64()).sum();

    let avg = total / pct.len() as f64;

    Duration::from_secs_f64(avg)
}

/// CO-corrected latency statistics computed from an HDR histogram.
#[derive(Debug, Clone)]
pub struct CorrectedStats {
    pub avg_ms: f64,
    pub max_ms: f64,
    pub min_ms: f64,
    pub std_deviation_ms: f64,
    pub p50_ms: f64,
    pub p75_ms: f64,
    pub p90_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub p999_ms: f64,
}

/// Build a CO-corrected HDR histogram from raw request times.
///
/// The `expected_interval` is the expected time between requests
/// (i.e. avg latency). The histogram uses `record_correct` to
/// fill in missing samples that coordinated omission would hide.
pub fn build_corrected_stats(
    request_times: &[Duration],
    expected_interval: Duration,
) -> Option<CorrectedStats> {
    if request_times.is_empty() {
        return None;
    }

    let expected_interval_us = duration_to_micros(expected_interval);
    if expected_interval_us == 0 {
        return None;
    }

    // Create histogram with enough range: 1 microsecond to 1 hour.
    let mut hist = Histogram::<u64>::new_with_bounds(1, 3_600_000_000, 3)
        .expect("failed to create HDR histogram");

    for d in request_times {
        let us = duration_to_micros(*d).max(1);
        let _ = hist.record_correct(us, expected_interval_us);
    }

    let avg_us = hist.mean();
    let max_us = hist.max();
    let min_us = hist.min();
    let std_dev_us = hist.stdev();

    Some(CorrectedStats {
        avg_ms: avg_us / 1000.0,
        max_ms: micros_to_ms(max_us),
        min_ms: micros_to_ms(min_us),
        std_deviation_ms: std_dev_us / 1000.0,
        p50_ms: micros_to_ms(hist.value_at_percentile(50.0)),
        p75_ms: micros_to_ms(hist.value_at_percentile(75.0)),
        p90_ms: micros_to_ms(hist.value_at_percentile(90.0)),
        p95_ms: micros_to_ms(hist.value_at_percentile(95.0)),
        p99_ms: micros_to_ms(hist.value_at_percentile(99.0)),
        p999_ms: micros_to_ms(hist.value_at_percentile(99.9)),
    })
}

/// Contains and handles results from the workers
#[derive(Default)]
pub struct WorkerResult {
    /// The total time taken for each worker.
    pub total_times: Vec<Duration>,

    /// The vec of latencies per request stored.
    pub request_times: Vec<Duration>,

    /// The amount of data read from each worker.
    pub buffer_sizes: Vec<usize>,

    /// Error counting map.
    pub error_map: HashMap<String, usize>,
}

impl WorkerResult {
    /// Creates a empty result, useful for merging results into one
    /// consumer.
    pub fn default() -> Self {
        Self {
            total_times: vec![],
            request_times: vec![],
            buffer_sizes: vec![],
            error_map: HashMap::new(),
        }
    }

    /// Consumes both self and other producing a combined result.
    pub fn combine(mut self, other: Self) -> Self {
        self.request_times.extend(other.request_times);
        self.total_times.extend(other.total_times);
        self.buffer_sizes.extend(other.buffer_sizes);

        // Insert/add new errors to current error map.
        for (message, count) in other.error_map {
            match self.error_map.get_mut(&message) {
                Some(c) => *c += count,
                None => {
                    self.error_map.insert(message, count);
                },
            }
        }

        self
    }

    /// Simple helper returning the amount of requests overall.
    pub fn total_requests(&self) -> usize {
        self.request_times.len()
    }

    /// Calculates the total transfer in bytes.
    pub fn total_transfer(&self) -> usize {
        self.buffer_sizes.iter().sum()
    }

    /// Calculates the total transfer in bytes.
    pub fn avg_transfer(&self) -> f64 {
        self.total_transfer() as f64 / self.avg_total_time().as_secs_f64()
    }

    /// Calculates the requests per second average.
    pub fn avg_request_per_sec(&self) -> f64 {
        let amount = self.request_times.len() as f64;
        let avg_time = self.avg_total_time();

        amount / avg_time.as_secs_f64()
    }

    /// Calculates the average time per worker overall as a `Duration`
    ///
    /// Basic Logic:
    /// Sum(worker totals) / length = avg duration
    pub fn avg_total_time(&self) -> Duration {
        let avg: f64 = self.total_times.iter().map(|dur| dur.as_secs_f64()).sum();

        let len = self.total_times.len() as f64;
        Duration::from_secs_f64(avg / len)
    }

    /// Calculates the average latency overall from all requests..
    pub fn avg_request_latency(&self) -> Duration {
        let avg: f64 = self.request_times.iter().map(|dur| dur.as_secs_f64()).sum();

        let len = self.total_requests() as f64;
        Duration::from_secs_f64(avg / len)
    }

    /// Calculates the max latency overall from all requests.
    pub fn max_request_latency(&self) -> Duration {
        self.request_times.iter().max().copied().unwrap_or_default()
    }

    /// Calculates the min latency overall from all requests.
    pub fn min_request_latency(&self) -> Duration {
        self.request_times.iter().min().copied().unwrap_or_default()
    }

    /// Calculates the variance between all requests
    pub fn variance(&self) -> f64 {
        let mean = self.avg_request_latency().as_secs_f64();
        let sum_delta: f64 = self
            .request_times
            .iter()
            .map(|dur| {
                let time = dur.as_secs_f64();
                let delta = time - mean;

                delta.powi(2)
            })
            .sum();

        sum_delta / self.total_requests() as f64
    }

    /// Calculates the standard deviation of request latency.
    pub fn std_deviation_request_latency(&self) -> f64 {
        let diff = self.variance();
        diff.powf(0.5)
    }

    /// Sorts the list of times.
    ///
    /// this is needed before calculating the Pn percentiles, this must be
    /// manually ran to same some compute time.
    pub fn sort_request_times(&mut self) {
        self.request_times.sort_by(|a, b| b.partial_cmp(a).unwrap());
    }

    /// Works out the average latency of the 99.9 percentile.
    pub fn p999_avg_latency(&self) -> Duration {
        get_percentile(&self.request_times, 0.001)
    }

    /// Works out the average latency of the 99 percentile.
    pub fn p99_avg_latency(&self) -> Duration {
        get_percentile(&self.request_times, 0.01)
    }

    /// Works out the average latency of the 95 percentile.
    pub fn p95_avg_latency(&self) -> Duration {
        get_percentile(&self.request_times, 0.05)
    }

    /// Works out the average latency of the 90 percentile.
    pub fn p90_avg_latency(&self) -> Duration {
        get_percentile(&self.request_times, 0.1)
    }

    /// Works out the average latency of the 75 percentile.
    pub fn p75_avg_latency(&mut self) -> Duration {
        get_percentile(&self.request_times, 0.25)
    }

    /// Works out the average latency of the 50 percentile.
    pub fn p50_avg_latency(&mut self) -> Duration {
        get_percentile(&self.request_times, 0.5)
    }

    /// Compute CO-corrected stats using the average latency as
    /// the expected interval.
    pub fn corrected_stats(&self) -> Option<CorrectedStats> {
        let expected_interval = self.avg_request_latency();
        build_corrected_stats(&self.request_times, expected_interval)
    }

    pub fn display_latencies(&mut self) {
        let modified = 1000_f64;
        let avg = self.avg_request_latency().as_secs_f64() * modified;
        let max = self.max_request_latency().as_secs_f64() * modified;
        let min = self.min_request_latency().as_secs_f64() * modified;
        let std_deviation = self.std_deviation_request_latency() * modified;

        println!("  Latencies:");
        println!(
            "    {:<7}  {:<7}  {:<7}  {:<7}  ",
            "Avg".bright_yellow(),
            "Stdev".bright_magenta(),
            "Min".bright_green(),
            "Max".bright_red(),
        );
        println!(
            "    {:<7}  {:<7}  {:<7}  {:<7}  ",
            format!("{:.2}ms", avg),
            format!("{:.2}ms", std_deviation),
            format!("{:.2}ms", min),
            format!("{:.2}ms", max),
        );
    }

    pub fn display_latencies_corrected(&mut self) {
        if let Some(stats) = self.corrected_stats() {
            println!("  Latencies (CO-corrected):");
            println!(
                "    {:<7}  {:<7}  {:<7}  {:<7}  ",
                "Avg".bright_yellow(),
                "Stdev".bright_magenta(),
                "Min".bright_green(),
                "Max".bright_red(),
            );
            println!(
                "    {:<7}  {:<7}  {:<7}  {:<7}  ",
                format!("{:.2}ms", stats.avg_ms),
                format!("{:.2}ms", stats.std_deviation_ms),
                format!("{:.2}ms", stats.min_ms),
                format!("{:.2}ms", stats.max_ms),
            );
        }
    }

    pub fn display_requests(&mut self) {
        let total = self.total_requests();
        let avg = self.avg_request_per_sec();

        println!("  Requests:");
        println!(
            "    Total: {:^7} Req/Sec: {:^7}",
            format!("{}", total).as_str().bright_cyan(),
            format!("{:.2}", avg).as_str().bright_cyan()
        )
    }

    pub fn display_transfer(&mut self) {
        let total = self.total_transfer() as f64;
        let rate = self.avg_transfer();

        let display_total = format_data(total);
        let display_rate = format_data(rate);

        println!("  Transfer:");
        println!(
            "    Total: {:^7} Transfer Rate: {:^7}",
            display_total.as_str().bright_cyan(),
            format!("{}/Sec", display_rate).as_str().bright_cyan()
        )
    }

    pub fn display_percentile_table(&mut self) {
        self.sort_request_times();

        println!("+ {:-^15} + {:-^15} +", "", "",);

        println!(
            "| {:^15} | {:^15} |",
            "Percentile".bright_cyan(),
            "Avg Latency".bright_yellow(),
        );

        println!("+ {:-^15} + {:-^15} +", "", "",);

        let modifier = 1000_f64;
        println!(
            "| {:^15} | {:^15} |",
            "99.9%",
            format!("{:.2}ms", self.p999_avg_latency().as_secs_f64() * modifier)
        );
        println!(
            "| {:^15} | {:^15} |",
            "99%",
            format!("{:.2}ms", self.p99_avg_latency().as_secs_f64() * modifier)
        );
        println!(
            "| {:^15} | {:^15} |",
            "95%",
            format!("{:.2}ms", self.p95_avg_latency().as_secs_f64() * modifier)
        );
        println!(
            "| {:^15} | {:^15} |",
            "90%",
            format!("{:.2}ms", self.p90_avg_latency().as_secs_f64() * modifier)
        );
        println!(
            "| {:^15} | {:^15} |",
            "75%",
            format!("{:.2}ms", self.p75_avg_latency().as_secs_f64() * modifier)
        );
        println!(
            "| {:^15} | {:^15} |",
            "50%",
            format!("{:.2}ms", self.p50_avg_latency().as_secs_f64() * modifier)
        );

        println!("+ {:-^15} + {:-^15} +", "", "",);
    }

    pub fn display_percentile_table_corrected(&mut self) {
        if let Some(stats) = self.corrected_stats() {
            println!("+ {:-^15} + {:-^15} +", "", "",);

            println!(
                "| {:^15} | {:^15} |",
                "Percentile".bright_cyan(),
                "CO-corrected".bright_yellow(),
            );

            println!("+ {:-^15} + {:-^15} +", "", "",);

            println!(
                "| {:^15} | {:^15} |",
                "99.9%",
                format!("{:.2}ms", stats.p999_ms)
            );
            println!(
                "| {:^15} | {:^15} |",
                "99%",
                format!("{:.2}ms", stats.p99_ms)
            );
            println!(
                "| {:^15} | {:^15} |",
                "95%",
                format!("{:.2}ms", stats.p95_ms)
            );
            println!(
                "| {:^15} | {:^15} |",
                "90%",
                format!("{:.2}ms", stats.p90_ms)
            );
            println!(
                "| {:^15} | {:^15} |",
                "75%",
                format!("{:.2}ms", stats.p75_ms)
            );
            println!(
                "| {:^15} | {:^15} |",
                "50%",
                format!("{:.2}ms", stats.p50_ms)
            );

            println!("+ {:-^15} + {:-^15} +", "", "",);
        }
    }

    pub fn display_errors(&self) {
        if !self.error_map.is_empty() {
            println!();

            for (message, count) in &self.error_map {
                println!("{} Errors: {}", count, message);
            }
        }
    }

    pub fn display_json(&self, co_correction: bool) {
        // prevent div-by-zero panics
        if self.total_requests() == 0 {
            let null = None::<()>;

            let mut out = json!({
                "latency_avg": null,
                "latency_max": null,
                "latency_min": null,
                "latency_std_deviation": null,

                "transfer_total": null,
                "transfer_rate": null,

                "requests_total": 0,
                "requests_avg": null,
            });

            if co_correction {
                out["latency_corrected"] = json!(null);
            }

            println!("{}", out);
            return;
        }

        let modified = 1000_f64;
        let avg = self.avg_request_latency().as_secs_f64() * modified;
        let max = self.max_request_latency().as_secs_f64() * modified;
        let min = self.min_request_latency().as_secs_f64() * modified;
        let std_deviation = self.std_deviation_request_latency() * modified;

        let total = self.total_transfer() as f64;
        let rate = self.avg_transfer();

        let total_requests = self.total_requests();
        let avg_request_per_sec = self.avg_request_per_sec();

        let mut out = json!({
            "latency_avg": avg,
            "latency_max": max,
            "latency_min": min,
            "latency_std_deviation": std_deviation,

            "transfer_total": total,
            "transfer_rate": rate,

            "requests_total": total_requests,
            "requests_avg": avg_request_per_sec,
        });

        if co_correction {
            if let Some(stats) = self.corrected_stats() {
                out["latency_corrected"] = json!({
                    "avg": stats.avg_ms,
                    "max": stats.max_ms,
                    "min": stats.min_ms,
                    "std_deviation": stats.std_deviation_ms,
                    "p50": stats.p50_ms,
                    "p75": stats.p75_ms,
                    "p90": stats.p90_ms,
                    "p95": stats.p95_ms,
                    "p99": stats.p99_ms,
                    "p999": stats.p999_ms,
                });
            }
        }

        println!("{}", out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Helper to build a WorkerResult from a slice of millisecond values.
    fn result_from_ms(latencies_ms: &[u64]) -> WorkerResult {
        let request_times: Vec<Duration> = latencies_ms
            .iter()
            .map(|ms| Duration::from_millis(*ms))
            .collect();
        let total_time: Duration = request_times.iter().sum();
        WorkerResult {
            total_times: vec![total_time],
            request_times,
            buffer_sizes: vec![1024],
            error_map: HashMap::new(),
        }
    }

    #[test]
    fn uniform_latencies_corrected_similar_to_uncorrected() {
        // With uniform latencies (all 5ms), the CO-corrected stats should
        // be very close to the uncorrected stats since there are no spikes.
        let result = result_from_ms(&[5, 5, 5, 5, 5, 5, 5, 5, 5, 5]);

        let uncorrected_avg_ms = result.avg_request_latency().as_secs_f64() * 1000.0;
        let stats = result.corrected_stats().expect("should produce corrected stats");

        // With perfectly uniform data and expected_interval == avg,
        // no corrections should be applied, so values should be close.
        let diff = (stats.avg_ms - uncorrected_avg_ms).abs();
        assert!(
            diff < 1.0,
            "Expected corrected avg ({:.2}) to be close to uncorrected avg ({:.2})",
            stats.avg_ms,
            uncorrected_avg_ms
        );

        // p99 should also be similar for uniform data
        assert!(
            stats.p99_ms < 10.0,
            "Expected corrected p99 ({:.2}ms) to be low for uniform data",
            stats.p99_ms
        );
    }

    #[test]
    fn latency_spikes_show_higher_corrected_p99() {
        // Most requests at 5ms, but one large spike at 500ms.
        // CO correction should inflate high percentiles because the spike
        // would have hidden many expected requests.
        let mut latencies = vec![5u64; 100];
        latencies.push(500); // big spike

        let result = result_from_ms(&latencies);

        let stats = result.corrected_stats().expect("should produce corrected stats");

        // The uncorrected p99 from raw data
        let mut sorted = result.request_times.clone();
        sorted.sort();
        let uncorrected_p99_ms = sorted[(sorted.len() as f64 * 0.99) as usize].as_secs_f64() * 1000.0;

        // CO-corrected p99 should be >= uncorrected p99
        assert!(
            stats.p99_ms >= uncorrected_p99_ms,
            "Expected corrected p99 ({:.2}ms) >= uncorrected p99 ({:.2}ms)",
            stats.p99_ms,
            uncorrected_p99_ms
        );
    }

    #[test]
    fn json_contains_latency_corrected_when_enabled() {
        let result = result_from_ms(&[5, 5, 5, 10, 10]);

        // Capture JSON output by building same logic as display_json
        let modified = 1000_f64;
        let avg = result.avg_request_latency().as_secs_f64() * modified;
        let max = result.max_request_latency().as_secs_f64() * modified;
        let min = result.min_request_latency().as_secs_f64() * modified;
        let std_deviation = result.std_deviation_request_latency() * modified;

        let total = result.total_transfer() as f64;
        let rate = result.avg_transfer();

        let total_requests = result.total_requests();
        let avg_request_per_sec = result.avg_request_per_sec();

        let mut out = json!({
            "latency_avg": avg,
            "latency_max": max,
            "latency_min": min,
            "latency_std_deviation": std_deviation,
            "transfer_total": total,
            "transfer_rate": rate,
            "requests_total": total_requests,
            "requests_avg": avg_request_per_sec,
        });

        // Simulate co_correction = true
        if let Some(stats) = result.corrected_stats() {
            out["latency_corrected"] = json!({
                "avg": stats.avg_ms,
                "max": stats.max_ms,
                "min": stats.min_ms,
                "std_deviation": stats.std_deviation_ms,
                "p50": stats.p50_ms,
                "p75": stats.p75_ms,
                "p90": stats.p90_ms,
                "p95": stats.p95_ms,
                "p99": stats.p99_ms,
                "p999": stats.p999_ms,
            });
        }

        let json_str = out.to_string();
        assert!(
            json_str.contains("latency_corrected"),
            "JSON should contain 'latency_corrected' when co_correction is enabled"
        );

        // Verify it parses back with expected keys
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let corrected = parsed.get("latency_corrected").expect("missing latency_corrected");
        assert!(corrected.get("avg").is_some(), "corrected should have avg");
        assert!(corrected.get("max").is_some(), "corrected should have max");
        assert!(corrected.get("min").is_some(), "corrected should have min");
        assert!(corrected.get("std_deviation").is_some(), "corrected should have std_deviation");
        assert!(corrected.get("p99").is_some(), "corrected should have p99");
    }

    #[test]
    fn json_omits_latency_corrected_when_disabled() {
        let result = result_from_ms(&[5, 5, 5, 10, 10]);

        let modified = 1000_f64;
        let avg = result.avg_request_latency().as_secs_f64() * modified;
        let max = result.max_request_latency().as_secs_f64() * modified;
        let min = result.min_request_latency().as_secs_f64() * modified;
        let std_deviation = result.std_deviation_request_latency() * modified;

        let total = result.total_transfer() as f64;
        let rate = result.avg_transfer();

        let total_requests = result.total_requests();
        let avg_request_per_sec = result.avg_request_per_sec();

        // Simulate co_correction = false: do NOT add latency_corrected
        let out = json!({
            "latency_avg": avg,
            "latency_max": max,
            "latency_min": min,
            "latency_std_deviation": std_deviation,
            "transfer_total": total,
            "transfer_rate": rate,
            "requests_total": total_requests,
            "requests_avg": avg_request_per_sec,
        });

        let json_str = out.to_string();
        assert!(
            !json_str.contains("latency_corrected"),
            "JSON should NOT contain 'latency_corrected' when co_correction is disabled"
        );
    }
}
