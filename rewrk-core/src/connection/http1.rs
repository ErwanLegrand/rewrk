//! HTTP/1.1 protocol connector implementation.
//!
//! Provides [`Http1Connector`] and [`Http1Connection`] as concrete implementations
//! of [`ProtocolConnector`] and [`ProtocolConnection`] for HTTP/1.1 benchmarking.
//!
//! This module is a thin wrapper around the generic [`BaseConnector`] /
//! [`BaseConnection`] types, parameterized with [`Http1Config`] which leaves the
//! hyper connection builder at its default settings (HTTP/1.1).

use hyper::client::conn;

use super::base::{BaseConnector, BaseConnection, BuilderConfig};

/// HTTP/1.1 builder configuration (no-op — uses hyper defaults).
#[derive(Clone)]
pub struct Http1Config;

impl BuilderConfig for Http1Config {
    fn configure(_builder: &mut conn::Builder) {
        // HTTP/1.1 uses default builder settings.
    }
}

/// An HTTP/1.1 connector that establishes plain TCP or TLS connections
/// and performs the HTTP/1 handshake.
///
/// This connector does **not** set `http2_only` on the connection builder,
/// so the resulting connection always speaks HTTP/1.1.
pub type Http1Connector = BaseConnector<Http1Config>;

/// An established HTTP/1.1 connection that can execute requests.
pub type Http1Connection = BaseConnection;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use axum::Router;
    use http::{HeaderValue, Request, Uri};
    use hyper::Body;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    use crate::connection::{protocol::ProtocolConnection, protocol::ProtocolConnector, Scheme};

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
