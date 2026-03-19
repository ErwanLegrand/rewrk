//! Enum-dispatch connector and connection for ReWrk benchmarking.
//!
//! [`ReWrkConnector`] and [`ReWrkConnection`] are enums that delegate to the
//! appropriate protocol-specific implementation ([`Http1Connector`] /
//! [`Http2Connector`] and their connection counterparts) based on the chosen
//! [`HttpProtocol`].
//!
//! ## Why enum dispatch?
//!
//! The benchmarking hot path is generic over `C: ProtocolConnector`, so
//! monomorphisation already eliminates virtual-dispatch overhead. Using an
//! enum keeps a single concrete type that the rest of the codebase can name
//! while still routing to the correct implementation at runtime.

use std::future::Future;

use async_trait::async_trait;
use http::response::Parts;
use http::Request;
use hyper::body::Bytes;
use hyper::client::conn;
use hyper::client::conn::SendRequest;
use hyper::Body;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::task::JoinHandle;

use super::http1::{Http1Connection, Http1Connector};
use super::http2::{Http2Connection, Http2Connector};
use super::protocol::{ProtocolConnection, ProtocolConnector};
use crate::utils::IoUsageTracker;

/// Enum-dispatch connector.
///
/// Created via [`ReWrkConnector::new`] which picks the right variant based on
/// the [`HttpProtocol`] argument.
#[derive(Clone)]
pub enum ReWrkConnector {
    /// HTTP/1.1 variant.
    Http1(Http1Connector),
    /// HTTP/2 variant.
    Http2(Http2Connector),
}

impl ReWrkConnector {
    /// Wrap an [`Http1Connector`] in the enum dispatcher.
    pub fn http1(connector: Http1Connector) -> Self {
        Self::Http1(connector)
    }

    /// Wrap an [`Http2Connector`] in the enum dispatcher.
    pub fn http2(connector: Http2Connector) -> Self {
        Self::Http2(connector)
    }
}

#[async_trait]
impl ProtocolConnector for ReWrkConnector {
    type Connection = ReWrkConnection;

    async fn connect(&self) -> anyhow::Result<Self::Connection> {
        match self {
            Self::Http1(c) => c.connect().await.map(ReWrkConnection::Http1),
            Self::Http2(c) => c.connect().await.map(ReWrkConnection::Http2),
        }
    }
}

/// Enum-dispatch connection.
///
/// Produced by [`ReWrkConnector::connect`].
pub enum ReWrkConnection {
    /// HTTP/1.1 connection.
    Http1(Http1Connection),
    /// HTTP/2 connection.
    Http2(Http2Connection),
}

#[async_trait]
impl ProtocolConnection for ReWrkConnection {
    #[inline]
    async fn execute_req(
        &mut self,
        request: Request<Body>,
    ) -> Result<(Parts, Bytes), hyper::Error> {
        match self {
            Self::Http1(c) => c.execute_req(request).await,
            Self::Http2(c) => c.execute_req(request).await,
        }
    }

    #[inline]
    fn usage(&self) -> &IoUsageTracker {
        match self {
            Self::Http1(c) => c.usage(),
            Self::Http2(c) => c.usage(),
        }
    }
}

/// Performs the HTTP handshake over any async read/write stream.
///
/// This is shared by [`Http1Connector`] and [`Http2Connector`].
pub(crate) async fn handshake<S>(
    conn_builder: conn::Builder,
    stream: S,
) -> Result<HttpStream, hyper::Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (send_request, connection) = conn_builder.handshake(stream).await?;
    let connection_task = tokio::spawn(connection);
    Ok(HttpStream {
        conn: send_request,
        waiter: connection_task,
    })
}

/// The established HTTP stream (wraps `hyper`'s `SendRequest`).
pub(crate) struct HttpStream {
    /// The live connection to send requests on.
    conn: SendRequest<Body>,
    /// The hyper connection driver task.
    waiter: JoinHandle<hyper::Result<()>>,
}

impl HttpStream {
    pub fn send(
        &mut self,
        request: Request<Body>,
    ) -> impl Future<Output = Result<hyper::Response<Body>, hyper::Error>> {
        self.conn.send_request(request)
    }
}

impl Drop for HttpStream {
    fn drop(&mut self) {
        self.waiter.abort();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use axum::routing::get;
    use axum::Router;
    use http::{HeaderValue, Uri};
    use hyper::Body;
    use tokio::net::TcpListener;

    use super::*;
    use crate::connection::{Http1Connector, Http2Connector, Scheme};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    async fn start_http1_server() -> SocketAddr {
        let app = Router::new()
            .route("/", get(|| async { "Hello from Http1!" }))
            .route("/ping", get(|| async { "pong1" }));

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

    async fn start_http2_server() -> SocketAddr {
        let app = Router::new()
            .route("/", get(|| async { "Hello from Http2!" }))
            .route("/ping", get(|| async { "pong2" }));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::Server::from_tcp(listener.into_std().unwrap())
                .unwrap()
                .http2_only(true)
                .serve(app.into_make_service())
                .await
                .unwrap();
        });

        addr
    }

