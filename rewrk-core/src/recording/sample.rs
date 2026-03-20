use std::fmt::{Debug, Formatter};
use std::time::{Duration, Instant};

use flume::TrySendError;
use hdrhistogram::Histogram;

use crate::recording::collector::CollectorMailbox;
use crate::validator::ValidationError;

#[derive(Debug, Clone, Copy)]
pub struct SampleMetadata {
    /// The unique ID of the worker thread.
    pub worker_id: usize,
}

#[derive(Debug, thiserror::Error)]
#[error("The service should shutdown.")]
/// The service worker has shutdown and should no longer process requests.
pub struct Shutdown;

#[derive(Clone)]
/// A sample factory produces and submits samples.
pub struct SampleFactory {
    /// The duration which should elapse before a sample
    /// is submitted to be processed.
    window_timeout: Duration,

    /// Metadata associated with the specific sample factory thread.
    metadata: SampleMetadata,
    submitter: CollectorMailbox,
}

impl SampleFactory {
    /// Create a new sample factory.
    pub fn new(
        window_timeout: Duration,
        metadata: SampleMetadata,
        submitter: CollectorMailbox,
    ) -> Self {
        Self {
            window_timeout,
            metadata,
            submitter,
        }
    }

    #[inline]
    /// Check if the handler should submit the current sample.
    pub fn should_submit(&self, instant: Instant) -> bool {
        self.window_timeout <= instant.elapsed()
    }

    #[inline]
    /// Create a new sample to record metrics.
    pub fn new_sample(&self, tag: usize) -> Sample {
        Sample {
            tag,
            latency_hist: Histogram::new(2).unwrap(),
            corrected_latency_hist: Histogram::new(2).unwrap(),
            write_transfer_hist: Histogram::new(2).unwrap(),
            read_transfer_hist: Histogram::new(2).unwrap(),
            errors: Vec::with_capacity(4),
            metadata: self.metadata,
        }
    }

    #[inline]
    /// Attempts to submit a sample to the processor.
    pub fn submit_sample(&self, sample: Sample) -> Result<(), Shutdown> {
        debug!(sample = ?sample, "Submitting sample to processor");
        // This should never block as it's an unbounded channel.
        let result = self.submitter.try_send(sample);

        match result {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                panic!("Sample submitter should never be full.")
            },
            Err(TrySendError::Disconnected(_)) => Err(Shutdown),
        }
    }
}

#[derive(Clone)]
/// A collection of metrics taken from the benchmark for a given time window.
///
/// The sample contains the standard metrics (latency, IO, etc...) along with
/// any errors, the worker ID and sample tag which can be used to group results.
///
/// Internally this uses HDR Histograms which can generate the min, max, stdev and
/// varying percentile statistics of the benchmark.
pub struct Sample {
    tag: usize,
    latency_hist: Histogram<u32>,
    corrected_latency_hist: Histogram<u32>,
    write_transfer_hist: Histogram<u32>,
    read_transfer_hist: Histogram<u32>,

    errors: Vec<ValidationError>,
    metadata: SampleMetadata,
}

