//! Protocol-agnostic connection abstraction traits.
//!
//! This module defines the [`ProtocolConnector`] and [`ProtocolConnection`] traits,
//! which provide a uniform interface for establishing connections and executing HTTP
//! requests across different protocol implementations (HTTP/1.1, HTTP/2, HTTP/3).
//!
//! # Design Goals
//!
//! - **Zero-cost abstraction**: The trait uses generics and associated types so that
//!   the compiler can monomorphize implementations, avoiding dynamic dispatch overhead
//!   in the hot path.
//! - **Protocol independence**: The benchmarking engine operates on these traits rather
//!   than concrete types, allowing new protocols (e.g., HTTP/3 via QUIC) to be added
//!   without modifying the engine.
//! - **IO tracking**: Every connection exposes an [`IoUsageTracker`] for measuring
//!   bytes read and written, enabling accurate transfer rate calculations.
//!
//! # Implementing a Custom Connector
//!
//! ```rust
//! use async_trait::async_trait;
//! use http::response::Parts;
//! use http::Request;
//! use hyper::body::Bytes;
//! use hyper::Body;
//! use rewrk_core::utils::IoUsageTracker;
//! use rewrk_core::connection::{ProtocolConnector, ProtocolConnection};
//!
//! #[derive(Clone)]
//! struct MyConnector {
//!     target: String,
//! }
//!
//! struct MyConnection {
//!     io_tracker: IoUsageTracker,
//! }
//!
//! #[async_trait]
//! impl ProtocolConnector for MyConnector {
//!     type Connection = MyConnection;
//!
//!     async fn connect(&self) -> anyhow::Result<Self::Connection> {
//!         // Establish your protocol-specific connection here.
//!         Ok(MyConnection {
//!             io_tracker: IoUsageTracker::new(),
//!         })
//!     }
//! }
//!
//! #[async_trait]
//! impl ProtocolConnection for MyConnection {
//!     async fn execute_req(
//!         &mut self,
//!         request: Request<Body>,
//!     ) -> Result<(Parts, Bytes), hyper::Error> {
//!         // Execute the request using your protocol.
//!         // This is a placeholder — a real implementation would send the request
//!         // over the wire and parse the response.
//!         unimplemented!("replace with real implementation")
//!     }
//!
//!     fn usage(&self) -> &IoUsageTracker {
//!         &self.io_tracker
//!     }
//! }
//! ```

use async_trait::async_trait;
use http::response::Parts;
use http::Request;
use hyper::body::Bytes;
use hyper::Body;

use crate::utils::IoUsageTracker;

/// A protocol-agnostic connector that can establish connections and execute HTTP requests.
///
/// Implementations handle the protocol-specific details (TCP+TLS+HTTP handshake for
/// HTTP/1 & HTTP/2, QUIC+TLS for HTTP/3) while providing a uniform interface to the
/// benchmarking engine.
///
/// # Requirements
///
/// - Must be `Send + Sync + Clone` so it can be shared across worker threads.
/// - The associated [`Connection`](ProtocolConnector::Connection) type must implement
///   [`ProtocolConnection`] and be `Send` so it can be used within async tasks.
///
/// # Zero-Cost Abstraction
///
/// This trait is designed for monomorphization: the benchmarking engine is generic over
/// `C: ProtocolConnector`, so each concrete connector compiles to direct function calls
/// with no dynamic dispatch overhead in the request execution hot path.
#[async_trait]
pub trait ProtocolConnector: Send + Sync + Clone {
    /// The established connection type produced by this connector.
    type Connection: ProtocolConnection + Send;

    /// Establish a new connection to the target server.
    ///
    /// This performs the full connection setup including any required handshakes
    /// (TCP, TLS, HTTP upgrade, QUIC, etc.).
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established (e.g., DNS resolution
    /// failure, TLS handshake failure, connection refused).
    async fn connect(&self) -> anyhow::Result<Self::Connection>;
}

/// An established protocol connection that can execute HTTP requests and track IO usage.
///
/// Each connection represents a single, established link to the target server. The
/// connection handles request serialization, sending, receiving, and response parsing
/// according to its protocol.
///
/// # IO Tracking
///
/// Every connection must expose an [`IoUsageTracker`] via the [`usage`](ProtocolConnection::usage)
/// method. The benchmarking engine reads byte counts before and after each request to
/// compute per-request transfer metrics.
#[async_trait]
pub trait ProtocolConnection: Send {
    /// Execute an HTTP request and return the response head and body.
    ///
    /// The implementation should send the request over the established connection and
    /// return the parsed response parts (status, headers) and the response body bytes.
    ///
    /// # Errors
    ///
    /// Returns a [`hyper::Error`] if the request fails at the protocol level (e.g.,
    /// connection reset, invalid response, timeout).
    async fn execute_req(
        &mut self,
        request: Request<Body>,
    ) -> Result<(Parts, Bytes), hyper::Error>;