    fn build_http1_connector(addr: SocketAddr) -> ReWrkConnector {
        let uri = Uri::builder()
            .scheme("http")
            .authority(format!("127.0.0.1:{}", addr.port()))
            .path_and_query("/")
            .build()
            .unwrap();
        let host_header =
            HeaderValue::from_str(&format!("127.0.0.1:{}", addr.port())).unwrap();

        ReWrkConnector::http1(Http1Connector::new(
            addr,
            Scheme::Http,
            "127.0.0.1",
            uri,
            host_header,
        ))
    }

    fn build_http2_connector(addr: SocketAddr) -> ReWrkConnector {
        let uri = Uri::builder()
            .scheme("http")
            .authority(format!("127.0.0.1:{}", addr.port()))
            .path_and_query("/")
            .build()
            .unwrap();
        let host_header =
            HeaderValue::from_str(&format!("127.0.0.1:{}", addr.port())).unwrap();

        ReWrkConnector::http2(Http2Connector::new(
            addr,
            Scheme::Http,
            "127.0.0.1",
            uri,
            host_header,
        ))
    }

    // -----------------------------------------------------------------------
    // Variant construction
    // -----------------------------------------------------------------------

    #[test]
    fn http1_variant_is_http1() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let uri = Uri::builder()
            .scheme("http")
            .authority("127.0.0.1:80")
            .path_and_query("/")
            .build()
            .unwrap();
        let hh = HeaderValue::from_static("127.0.0.1");
        let c = ReWrkConnector::http1(Http1Connector::new(addr, Scheme::Http, "127.0.0.1", uri, hh));
        assert!(matches!(c, ReWrkConnector::Http1(_)));
    }

    #[test]
    fn http2_variant_is_http2() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let uri = Uri::builder()
            .scheme("http")
            .authority("127.0.0.1:80")
            .path_and_query("/")
            .build()
            .unwrap();
        let hh = HeaderValue::from_static("127.0.0.1");
        let c = ReWrkConnector::http2(Http2Connector::new(addr, Scheme::Http, "127.0.0.1", uri, hh));
        assert!(matches!(c, ReWrkConnector::Http2(_)));
    }

    // -----------------------------------------------------------------------
    // HTTP/1.1 dispatch
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn http1_connector_connects_and_gets_200() {
        let addr = start_http1_server().await;
        let connector = build_http1_connector(addr);
        let mut conn = connector.connect().await.unwrap();

        let req = http::Request::builder()
            .method(http::Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let (parts, body) = conn.execute_req(req).await.unwrap();
        assert_eq!(parts.status, http::StatusCode::OK);
        assert_eq!(body.as_ref(), b"Hello from Http1!");
    }

    #[tokio::test]
    async fn http1_connection_is_http1_variant() {
        let addr = start_http1_server().await;
        let connector = build_http1_connector(addr);
        let conn = connector.connect().await.unwrap();
        assert!(matches!(conn, ReWrkConnection::Http1(_)));
    }

    #[tokio::test]
    async fn http1_io_tracker_records_bytes() {
        let addr = start_http1_server().await;
        let connector = build_http1_connector(addr);
        let mut conn = connector.connect().await.unwrap();

        let req = http::Request::builder()
            .method(http::Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();
        let _ = conn.execute_req(req).await.unwrap();

        assert!(conn.usage().get_written_count() > 0);
        assert!(conn.usage().get_received_count() > 0);
    }

    // -----------------------------------------------------------------------
    // HTTP/2 dispatch
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn http2_connector_connects_and_gets_200() {
        let addr = start_http2_server().await;
        let connector = build_http2_connector(addr);
        let mut conn = connector.connect().await.unwrap();

        let req = http::Request::builder()
            .method(http::Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let (parts, body) = conn.execute_req(req).await.unwrap();
        assert_eq!(parts.status, http::StatusCode::OK);
        assert_eq!(body.as_ref(), b"Hello from Http2!");
    }

    #[tokio::test]
    async fn http2_connection_is_http2_variant() {
        let addr = start_http2_server().await;
        let connector = build_http2_connector(addr);
        let conn = connector.connect().await.unwrap();
        assert!(matches!(conn, ReWrkConnection::Http2(_)));
    }

    #[tokio::test]
    async fn http2_io_tracker_records_bytes() {
        let addr = start_http2_server().await;
        let connector = build_http2_connector(addr);
        let mut conn = connector.connect().await.unwrap();

        let req = http::Request::builder()
            .method(http::Method::GET)
            .uri("/")
            .body(Body::empty())
            .unwrap();
        let _ = conn.execute_req(req).await.unwrap();

        assert!(conn.usage().get_written_count() > 0);
        assert!(conn.usage().get_received_count() > 0);
    }

    // -----------------------------------------------------------------------
    // Clone / Send / Sync
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn rewrk_connector_is_clone_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        fn assert_clone<T: Clone>() {}

        assert_send::<ReWrkConnector>();
        assert_sync::<ReWrkConnector>();
        assert_clone::<ReWrkConnector>();
        assert_send::<ReWrkConnection>();
    }
}
