use axum::response::IntoResponse;
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

static ADDR: &str = "127.0.0.1:19997";

/// Handler that always sends `Connection: close`, forcing the client to
/// reconnect after every response.
async fn connection_close_handler() -> impl IntoResponse {
    (
        [(http::header::CONNECTION, "close")],
        "Hello with close",
    )
}

async fn run_server() {
    let app = Router::new().route("/", get(connection_close_handler));

    axum::Server::bind(&ADDR.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Default, Clone)]
pub struct ReconnectProducer {
    remaining: usize,
}

#[rewrk_core::async_trait]
impl Producer for ReconnectProducer {
    fn ready(&mut self) {
        // Send 5 requests, each in its own batch.
        // Every request will get `Connection: close` back,
        // so the worker must reconnect between batches.
        self.remaining = 5;
    }

    async fn create_batch(&mut self) -> anyhow::Result<RequestBatch> {
        if self.remaining > 0 {
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
        } else {
            Ok(RequestBatch::End)
        }
    }
}

#[derive(Default)]
pub struct ReconnectCollector {
    samples: Vec<Sample>,
}

#[rewrk_core::async_trait]
impl SampleCollector for ReconnectCollector {
    async fn process_sample(&mut self, sample: Sample) -> anyhow::Result<()> {
        self.samples.push(sample);
        Ok(())
    }
}

#[tokio::test]
async fn test_reconnect_on_connection_close() {
    let _ = tracing_subscriber::fmt::try_init();

    tokio::spawn(run_server());

    // Give the server a moment to bind.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let uri = Uri::builder()
        .scheme("http")
        .authority(ADDR)
        .path_and_query("/")
        .build()
        .expect("Create URI");

    let mut benchmarker = ReWrkBenchmark::create(
        uri,
        1,
        HttpProtocol::HTTP1,
        ReconnectProducer::default(),
        ReconnectCollector::default(),
    )
    .await
    .expect("Create benchmark");
    benchmarker.set_num_workers(1);
    benchmarker.run().await;

    let collector = benchmarker.consume_collector().await;

    // Gather all latency recordings across every sample.
    let total_latencies: u64 = collector.samples.iter().map(|s| s.latency().len()).sum();

    // We sent 5 requests, each forcing a reconnect. With the reconnection
    // logic the worker should successfully complete all of them.
    // Without reconnection logic this would be 0 or 1.
    assert!(
        total_latencies >= 2,
        "Expected at least 2 successful latency recordings across reconnects, got {total_latencies}"
    );
}
