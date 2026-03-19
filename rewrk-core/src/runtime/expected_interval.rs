/// Tracks the expected inter-request interval per connection using a
/// cumulative moving average of observed successful request latencies.
///
/// The expected interval represents how often requests "should" be sent
/// and is used for Coordinated Omission correction.
#[derive(Debug, Clone)]
pub(crate) struct ExpectedIntervalTracker {
    /// The cumulative sum of all observed latencies in microseconds.
    total_latency_us: u64,
    /// The number of successful requests observed.
    request_count: u64,
}

impl ExpectedIntervalTracker {
    /// Create a new tracker with no observations.
    pub fn new() -> Self {
        Self {
            total_latency_us: 0,
            request_count: 0,
        }
    }

    /// Record a successful request latency (in microseconds) and return
    /// the updated expected interval.
    #[inline]
    pub fn record(&self, latency_us: u64) -> Self {
        Self {
            total_latency_us: self.total_latency_us + latency_us,
            request_count: self.request_count + 1,
        }
    }

    /// Return the current expected interval in microseconds, or `None`
    /// if no requests have been observed yet.
    #[inline]
    pub fn expected_interval_us(&self) -> Option<u64> {
        self.total_latency_us.checked_div(self.request_count)
    }

    /// Return the number of requests observed so far.
    #[cfg(test)]
    pub fn request_count(&self) -> u64 {
        self.request_count
    }
}

impl Default for ExpectedIntervalTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_has_no_expected_interval() {
        let tracker = ExpectedIntervalTracker::new();
        assert_eq!(tracker.expected_interval_us(), None);
        assert_eq!(tracker.request_count(), 0);
    }

    #[test]
    fn single_request_sets_expected_interval() {
        let tracker = ExpectedIntervalTracker::new();
        let tracker = tracker.record(5000);
        assert_eq!(tracker.expected_interval_us(), Some(5000));
        assert_eq!(tracker.request_count(), 1);
    }

    #[test]
    fn average_of_uniform_latencies() {
        let tracker = ExpectedIntervalTracker::new();
        let tracker = tracker.record(1000);
        let tracker = tracker.record(1000);
        let tracker = tracker.record(1000);
        assert_eq!(tracker.expected_interval_us(), Some(1000));
        assert_eq!(tracker.request_count(), 3);
    }

    #[test]
    fn average_of_varying_latencies() {
        let tracker = ExpectedIntervalTracker::new();
        // Feed latencies: 1000, 2000, 3000 -> average = 2000
        let tracker = tracker.record(1000);
        let tracker = tracker.record(2000);
        let tracker = tracker.record(3000);
        assert_eq!(tracker.expected_interval_us(), Some(2000));
    }

    #[test]
    fn average_converges_with_many_observations() {
        // Start with a high outlier, then feed many 1000us latencies.
        // The average should converge toward 1000.
        let mut tracker = ExpectedIntervalTracker::new();
        tracker = tracker.record(10_000); // outlier

        for _ in 0..99 {
            tracker = tracker.record(1000);
        }

        let expected = tracker.expected_interval_us().unwrap();
        // (10_000 + 99 * 1000) / 100 = 109_000 / 100 = 1090
        assert_eq!(expected, 1090);
        assert_eq!(tracker.request_count(), 100);
    }

    #[test]
    fn record_does_not_mutate_original() {
        let original = ExpectedIntervalTracker::new();
        let _updated = original.record(5000);
        // Original should remain unchanged (immutability).
        assert_eq!(original.expected_interval_us(), None);
        assert_eq!(original.request_count(), 0);
    }

    #[test]
    fn default_is_same_as_new() {
        let tracker = ExpectedIntervalTracker::default();
        assert_eq!(tracker.expected_interval_us(), None);
        assert_eq!(tracker.request_count(), 0);
    }
}