    /// Get the IO usage tracker for this connection.
    ///
    /// The tracker records the total bytes read from and written to the underlying
    /// transport. The benchmarking engine uses this to compute per-request transfer
    /// sizes by taking snapshots before and after each request.
    fn usage(&self) -> &IoUsageTracker;
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Mock implementations for testing trait contracts --

    #[derive(Clone)]
    struct MockConnector;

    struct MockConnection {
        io_tracker: IoUsageTracker,
        request_count: usize,
    }

    #[async_trait]
    impl ProtocolConnector for MockConnector {
        type Connection = MockConnection;

        async fn connect(&self) -> anyhow::Result<Self::Connection> {
            Ok(MockConnection {
                io_tracker: IoUsageTracker::new(),
                request_count: 0,
            })
        }
    }

    #[async_trait]
    impl ProtocolConnection for MockConnection {
        async fn execute_req(
            &mut self,
            _request: Request<Body>,
        ) -> Result<(Parts, Bytes), hyper::Error> {
            self.request_count += 1;

            let response = http::Response::builder()
                .status(200)
                .body(Body::empty())
                .unwrap();

            let (parts, body) = response.into_parts();
            let body_bytes = hyper::body::to_bytes(body).await?;
            Ok((parts, body_bytes))
        }

        fn usage(&self) -> &IoUsageTracker {
            &self.io_tracker
        }
    }

    #[tokio::test]
    async fn mock_connector_produces_connection() {
        let connector = MockConnector;
        let connection = connector.connect().await;
        assert!(connection.is_ok(), "connect() should succeed for mock");
    }

    #[tokio::test]
    async fn mock_connection_executes_request() {
        let connector = MockConnector;
        let mut conn = connector.connect().await.unwrap();

        let request = Request::builder()
            .method(http::Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let result = conn.execute_req(request).await;
        assert!(result.is_ok(), "execute_req() should succeed for mock");

        let (parts, _body) = result.unwrap();
        assert_eq!(parts.status, http::StatusCode::OK);
    }

    #[tokio::test]
    async fn mock_connection_tracks_io_usage() {
        let connector = MockConnector;
        let conn = connector.connect().await.unwrap();

        let tracker = conn.usage();
        assert_eq!(tracker.get_received_count(), 0);
        assert_eq!(tracker.get_written_count(), 0);
    }

    #[tokio::test]
    async fn mock_connection_tracks_request_count() {
        let connector = MockConnector;
        let mut conn = connector.connect().await.unwrap();

        for _ in 0..3 {
            let request = Request::builder()
                .uri("/")
                .body(Body::empty())
                .unwrap();
            let _ = conn.execute_req(request).await.unwrap();
        }

        assert_eq!(conn.request_count, 3);
    }

    /// Verify that the trait bounds allow the connector to be sent across threads
    /// and the connection to be used within async tasks.
    #[tokio::test]
    async fn traits_are_send_and_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        fn assert_clone<T: Clone>() {}

        assert_send::<MockConnector>();
        assert_sync::<MockConnector>();
        assert_clone::<MockConnector>();
        assert_send::<MockConnection>();
    }

    /// Verify that a connector can be used from a spawned task,
    /// proving it satisfies the 'static + Send bounds required by tokio::spawn.
    #[tokio::test]
    async fn connector_works_across_spawn_boundary() {
        let connector = MockConnector;

        let handle = tokio::spawn(async move {
            let mut conn = connector.connect().await.unwrap();
            let request = Request::builder()
                .uri("/spawn-test")
                .body(Body::empty())
                .unwrap();
            conn.execute_req(request).await.unwrap()
        });

        let (parts, _body) = handle.await.unwrap();
        assert_eq!(parts.status, http::StatusCode::OK);
    }
}
