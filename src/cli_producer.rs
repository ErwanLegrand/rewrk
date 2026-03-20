use std::time::{Duration, Instant};

use http::{HeaderMap, Method, Request, Uri};
use hyper::Body;
use hyper::body::Bytes;
use rewrk_core::{Batch, Producer, RequestBatch};

/// A producer that generates HTTP request batches for the CLI benchmark.
///
/// Produces batches until the configured duration elapses, then returns `End`.
/// Each batch contains `BATCH_SIZE` identical requests built from the CLI settings.
#[derive(Clone)]
pub struct CliProducer {
    /// The request URI path (e.g., "/").
    uri: Uri,
    /// The HTTP method (GET, POST, etc.).
    method: Method,
    /// Additional request headers.
    headers: HeaderMap,
    /// Request body bytes.
    body: Bytes,
    /// How long to produce requests.
    duration: Duration,
    /// Set when `ready()` is called (benchmark start time).
    start: Option<Instant>,
}

const BATCH_SIZE: usize = 500;

impl CliProducer {
    pub fn new(
        uri: Uri,
        method: Method,
        headers: HeaderMap,
        body: Bytes,
        duration: Duration,
    ) -> Self {
        Self {
            uri,
            method,
            headers,
            body,
            duration,
            start: None,
        }
    }
}

#[rewrk_core::async_trait]
impl Producer for CliProducer {
    fn ready(&mut self) {
        self.start = Some(Instant::now());
    }

    async fn create_batch(&mut self) -> anyhow::Result<RequestBatch> {
        let start = self.start.ok_or_else(|| anyhow::anyhow!("ready() must be called before create_batch()"))?;
        if start.elapsed() >= self.duration {
            return Ok(RequestBatch::End);
        }

        let requests = (0..BATCH_SIZE)
            .map(|_| {
                let mut request = Request::builder()
                    .method(self.method.clone())
                    .uri(self.uri.clone())
                    .body(Body::from(self.body.clone()))?;
                *request.headers_mut() = self.headers.clone();
                Ok(request)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(RequestBatch::Batch(Batch { tag: 0, requests }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_producer(duration: Duration) -> CliProducer {
        CliProducer::new(
            Uri::from_static("/"),
            Method::GET,
            HeaderMap::new(),
            Bytes::new(),
            duration,
        )
    }

    #[test]
    fn test_cli_producer_new_creates_instance() {
        let _producer = default_producer(Duration::from_secs(10));
    }

    #[tokio::test]
    async fn test_cli_producer_ends_after_duration() {
        let mut producer = default_producer(Duration::from_secs(0));
        producer.ready();
        let batch = producer.create_batch().await.expect("create_batch failed");
        assert!(
            matches!(batch, RequestBatch::End),
            "Expected End for zero duration"
        );
    }

    #[tokio::test]
    async fn test_cli_producer_produces_batches_before_duration() {
        let mut producer = default_producer(Duration::from_secs(10));
        producer.ready();
        let batch = producer.create_batch().await.expect("create_batch failed");
        match batch {
            RequestBatch::Batch(b) => assert_eq!(b.requests.len(), BATCH_SIZE),
            RequestBatch::End => panic!("Expected Batch, got End"),
        }
    }

    #[tokio::test]
    async fn test_cli_producer_batch_has_correct_method() {
        let mut producer = CliProducer::new(
            Uri::from_static("/"),
            Method::POST,
            HeaderMap::new(),
            Bytes::new(),
            Duration::from_secs(10),
        );
        producer.ready();
        let batch = producer.create_batch().await.expect("create_batch failed");
        match batch {
            RequestBatch::Batch(b) => {
                for request in &b.requests {
                    assert_eq!(request.method(), Method::POST);
                }
            },
            RequestBatch::End => panic!("Expected Batch, got End"),
        }
    }

    #[tokio::test]
    async fn test_cli_producer_batch_has_correct_body() {
        let body_bytes = Bytes::from_static(b"hello world");
        let mut producer = CliProducer::new(
            Uri::from_static("/"),
            Method::POST,
            HeaderMap::new(),
            body_bytes.clone(),
            Duration::from_secs(10),
        );
        producer.ready();
        let batch = producer.create_batch().await.expect("create_batch failed");
        match batch {
            RequestBatch::Batch(b) => {
                for request in b.requests {
                    let collected = hyper::body::to_bytes(request.into_body())
                        .await
                        .expect("failed to read body");
                    assert_eq!(collected, body_bytes);
                }
            },
            RequestBatch::End => panic!("Expected Batch, got End"),
        }
    }

    #[test]
    fn test_cli_producer_is_clone() {
        let producer = default_producer(Duration::from_secs(10));
        let _cloned = producer.clone();
    }
}
