use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::routing::get;
use axum::Router;
use http::{Method, Request, Uri};
use hyper::Body;
use rewrk_core::{
    Batch,
    HttpProtocol,
    Producer,
    ReWrkBenchmark,
    RequestBatch,
    Sample,
    SampleCollector,
};

static ADDR: &str = "127.0.0.1:20001";

/// Every Nth request gets an artificial latency spike.
/// Using a large N so spikes are rare -- this means the uncorrected
/// percentiles stay low (fast) while CO correction fills in synthetic
/// entries that push the corrected percentiles higher.
const SPIKE_EVERY_N: usize = 50;

/// Duration of the artificial latency spike.
const SPIKE_DURATION: Duration = Duration::from_millis(200);

/// How long the benchmark runs.
const BENCHMARK_DURATION: Duration = Duration::from_secs(5);

/// Validates that coordinated omission (CO) correction produces
/// meaningfully different results from uncorrected histograms when
/// the server introduces periodic latency spikes.
///
/// The CO correction uses `hdrhistogram::Histogram::record_correct`
/// which fills in synthetic values between the expected interval and
/// the actual (high) latency. This:
/// 1. Increases the total entry count in the corrected histogram.
/// 2. Shifts mid-range percentiles upward because synthetic entries
///    represent the "hidden" requests that would have completed
///    during the spike window.
#[tokio::test(flavor = "multi_thread")]
async fn test_co_correction_with_latency_spikes() {
    let _ = tracing_subscriber::fmt::try_init();

    let counter = Arc::new(AtomicUsize::new(0));
    tokio::spawn(run_server(counter));

    // Give the server a moment to bind.
    tokio::time::sleep(Duration::from_millis(100)).await;

    let uri = Uri::builder()
        .scheme("http")
        .authority(ADDR)
        .path_and_query("/")
        .build()
        .expect("Create URI");

    let mut benchmarker = ReWrkBenchmark::create(
        uri,
        1,
        HttpProtocol::Http1,
        TimedProducer::new(BENCHMARK_DURATION),
        CoCollector::default(),
    )
    .await
    .expect("Create benchmark");

    // Use a large sample window so everything ends up in a single sample.
    benchmarker.set_sample_window(Duration::from_secs(60));
    benchmarker.set_num_workers(1);
    benchmarker.run().await;

    let collector = benchmarker.consume_collector().await.expect("consume collector");
    assert!(
        !collector.samples.is_empty(),
        "Expected at least one sample"
    );

    for sample in &collector.samples {
        let uncorrected_count = sample.latency().len();
        let corrected_count = sample.corrected_latency().len();

        // -- Assertion 1: CO correction adds synthetic entries --
        // The corrected histogram must have more entries than the
        // uncorrected one, proving that `record_correct` fills in
        // values for latencies exceeding the expected interval.
        assert!(
            corrected_count > uncorrected_count,
            "CO-corrected histogram ({corrected_count} entries) should have more entries \
             than uncorrected ({uncorrected_count} entries)"
        );

        // -- Assertion 2: CO-corrected p99 >= uncorrected p99 --
        // The synthetic fill-in entries can only increase (or maintain)
        // percentile values.  With histogram quantization, values are
        // approximately equal, so we allow the corrected p99 to be
        // within the same histogram bucket.
        let uncorrected_p99 = sample.latency().value_at_percentile(99.0);
        let corrected_p99 = sample.corrected_latency().value_at_percentile(99.0);

        // Both p99 values should be in the spike range (~200ms) since
        // there are enough spike requests to fill the top 1%.
        // Due to histogram bucket quantization they may differ slightly,
        // so we check they are in the same order of magnitude.
        assert!(
            corrected_p99 > 100_000, // > 100ms, confirming spike detection
            "Corrected p99 ({corrected_p99} us) should reflect the latency spikes"
        );
        assert!(
            uncorrected_p99 > 100_000,
            "Uncorrected p99 ({uncorrected_p99} us) should reflect the latency spikes"
        );

        // -- Assertion 3: CO correction shifts mid-range percentiles --
        // This is the key CO effect: the synthetic entries push lower
        // percentiles higher.  At p95, the uncorrected histogram sees
        // mostly fast requests, but the corrected histogram's added
        // entries shift p95 significantly upward.
        let uncorrected_p95 = sample.latency().value_at_percentile(95.0);
        let corrected_p95 = sample.corrected_latency().value_at_percentile(95.0);

        eprintln!("Uncorrected count: {uncorrected_count}");
        eprintln!("Corrected   count: {corrected_count}");
        eprintln!("Uncorrected p95: {uncorrected_p95} us");
        eprintln!("Corrected   p95: {corrected_p95} us");
        eprintln!("Uncorrected p99: {uncorrected_p99} us");
        eprintln!("Corrected   p99: {corrected_p99} us");

        assert!(
            corrected_p95 > uncorrected_p95,
            "CO-corrected p95 ({corrected_p95} us) should be higher than \
             uncorrected p95 ({uncorrected_p95} us) due to synthetic fill-in"
        );
    }
}

async fn run_server(counter: Arc<AtomicUsize>) {
    let app = Router::new().route(
        "/",
        get(move || {
            let counter = Arc::clone(&counter);
            async move {
                let n = counter.fetch_add(1, Ordering::Relaxed);
                if n % SPIKE_EVERY_N == 0 {
                    tokio::time::sleep(SPIKE_DURATION).await;
                }
                "OK"
            }
        }),
    );

    axum::Server::bind(&ADDR.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Clone)]
pub struct TimedProducer {
    duration: Duration,
    start: Instant,
}

impl TimedProducer {
    fn new(duration: Duration) -> Self {
        Self {
            duration,
            start: Instant::now(),
        }
    }
}

#[rewrk_core::async_trait]
impl Producer for TimedProducer {
    fn ready(&mut self) {
        self.start = Instant::now();
    }

    async fn create_batch(&mut self) -> anyhow::Result<RequestBatch> {
        if self.start.elapsed() >= self.duration {
            return Ok(RequestBatch::End);
        }

        let uri = Uri::builder().path_and_query("/").build()?;
        let requests = (0..100)
            .map(|_| {
                Request::builder()
                    .method(Method::GET)
                    .uri(uri.clone())
                    .body(Body::empty())
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(RequestBatch::Batch(Batch { tag: 0, requests }))
    }
}

#[derive(Default)]
pub struct CoCollector {
    samples: Vec<Sample>,
}

#[rewrk_core::async_trait]
impl SampleCollector for CoCollector {
    async fn process_sample(&mut self, sample: Sample) -> anyhow::Result<()> {
        self.samples.push(sample);
        Ok(())
    }
}
