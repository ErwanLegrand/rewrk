use std::collections::HashMap;

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

// ---------------------------------------------------------------------------
// Helper: spawn an axum server on a random OS-assigned port and return the
// bound socket address so each test can construct its own URI.
// ---------------------------------------------------------------------------
async fn spawn_server() -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind random port");
    let addr = listener.local_addr().expect("local addr");

    // Convert the tokio TcpListener into a std listener for axum::Server.
    let std_listener = listener.into_std().expect("into std listener");
    std_listener
        .set_nonblocking(false)
        .expect("set blocking for axum");

    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    tokio::spawn(async move {
        axum::Server::from_tcp(std_listener)
            .expect("from tcp")
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    addr
}

// ---------------------------------------------------------------------------
// Test 1: 2 workers, 4 connections, 10 requests total
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn test_multi_worker_benchmark() {
    let _ = tracing_subscriber::fmt::try_init();

    let addr = spawn_server().await;

    let uri = Uri::builder()
        .scheme("http")
        .authority(addr.to_string().as_str())
        .path_and_query("/")
        .build()
        .expect("Create URI");

    let mut benchmarker = ReWrkBenchmark::create(
        uri,
        4, // concurrency (connections)
        HttpProtocol::HTTP1,
        CountedProducer::new(10),
        BasicCollector::default(),
    )
    .await
    .expect("Create benchmark");

    benchmarker.set_num_workers(2);
    benchmarker.run().await;

    let collector = benchmarker.consume_collector().await.expect("consume collector");

    assert!(
        !collector.samples.is_empty(),
        "Expected at least one sample from multi-worker benchmark"
    );

    let total_latency_recordings: u64 = collector
        .samples
        .iter()
        .map(|s| s.latency().len())
        .sum();

    assert!(
        total_latency_recordings >= 10,
        "Expected at least 10 latency recordings, got {total_latency_recordings}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: 1 worker, 4 connections, 8 requests
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn test_multi_connection_per_worker() {
    let _ = tracing_subscriber::fmt::try_init();

    let addr = spawn_server().await;

    let uri = Uri::builder()
        .scheme("http")
        .authority(addr.to_string().as_str())
        .path_and_query("/")
        .build()
        .expect("Create URI");

    let mut benchmarker = ReWrkBenchmark::create(
        uri,
        4, // concurrency (connections)
        HttpProtocol::HTTP1,
        CountedProducer::new(8),
        BasicCollector::default(),
    )
    .await
    .expect("Create benchmark");

    benchmarker.set_num_workers(1);
    benchmarker.run().await;

    let collector = benchmarker.consume_collector().await.expect("consume collector");

    assert!(
        !collector.samples.is_empty(),
        "Expected at least one sample"
    );

    let total_latency_recordings: u64 = collector
        .samples
        .iter()
        .map(|s| s.latency().len())
        .sum();

    assert!(
        total_latency_recordings > 0,
        "Expected some latency recordings, got {total_latency_recordings}"
    );
}

// ---------------------------------------------------------------------------
// Test 3: Custom producer (3 batches, tag 0/1/2, 2 requests each) and
//         collector that groups samples by tag.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn test_custom_producer_and_collector() {
    let _ = tracing_subscriber::fmt::try_init();

    let addr = spawn_server().await;

    let uri = Uri::builder()
        .scheme("http")
        .authority(addr.to_string().as_str())
        .path_and_query("/")
        .build()
        .expect("Create URI");

    let mut benchmarker = ReWrkBenchmark::create(
        uri,
        1,
        HttpProtocol::HTTP1,
        TaggedProducer::default(),
        TagGroupingCollector::default(),
    )
    .await
    .expect("Create benchmark");

    benchmarker.set_num_workers(1);
    benchmarker.run().await;

    let collector = benchmarker.consume_collector().await.expect("consume collector");

    assert!(
        collector.samples_by_tag.len() > 1,
        "Expected multiple tags in collector, got {} tag(s): {:?}",
        collector.samples_by_tag.len(),
        collector.samples_by_tag.keys().collect::<Vec<_>>(),
    );
}

// ---------------------------------------------------------------------------
// Shared producer: emits exactly `total` single-request batches (tag = 0)
// then signals End. Each batch contains one GET request.
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct CountedProducer {
    total: usize,
    remaining: usize,
}

impl CountedProducer {
    fn new(total: usize) -> Self {
        Self {
            total,
            remaining: total,
        }
    }
}

#[rewrk_core::async_trait]
impl Producer for CountedProducer {
    fn ready(&mut self) {
        self.remaining = self.total;
    }

    async fn create_batch(&mut self) -> anyhow::Result<RequestBatch> {
        if self.remaining == 0 {
            return Ok(RequestBatch::End);
        }
        self.remaining -= 1;

        let uri = Uri::builder().path_and_query("/").build()?;
        let request = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Body::empty())?;

        Ok(RequestBatch::Batch(Batch {
            tag: 0,
            requests: vec![request],
        }))
    }
}

// ---------------------------------------------------------------------------
// Tagged producer: emits 3 batches with tags 0, 1, 2; each batch has 2 reqs.
// ---------------------------------------------------------------------------

#[derive(Default, Clone)]
pub struct TaggedProducer {
    next_tag: usize,
}

#[rewrk_core::async_trait]
impl Producer for TaggedProducer {
    fn ready(&mut self) {
        self.next_tag = 0;
    }

    async fn create_batch(&mut self) -> anyhow::Result<RequestBatch> {
        if self.next_tag > 2 {
            return Ok(RequestBatch::End);
        }

        let tag: usize = self.next_tag;
        self.next_tag += 1;

        let uri = Uri::builder().path_and_query("/").build()?;
        let requests = (0..2)
            .map(|_| {
                Request::builder()
                    .method(Method::GET)
                    .uri(uri.clone())
                    .body(Body::empty())
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(RequestBatch::Batch(Batch { tag, requests }))
    }
}

// ---------------------------------------------------------------------------
// Basic collector: accumulates all samples in a Vec.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct BasicCollector {
    samples: Vec<Sample>,
}

#[rewrk_core::async_trait]
impl SampleCollector for BasicCollector {
    async fn process_sample(&mut self, sample: Sample) -> anyhow::Result<()> {
        self.samples.push(sample);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tag-grouping collector: groups samples by their tag value.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct TagGroupingCollector {
    samples_by_tag: HashMap<usize, Vec<Sample>>,
}

#[rewrk_core::async_trait]
impl SampleCollector for TagGroupingCollector {
    async fn process_sample(&mut self, sample: Sample) -> anyhow::Result<()> {
        self.samples_by_tag
            .entry(sample.tag())
            .or_default()
            .push(sample);
        Ok(())
    }
}
