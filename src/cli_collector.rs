use hdrhistogram::Histogram;
use rewrk_core::{Sample, SampleCollector};

/// Aggregates benchmark samples into combined histograms for CLI display.
///
/// Each call to `process_sample` merges the sample's histograms into
/// running totals. After the benchmark, the aggregated histograms can
/// be used to compute and display statistics.
pub struct CliCollector {
    /// Aggregated uncorrected latency histogram.
    latency: Histogram<u32>,
    /// Aggregated CO-corrected latency histogram.
    corrected_latency: Histogram<u32>,
    /// Aggregated write transfer rate histogram.
    write_transfer: Histogram<u32>,
    /// Aggregated read transfer rate histogram.
    read_transfer: Histogram<u32>,
    /// Number of samples processed.
    sample_count: usize,
}

impl CliCollector {
    pub fn new() -> Self {
        Self {
            latency: Histogram::new(2).expect("create histogram"),
            corrected_latency: Histogram::new(2).expect("create histogram"),
            write_transfer: Histogram::new(2).expect("create histogram"),
            read_transfer: Histogram::new(2).expect("create histogram"),
            sample_count: 0,
        }
    }

    /// The aggregated uncorrected latency histogram.
    pub fn latency(&self) -> &Histogram<u32> {
        &self.latency
    }

    /// The aggregated CO-corrected latency histogram.
    pub fn corrected_latency(&self) -> &Histogram<u32> {
        &self.corrected_latency
    }

    /// The aggregated write transfer rate histogram.
    pub fn write_transfer(&self) -> &Histogram<u32> {
        &self.write_transfer
    }

    /// The aggregated read transfer rate histogram.
    pub fn read_transfer(&self) -> &Histogram<u32> {
        &self.read_transfer
    }

    /// Total number of requests recorded (latency histogram count).
    pub fn total_requests(&self) -> u64 {
        self.latency.len()
    }

    /// Number of samples processed.
    pub fn sample_count(&self) -> usize {
        self.sample_count
    }

    /// Mutable access to the uncorrected latency histogram (for testing).
    #[cfg(test)]
    pub fn latency_mut(&mut self) -> &mut Histogram<u32> {
        &mut self.latency
    }

    /// Mutable access to the CO-corrected latency histogram (for testing).
    #[cfg(test)]
    pub fn corrected_latency_mut(&mut self) -> &mut Histogram<u32> {
        &mut self.corrected_latency
    }
}

impl Default for CliCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[rewrk_core::async_trait]
impl SampleCollector for CliCollector {
    async fn process_sample(&mut self, sample: Sample) -> anyhow::Result<()> {
        self.latency.add(sample.latency())?;
        self.corrected_latency.add(sample.corrected_latency())?;
        self.write_transfer.add(sample.write_transfer())?;
        self.read_transfer.add(sample.read_transfer())?;
        self.sample_count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_collector_empty() {
        let collector = CliCollector::new();
        assert_eq!(collector.total_requests(), 0);
        assert_eq!(collector.sample_count(), 0);
        assert_eq!(collector.latency().len(), 0);
        assert_eq!(collector.corrected_latency().len(), 0);
        assert_eq!(collector.write_transfer().len(), 0);
        assert_eq!(collector.read_transfer().len(), 0);
    }

    #[test]
    fn test_histogram_add_merges_counts() {
        let mut h1: Histogram<u32> = Histogram::new(2).unwrap();
        let mut h2: Histogram<u32> = Histogram::new(2).unwrap();

        h1.record(100).unwrap();
        h1.record(200).unwrap();

        h2.record(300).unwrap();

        h1.add(&h2).unwrap();

        assert_eq!(h1.len(), 3);
        assert!(h1.min() <= 100);
        assert!(h1.max() >= 300);
    }

    #[test]
    fn test_collector_accessors() {
        let collector = CliCollector::new();

        // Accessors return references to empty histograms.
        assert_eq!(collector.latency().len(), 0);
        assert_eq!(collector.corrected_latency().len(), 0);
        assert_eq!(collector.write_transfer().len(), 0);
        assert_eq!(collector.read_transfer().len(), 0);
    }

    #[test]
    fn test_default_is_equivalent_to_new() {
        let via_new = CliCollector::new();
        let via_default = CliCollector::default();

        assert_eq!(via_new.total_requests(), via_default.total_requests());
        assert_eq!(via_new.sample_count(), via_default.sample_count());
    }
}