impl Debug for Sample {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sample")
            .field("num_records", &self.latency().len())
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl Sample {
    /// The sample metadata.
    pub fn metadata(&self) -> SampleMetadata {
        self.metadata
    }

    /// The sample latency histogram
    pub fn latency(&self) -> &Histogram<u32> {
        &self.latency_hist
    }

    /// The CO-corrected latency histogram.
    pub fn corrected_latency(&self) -> &Histogram<u32> {
        &self.corrected_latency_hist
    }

    /// The sample write transfer rate histogram
    pub fn write_transfer(&self) -> &Histogram<u32> {
        &self.write_transfer_hist
    }

    /// The sample read transfer rate histogram
    pub fn read_transfer(&self) -> &Histogram<u32> {
        &self.read_transfer_hist
    }

    #[inline]
    /// The current sample batch tag.
    pub fn tag(&self) -> usize {
        self.tag
    }

    #[inline]
    /// Record a request validation error.
    pub(crate) fn record_error(&mut self, e: ValidationError) {
        self.errors.push(e);
    }

    #[inline]
    /// Record a latency duration.
    ///
    /// This value is converted to micro seconds.
    pub(crate) fn record_latency(&mut self, dur: Duration) {
        let micros = dur.as_micros() as u64;
        if let Err(e) = self.latency_hist.record(micros) {
            warn!(value = micros, error = %e, "Failed to record latency value");
        }
    }

    #[inline]
    /// Record a CO-corrected latency duration.
    ///
    /// Uses `hdrhistogram::Histogram::record_correct` to fill in synthetic
    /// values that compensate for coordinated omission. The
    /// `expected_interval_us` is the expected inter-request interval in
    /// microseconds.
    pub(crate) fn record_latency_corrected(
        &mut self,
        dur: Duration,
        expected_interval_us: u64,
    ) {
        let micros = dur.as_micros() as u64;
        if let Err(e) = self.corrected_latency_hist.record_correct(micros, expected_interval_us) {
            warn!(value = micros, expected_interval = expected_interval_us, error = %e, "Failed to record corrected latency value");
        }
    }

    #[inline]
    /// Record a write transfer rate.
    pub(crate) fn record_write_transfer(
        &mut self,
        start_count: u64,
        end_count: u64,
        dur: Duration,
    ) {
        let rate = calculate_rate(start_count, end_count, dur);
        if let Err(e) = self.write_transfer_hist.record(rate) {
            warn!(value = rate, error = %e, "Failed to record write transfer rate");
        }
    }

    #[inline]
    /// Record a read transfer rate.
    pub(crate) fn record_read_transfer(
        &mut self,
        start_count: u64,
        end_count: u64,
        dur: Duration,
    ) {
        let rate = calculate_rate(start_count, end_count, dur);
        if let Err(e) = self.read_transfer_hist.record(rate) {
            warn!(value = rate, error = %e, "Failed to record read transfer rate");
        }
    }
}

#[inline]
fn calculate_rate(start: u64, stop: u64, dur: Duration) -> u64 {
    let secs = dur.as_secs_f64();
    if secs == 0.0 {
        return 0;
    }
    (stop.saturating_sub(start) as f64 / secs).round() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a blank sample for testing.
    fn test_sample() -> Sample {
        Sample {
            tag: 0,
            latency_hist: Histogram::new(2).unwrap(),
            corrected_latency_hist: Histogram::new(2).unwrap(),
            write_transfer_hist: Histogram::new(2).unwrap(),
            read_transfer_hist: Histogram::new(2).unwrap(),
            errors: Vec::new(),
            metadata: SampleMetadata { worker_id: 0 },
        }
    }

    #[test]
    fn corrected_latency_accessor_returns_corrected_histogram() {
        let mut sample = test_sample();
        let dur = Duration::from_micros(500);
        sample.record_latency_corrected(dur, 500);

        assert_eq!(sample.corrected_latency().len(), 1);
        assert!(sample.corrected_latency().max() >= 500);
    }

    #[test]
    fn record_correct_with_spike_produces_more_entries_than_record() {
        let mut sample = test_sample();

        // Simulate 9 normal requests at 1000us, then one spike at 10_000us.
        let expected_interval_us: u64 = 1000;

        for _ in 0..9 {
            let dur = Duration::from_micros(expected_interval_us);
            sample.record_latency(dur);
            sample.record_latency_corrected(dur, expected_interval_us);
        }

        // Latency spike: 10x the expected interval.
        let spike = Duration::from_micros(10_000);
        sample.record_latency(spike);
        sample.record_latency_corrected(spike, expected_interval_us);

        // The uncorrected histogram should have exactly 10 entries.
        assert_eq!(sample.latency().len(), 10);

        // The corrected histogram should have MORE entries because
        // record_correct fills in synthetic values for the spike.
        assert!(
            sample.corrected_latency().len() > sample.latency().len(),
            "corrected histogram ({}) should have more entries than uncorrected ({})",
            sample.corrected_latency().len(),
            sample.latency().len(),
        );
    }

    #[test]
    fn record_correct_with_spike_shifts_percentiles_higher() {
        let mut sample = test_sample();

        let expected_interval_us: u64 = 1000;

        for _ in 0..9 {
            let dur = Duration::from_micros(expected_interval_us);
            sample.record_latency(dur);
            sample.record_latency_corrected(dur, expected_interval_us);
        }

        let spike = Duration::from_micros(10_000);
        sample.record_latency(spike);
        sample.record_latency_corrected(spike, expected_interval_us);

        // The corrected p99 should be >= the uncorrected p99 because
        // CO correction fills in missed measurements during the spike.
        let uncorrected_p99 = sample.latency().value_at_percentile(99.0);
        let corrected_p99 = sample.corrected_latency().value_at_percentile(99.0);

        assert!(
            corrected_p99 >= uncorrected_p99,
            "corrected p99 ({}) should be >= uncorrected p99 ({})",
            corrected_p99,
            uncorrected_p99,
        );
    }

    #[test]
    fn record_and_record_correct_equivalent_when_value_equals_interval() {
        let mut sample = test_sample();

        // When every value equals the expected interval, record_correct
        // should NOT add any synthetic entries.
        let interval_us: u64 = 1000;

        for _ in 0..100 {
            let dur = Duration::from_micros(interval_us);
            sample.record_latency(dur);
            sample.record_latency_corrected(dur, interval_us);
        }

        assert_eq!(
            sample.latency().len(),
            sample.corrected_latency().len(),
            "when value == expected_interval, both histograms should have the same count"
        );

        // Percentiles should also match.
        assert_eq!(
            sample.latency().value_at_percentile(50.0),
            sample.corrected_latency().value_at_percentile(50.0),
        );
        assert_eq!(
            sample.latency().value_at_percentile(99.0),
            sample.corrected_latency().value_at_percentile(99.0),
        );
    }

    #[test]
    fn test_sample_creation_defaults() {
        let sample = test_sample();
        assert_eq!(sample.tag(), 0);
        assert_eq!(sample.metadata().worker_id, 0);
        assert_eq!(sample.latency().len(), 0);
        assert_eq!(sample.corrected_latency().len(), 0);
        assert_eq!(sample.write_transfer().len(), 0);
        assert_eq!(sample.read_transfer().len(), 0);
        assert!(sample.errors.is_empty());
    }

    #[test]
    fn test_sample_tag_accessor() {
        let sample = Sample {
            tag: 42,
            latency_hist: Histogram::new(2).unwrap(),
            corrected_latency_hist: Histogram::new(2).unwrap(),
            write_transfer_hist: Histogram::new(2).unwrap(),
            read_transfer_hist: Histogram::new(2).unwrap(),
            errors: Vec::new(),
            metadata: SampleMetadata { worker_id: 0 },
        };
        assert_eq!(sample.tag(), 42);
    }

    #[test]
    fn test_record_latency_single() {
        let mut sample = test_sample();
        sample.record_latency(Duration::from_micros(250));
        assert_eq!(sample.latency().len(), 1);
        assert!(sample.latency().max() >= 250);
    }

    #[test]
    fn test_record_latency_multiple() {
        let mut sample = test_sample();
        sample.record_latency(Duration::from_micros(100));
        sample.record_latency(Duration::from_micros(500));
        sample.record_latency(Duration::from_micros(1000));
        assert_eq!(sample.latency().len(), 3);
        assert!(sample.latency().min() <= 100);
        assert!(sample.latency().max() >= 1000);
    }

    #[test]
    fn test_record_write_transfer() {
        let mut sample = test_sample();
        // 1000 bytes over 1 second = 1000 bytes/sec
        sample.record_write_transfer(0, 1000, Duration::from_secs(1));
        assert_eq!(sample.write_transfer().len(), 1);
        assert!(sample.write_transfer().max() > 0);
    }

    #[test]
    fn test_record_read_transfer() {
        let mut sample = test_sample();
        // 2000 bytes over 1 second = 2000 bytes/sec
        sample.record_read_transfer(0, 2000, Duration::from_secs(1));
        assert_eq!(sample.read_transfer().len(), 1);
        assert!(sample.read_transfer().max() > 0);
    }

    #[test]
    fn test_record_error_single() {
        let mut sample = test_sample();
        sample.record_error(ValidationError::ConnectionAborted);
        assert_eq!(sample.errors.len(), 1);
    }

    #[test]
    fn test_record_error_multiple_types() {
        let mut sample = test_sample();
        sample.record_error(ValidationError::ConnectionAborted);
        sample.record_error(ValidationError::Timeout);
        sample.record_error(ValidationError::InvalidStatus(404));
        assert_eq!(sample.errors.len(), 3);
    }

    #[test]
    fn test_record_error_classification() {
        let mut sample = test_sample();
        sample.record_error(ValidationError::InvalidStatus(500));
        sample.record_error(ValidationError::Timeout);

        assert!(matches!(sample.errors[0], ValidationError::InvalidStatus(500)));
        assert!(matches!(sample.errors[1], ValidationError::Timeout));
    }

    #[test]
    fn test_calculate_rate_zero_duration() {
        assert_eq!(calculate_rate(0, 100, Duration::ZERO), 0);
    }

    #[test]
    fn test_calculate_rate_normal() {
        // 100 bytes in 1 second = 100 bytes/sec
        assert_eq!(calculate_rate(0, 100, Duration::from_secs(1)), 100);
    }

    #[test]
    fn test_metadata_accessor() {
        let sample = Sample {
            tag: 0,
            latency_hist: Histogram::new(2).unwrap(),
            corrected_latency_hist: Histogram::new(2).unwrap(),
            write_transfer_hist: Histogram::new(2).unwrap(),
            read_transfer_hist: Histogram::new(2).unwrap(),
            errors: Vec::new(),
            metadata: SampleMetadata { worker_id: 7 },
        };
        assert_eq!(sample.metadata().worker_id, 7);
    }
}
