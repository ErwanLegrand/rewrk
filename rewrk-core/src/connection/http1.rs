//! HTTP/1.1 protocol connector implementation.
//!
//! Provides [`Http1Connector`] and [`Http1Connection`] as concrete implementations
//! of [`ProtocolConnector`] and [`ProtocolConnection`] for HTTP/1.1 benchmarking.

use std::net::SocketAddr;

use async_trait::async_trait;
use http::response::Parts;
use http::uri::{Authority, Scheme as HttpScheme};
use http::{header, HeaderValue, Request, Uri};
use hyper::body::Bytes;
use hyper::client::conn;
use hyper::Body;
use tokio::net::TcpStream;

use super::conn::{handshake, HttpStream};
use super::protocol::{ProtocolConnection, ProtocolConnector};
use super::Scheme;
use crate::utils::IoUsageTracker;

/// An HTTP/1.1 connector that establishes plain TCP or TLS connections
/// and performs the HTTP/1 handshake.
///
/// This connector does **not** set `http2_only` on the connection builder,
/// so the resulting connection always speaks HTTP/1.1.
#[derive(Clone)]
pub struct Http1Connector {
    addr: SocketAddr,
    scheme: Scheme,
    host: String,
    uri_scheme: HttpScheme,
    uri_authority: Authority,
    host_header: HeaderValue,
}

impl Http1Connector {
    /// Create a new HTTP/1.1 connector.
    ///
    /// # Arguments
    ///
    /// * `addr` - The resolved socket address to connect to.
    /// * `scheme` - Whether to use plain HTTP or HTTPS (with TLS connector).
    /// * `host` - The hostname used for TLS SNI.
    /// * `uri` - The base URI (scheme + authority) for requests. Must have both scheme and authority.
    /// * `host_header` - The value to set in the `Host` header on each request.
    ///
    /// # Panics
    ///
    /// Panics if `uri` is missing a scheme or authority.
    pub fn new(
        addr: SocketAddr,
        scheme: Scheme,
        host: impl Into<String>,
        uri: Uri,
        host_header: HeaderValue,
    ) -> Self {
        let uri_scheme = uri.scheme().expect("URI must have a scheme").clone();
        let uri_authority = uri.authority().expect("URI must have an authority").clone();
        Self {
            addr,
            scheme,
            host: host.into(),
            uri_scheme,
            uri_authority,
            host_header,
        }
    }
}

#[async_trait]
impl ProtocolConnector for Http1Connector {
    type Connection = Http1Connection;

    async fn connect(&self) -> anyhow::Result<Self::Connection> {
        let conn_builder = conn::Builder::new();

        let stream = TcpStream::connect(self.addr).await?;

        let usage_tracker = IoUsageTracker::new();
        let stream = usage_tracker.wrap_stream(stream);

        let stream = match self.scheme {
            Scheme::Http => handshake(conn_builder, stream).await?,
            Scheme::Https(ref tls_connector) => {
                let stream = tls_connector.connect(&self.host, stream).await?;
                handshake(conn_builder, stream).await?
            },
        };

        Ok(Http1Connection {
            uri_scheme: self.uri_scheme.clone(),
            uri_authority: self.uri_authority.clone(),
            host_header: self.host_header.clone(),
            stream,
            io_tracker: usage_tracker,
        })
    }
}

/// An established HTTP/1.1 connection that can execute requests.
pub struct Http1Connection {
    uri_scheme: HttpScheme,
    uri_authority: Authority,
    host_header: HeaderValue,
    stream: HttpStream,
    io_tracker: IoUsageTracker,
}

#[async_trait]
impl ProtocolConnection for Http1Connection {
    #[inline]
    async fn execute_req(
        &mut self,
        mut request: Request<Body>,
    ) -> Result<(Parts, Bytes), hyper::Error> {
        let request_uri = request.uri();
        let mut builder = Uri::builder()
            .scheme(self.uri_scheme.clone())
            .authority(self.uri_authority.clone());
        if let Some(path) = request_uri.path_and_query() {
            builder = builder.path_and_query(path.clone());
        }
        *request.uri_mut() = builder.build().expect("pre-validated URI components");
        request
            .headers_mut()
            .insert(header::HOST, self.host_header.clone());

        let resp = self.stream.send(request).await?;
        let (head, body) = resp.into_parts();
        let body = hyper::body::to_bytes(body).await?;
        Ok((head, body))
    }

    #[inline]
    fn usage(&self) -> &IoUsageTracker {
        &self.io_tracker
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use axum::Router;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    /// Spin up a lightweight axum HTTP/1.1 server and return its address.
    async fn start_test_server() -> SocketAddr {
        let app = Router::new()
            .route("/", get(|| async { "Hello, Http1!" }))
            .route("/echo", get(|| async { "echo-response" }));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::Server::from_tcp(listener.into_std().unwrap())
                .unwrap()
                .serve(app.into_make_service())
                .await
                .unwrap();
        });

