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

async fn spawn_server() -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind random port");
    let addr = listener.local_addr().expect("local addr");

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

#[tokio::test]
async fn test_basic_benchmark() {
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
        HttpProtocol::Http1,
        TimedProducer::default(),
        BasicCollector::default(),
    )
    .await
    .expect("Create benchmark");
    benchmarker.set_sample_window(Duration::from_secs(30));
    benchmarker.set_num_workers(1);

    let start = Instant::now();
    benchmarker.run().await;
    assert!(start.elapsed() >= Duration::from_secs(10));

    let mut collector = benchmarker.consume_collector().await.expect("consume collector");
    let sample = collector.samples.remove(0);
    assert_eq!(sample.tag(), 0);
    dbg!(
        sample.latency().len(),
        sample.latency().min(),
        sample.latency().max(),
        sample.latency().stdev(),
    );
}

#[derive(Clone)]
pub struct TimedProducer {
    start: Instant,
}

impl Default for TimedProducer {
    fn default() -> Self {
        Self {
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
        if self.start.elapsed() >= Duration::from_secs(10) {
            return Ok(RequestBatch::End);
        }

        let uri = Uri::builder().path_and_query("/").build()?;
        let requests = (0..500)
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
