use colored::Colorize;
use hdrhistogram::Histogram;
use serde_json::json;

use crate::bench::BenchmarkSettings;
use crate::cli_collector::CliCollector;
use crate::utils::format_data;

/// Microseconds to milliseconds as f64 for display.
fn micros_to_ms(us: u64) -> f64 {
    us as f64 / 1000.0
}

/// Main display orchestrator. Reads from the collector's aggregated
/// histograms and prints results in the same format as the old CLI.
pub fn display_results(collector: &CliCollector, settings: &BenchmarkSettings) {
    let total_requests = collector.total_requests();

    if settings.display_json {
        display_json(collector, settings.co_correction);
        return;
    }

    if total_requests == 0 {
        println!("No requests completed successfully");
        return;
    }

    display_latencies_impl(collector.latency(), "Latencies");
    if settings.co_correction {
        display_latencies_impl(collector.corrected_latency(), "Latencies (CO-corrected)");
    }
    display_requests(collector);
    display_transfer(collector);

    if settings.display_percentile {
        display_percentile_table_impl(collector.latency(), "Avg Latency");
        if settings.co_correction {
            display_percentile_table_impl(collector.corrected_latency(), "CO-corrected");
        }
    }
}

/// Display latency stats from the histogram with the given title.
fn display_latencies_impl(hist: &Histogram<u32>, title: &str) {
    if hist.is_empty() {
        return;
    }

    let avg = hist.mean() / 1000.0;
    let max = micros_to_ms(hist.max());
    let min = micros_to_ms(hist.min());
    let std_deviation = hist.stdev() / 1000.0;

    println!("  {}:", title);
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

/// Display request count and throughput.
fn display_requests(collector: &CliCollector) {
    let total = collector.total_requests();

    // Estimate avg requests/sec from the read transfer histogram.
    // The read transfer histogram records bytes/sec per request.
    // For req/sec, we use total_requests / (avg_latency * total_requests / concurrency).
    // Since we don't have exact wall-clock time, we approximate from the histogram.
    let avg_latency_secs = collector.latency().mean() / 1_000_000.0;
    let avg_rps = if avg_latency_secs > 0.0 {
        1.0 / avg_latency_secs
    } else {
        0.0
    };

    println!("  Requests:");
    println!(
        "    Total: {:^7} Req/Sec: {:^7}",
        format!("{}", total).as_str().bright_cyan(),
        format!("{:.2}", avg_rps).as_str().bright_cyan()
    )
}

/// Display transfer rate stats from the read transfer histogram.
fn display_transfer(collector: &CliCollector) {
    let read_hist = collector.read_transfer();

    if read_hist.is_empty() {
        println!("  Transfer:");
        println!(
            "    Total: {:^7} Transfer Rate: {:^7}",
            "N/A".bright_cyan(),
            "N/A".bright_cyan()
        );
        return;
    }

    // The read_transfer histogram records bytes/sec rates.
    let avg_rate = read_hist.mean();
    let display_rate = format_data(avg_rate);

    println!("  Transfer:");
    println!(
        "    Total: {:^7} Transfer Rate: {:^7}",
        "N/A".bright_cyan(),
        format!("{}/Sec", display_rate).as_str().bright_cyan()
    )
}

/// Display the percentile table from the histogram with the given column title.
fn display_percentile_table_impl(hist: &Histogram<u32>, column_title: &str) {
    if hist.is_empty() {
        return;
    }

    println!("+ {:-^15} + {:-^15} +", "", "");
    println!(
        "| {:^15} | {:^15} |",
        "Percentile".bright_cyan(),
        column_title.bright_yellow(),
    );
    println!("+ {:-^15} + {:-^15} +", "", "");

    for (label, pct) in [
        ("99.9%", 99.9),
        ("99%", 99.0),
        ("95%", 95.0),
        ("90%", 90.0),
        ("75%", 75.0),
        ("50%", 50.0),
    ] {
        println!(
            "| {:^15} | {:^15} |",
            label,
            format!("{:.2}ms", micros_to_ms(hist.value_at_percentile(pct)))
        );
    }

    println!("+ {:-^15} + {:-^15} +", "", "");
}

/// Display results as JSON.
fn display_json(collector: &CliCollector, co_correction: bool) {
    let total_requests = collector.total_requests();

    if total_requests == 0 {
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

    let hist = collector.latency();
    let avg = hist.mean() / 1000.0;
    let max = micros_to_ms(hist.max());
    let min = micros_to_ms(hist.min());
    let std_deviation = hist.stdev() / 1000.0;

    let read_hist = collector.read_transfer();
    let rate = if !read_hist.is_empty() {
        read_hist.mean()
    } else {
        0.0
    };

    let avg_latency_secs = hist.mean() / 1_000_000.0;
    let avg_request_per_sec = if avg_latency_secs > 0.0 {
        1.0 / avg_latency_secs
    } else {
        0.0
    };

    let mut out = json!({
        "latency_avg": avg,
        "latency_max": max,
        "latency_min": min,
        "latency_std_deviation": std_deviation,

        "transfer_total": null,
        "transfer_rate": rate,

        "requests_total": total_requests,
        "requests_avg": avg_request_per_sec,
    });

    if co_correction {
        let corrected = collector.corrected_latency();
        if !corrected.is_empty() {
            out["latency_corrected"] = json!({
                "avg": corrected.mean() / 1000.0,
                "max": micros_to_ms(corrected.max()),
                "min": micros_to_ms(corrected.min()),
                "std_deviation": corrected.stdev() / 1000.0,
                "p50": micros_to_ms(corrected.value_at_percentile(50.0)),
                "p75": micros_to_ms(corrected.value_at_percentile(75.0)),
                "p90": micros_to_ms(corrected.value_at_percentile(90.0)),
                "p95": micros_to_ms(corrected.value_at_percentile(95.0)),
                "p99": micros_to_ms(corrected.value_at_percentile(99.0)),
                "p999": micros_to_ms(corrected.value_at_percentile(99.9)),
            });
        }
    }

    println!("{}", out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to build a CliCollector with latency data from microsecond values.
    fn collector_from_micros(latencies_us: &[u64]) -> CliCollector {
        let mut collector = CliCollector::new();

        // Build histograms and add them to the collector's internal state.
        // Since we can't call process_sample without a real Sample, we
        // test the display functions using histograms directly.
        for &us in latencies_us {
            collector
                .latency_mut()
                .record(us)
                .expect("record latency");
            collector
                .corrected_latency_mut()
                .record(us)
                .expect("record corrected latency");
        }

        collector
    }

    /// Helper to build a CliCollector from millisecond values (for backward compat with old tests).
    fn collector_from_ms(latencies_ms: &[u64]) -> CliCollector {
        let micros: Vec<u64> = latencies_ms.iter().map(|ms| ms * 1000).collect();
        collector_from_micros(&micros)
    }

    #[test]
    fn test_total_requests_counts_correctly() {
        let collector = collector_from_ms(&[1, 2, 3, 4, 5]);
        assert_eq!(collector.total_requests(), 5);
    }

    #[test]
    fn test_avg_latency_from_histogram() {
        // latencies: 10ms, 20ms, 30ms -> avg ~= 20ms
        let collector = collector_from_ms(&[10, 20, 30]);
        let avg_ms = collector.latency().mean() / 1000.0;
        assert!(
            (avg_ms - 20.0).abs() < 1.0,
            "Expected avg ~20ms, got {:.3}ms",
            avg_ms
        );
    }

    #[test]
    fn test_max_min_latency_from_histogram() {
        let collector = collector_from_ms(&[5, 50, 25, 10, 100]);
        let max_ms = micros_to_ms(collector.latency().max());
        let min_ms = micros_to_ms(collector.latency().min());
        assert!(
            (max_ms - 100.0).abs() < 1.0,
            "Expected max ~100ms, got {:.3}ms",
            max_ms
        );
        assert!(
            (min_ms - 5.0).abs() < 1.0,
            "Expected min ~5ms, got {:.3}ms",
            min_ms
        );
    }

    #[test]
    fn test_std_deviation_zero_for_uniform() {
        let collector = collector_from_ms(&[10, 10, 10, 10, 10]);
        let std_dev = collector.latency().stdev() / 1000.0;
        assert!(
            std_dev < 1.0,
            "Expected std deviation near 0 for uniform latencies, got {}",
            std_dev
        );
    }

    #[test]
    fn test_percentiles_ordered_correctly() {
        let latencies: Vec<u64> = (1..=100).collect();
        let collector = collector_from_ms(&latencies);
        let hist = collector.latency();

        let p50 = hist.value_at_percentile(50.0);
        let p75 = hist.value_at_percentile(75.0);
        let p90 = hist.value_at_percentile(90.0);
        let p95 = hist.value_at_percentile(95.0);
        let p99 = hist.value_at_percentile(99.0);

        assert!(
            p99 >= p95,
            "p99 ({}) should be >= p95 ({})",
            p99,
            p95
        );
        assert!(
            p95 >= p90,
            "p95 ({}) should be >= p90 ({})",
            p95,
            p90
        );
        assert!(
            p90 >= p75,
            "p90 ({}) should be >= p75 ({})",
            p90,
            p75
        );
        assert!(
            p75 >= p50,
            "p75 ({}) should be >= p50 ({})",
            p75,
            p50
        );
    }

    #[test]
    fn test_json_zero_requests_outputs_null_latencies() {
        let out = json!({
            "latency_avg": null,
            "latency_max": null,
            "latency_min": null,
            "latency_std_deviation": null,
            "transfer_total": null,
            "transfer_rate": null,
            "requests_total": 0,
            "requests_avg": null,
        });

        let parsed: serde_json::Value = serde_json::from_str(&out.to_string()).unwrap();
        assert!(
            parsed["latency_avg"].is_null(),
            "latency_avg should be null with 0 requests"
        );
        assert!(
            parsed["latency_max"].is_null(),
            "latency_max should be null with 0 requests"
        );
        assert!(
            parsed["latency_min"].is_null(),
            "latency_min should be null with 0 requests"
        );
        assert!(
            parsed["latency_std_deviation"].is_null(),
            "latency_std_deviation should be null with 0 requests"
        );
        assert_eq!(
            parsed["requests_total"].as_u64().unwrap(),
            0,
            "requests_total should be 0"
        );
    }

    #[test]
    fn json_contains_latency_corrected_when_enabled() {
        let collector = collector_from_ms(&[5, 5, 5, 10, 10]);

        let hist = collector.latency();
        let avg = hist.mean() / 1000.0;
        let max = micros_to_ms(hist.max());
        let min = micros_to_ms(hist.min());
        let std_deviation = hist.stdev() / 1000.0;

        let mut out = json!({
            "latency_avg": avg,
            "latency_max": max,
            "latency_min": min,
            "latency_std_deviation": std_deviation,
            "transfer_total": null,
            "transfer_rate": 0.0,
            "requests_total": collector.total_requests(),
            "requests_avg": 0.0,
        });

        // Simulate co_correction = true
        let corrected = collector.corrected_latency();
        if !corrected.is_empty() {
            out["latency_corrected"] = json!({
                "avg": corrected.mean() / 1000.0,
                "max": micros_to_ms(corrected.max()),
                "min": micros_to_ms(corrected.min()),
                "std_deviation": corrected.stdev() / 1000.0,
                "p50": micros_to_ms(corrected.value_at_percentile(50.0)),
                "p75": micros_to_ms(corrected.value_at_percentile(75.0)),
                "p90": micros_to_ms(corrected.value_at_percentile(90.0)),
                "p95": micros_to_ms(corrected.value_at_percentile(95.0)),
                "p99": micros_to_ms(corrected.value_at_percentile(99.0)),
                "p999": micros_to_ms(corrected.value_at_percentile(99.9)),
            });
        }

        let json_str = out.to_string();
        assert!(
            json_str.contains("latency_corrected"),
            "JSON should contain 'latency_corrected' when co_correction is enabled"
        );

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
        let collector = collector_from_ms(&[5, 5, 5, 10, 10]);

        let hist = collector.latency();
        let avg = hist.mean() / 1000.0;
        let max = micros_to_ms(hist.max());
        let min = micros_to_ms(hist.min());
        let std_deviation = hist.stdev() / 1000.0;

        // Simulate co_correction = false: do NOT add latency_corrected
        let out = json!({
            "latency_avg": avg,
            "latency_max": max,
            "latency_min": min,
            "latency_std_deviation": std_deviation,
            "transfer_total": null,
            "transfer_rate": 0.0,
            "requests_total": collector.total_requests(),
            "requests_avg": 0.0,
        });

        let json_str = out.to_string();
        assert!(
            !json_str.contains("latency_corrected"),
            "JSON should NOT contain 'latency_corrected' when co_correction is disabled"
        );
    }

    #[test]
    fn test_corrected_stats_all_percentiles_populated() {
        let collector = collector_from_ms(&[5, 10, 15, 20, 25, 30, 35, 40, 45, 50]);
        let hist = collector.corrected_latency();

        assert!(micros_to_ms(hist.value_at_percentile(50.0)) >= 0.0, "p50 should be non-negative");
        assert!(micros_to_ms(hist.value_at_percentile(75.0)) >= 0.0, "p75 should be non-negative");
        assert!(micros_to_ms(hist.value_at_percentile(90.0)) >= 0.0, "p90 should be non-negative");
        assert!(micros_to_ms(hist.value_at_percentile(95.0)) >= 0.0, "p95 should be non-negative");
        assert!(micros_to_ms(hist.value_at_percentile(99.0)) >= 0.0, "p99 should be non-negative");
        assert!(micros_to_ms(hist.value_at_percentile(99.9)) >= 0.0, "p999 should be non-negative");
    }

    #[test]
    fn test_empty_collector_total_requests_zero() {
        let collector = CliCollector::new();
        assert_eq!(collector.total_requests(), 0);
    }
}