        addr
    }

    fn build_connector(addr: SocketAddr) -> Http1Connector {
        let uri = Uri::builder()
            .scheme("http")
            .authority(format!("127.0.0.1:{}", addr.port()))
            .path_and_query("/")
            .build()
            .unwrap();

        let host_header =
            HeaderValue::from_str(&format!("127.0.0.1:{}", addr.port())).unwrap();

        Http1Connector::new(
            addr,
            Scheme::Http,
            "127.0.0.1",
            uri,
            host_header,
        )
    }

    #[tokio::test]
    async fn connect_establishes_http1_connection() {
        let addr = start_test_server().await;
        let connector = build_connector(addr);

        let result = connector.connect().await;
        assert!(result.is_ok(), "Http1Connector::connect() should succeed");
    }

    #[tokio::test]
    async fn execute_get_returns_200_with_body() {
        let addr = start_test_server().await;
        let connector = build_connector(addr);
        let mut conn = connector.connect().await.unwrap();

        let request = Request::builder()
            .method(http::Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let (parts, body) = conn.execute_req(request).await.unwrap();

        assert_eq!(parts.status, http::StatusCode::OK);
        assert_eq!(body.as_ref(), b"Hello, Http1!");
    }

    #[tokio::test]
    async fn execute_get_different_path() {
        let addr = start_test_server().await;
        let connector = build_connector(addr);
        let mut conn = connector.connect().await.unwrap();

        let request = Request::builder()
            .method(http::Method::GET)
            .uri("/echo")
            .body(Body::empty())
            .unwrap();

        let (parts, body) = conn.execute_req(request).await.unwrap();

        assert_eq!(parts.status, http::StatusCode::OK);
        assert_eq!(body.as_ref(), b"echo-response");
    }

    #[tokio::test]
    async fn io_tracker_records_bytes_transferred() {
        let addr = start_test_server().await;
        let connector = build_connector(addr);
        let mut conn = connector.connect().await.unwrap();

        let request = Request::builder()
            .method(http::Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let _ = conn.execute_req(request).await.unwrap();

        let tracker = conn.usage();
        assert!(
            tracker.get_written_count() > 0,
            "Should have written bytes (request)"
        );
        assert!(
            tracker.get_received_count() > 0,
            "Should have received bytes (response)"
        );
    }

    #[tokio::test]
    async fn multiple_requests_on_same_connection() {
        let addr = start_test_server().await;
        let connector = build_connector(addr);
        let mut conn = connector.connect().await.unwrap();

        for i in 0..3 {
            let request = Request::builder()
                .method(http::Method::GET)
                .uri("/")
                .body(Body::empty())
                .unwrap();

            let (parts, body) = conn
                .execute_req(request)
                .await
                .unwrap_or_else(|e| panic!("Request {} failed: {}", i, e));

            assert_eq!(parts.status, http::StatusCode::OK);
            assert_eq!(body.as_ref(), b"Hello, Http1!");
        }

        // IO counts should accumulate across requests.
        let tracker = conn.usage();
        assert!(tracker.get_written_count() > 100, "Multiple requests should write significant bytes");
        assert!(tracker.get_received_count() > 100, "Multiple requests should receive significant bytes");
    }

    #[tokio::test]
    async fn connector_is_clone_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        fn assert_clone<T: Clone>() {}

        assert_send::<Http1Connector>();
        assert_sync::<Http1Connector>();
        assert_clone::<Http1Connector>();
        assert_send::<Http1Connection>();
    }

    #[tokio::test]
    async fn connector_works_across_spawn_boundary() {
        let addr = start_test_server().await;
        let connector = build_connector(addr);

        let handle = tokio::spawn(async move {
            let mut conn = connector.connect().await.unwrap();
            let request = Request::builder()
                .method(http::Method::GET)
                .uri("/")
                .body(Body::empty())
                .unwrap();
            conn.execute_req(request).await.unwrap()
        });

        let (parts, body) = handle.await.unwrap();
        assert_eq!(parts.status, http::StatusCode::OK);
        assert_eq!(body.as_ref(), b"Hello, Http1!");
    }

    /// A relative URI (no scheme, no authority) must panic with the scheme message,
    /// since scheme is checked before authority.
    #[test]
    #[should_panic(expected = "URI must have a scheme")]
    fn new_panics_when_uri_missing_scheme() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let uri: Uri = "/path".parse().unwrap();
        let host_header = HeaderValue::from_static("127.0.0.1:8080");
        Http1Connector::new(addr, Scheme::Http, "127.0.0.1", uri, host_header);
    }

    /// A URI with only an authority and no scheme must panic with the scheme message.
    /// The http crate does not allow constructing a valid Uri with Some(scheme) but None
    /// authority through safe APIs (from_parts rejects scheme-without-authority), so the
    /// authority check is a secondary defensive guard. This test verifies the combined
    /// validation rejects URIs that lack scheme+authority.
    #[test]
    #[should_panic(expected = "URI must have a scheme")]
    fn new_panics_when_uri_missing_scheme_and_authority() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        // Authority-only reference — no scheme, no path.
        let uri: Uri = "//127.0.0.1:8080".parse().unwrap();
        let host_header = HeaderValue::from_static("127.0.0.1:8080");
        Http1Connector::new(addr, Scheme::Http, "127.0.0.1", uri, host_header);
    }
}
