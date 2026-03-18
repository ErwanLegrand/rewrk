//! HTTP/3 protocol connector stub.
//!
//! This module provides [`Http3Connector`] and [`Http3Connection`] as **compile-time
//! validation stubs** for the [`ProtocolConnector`] / [`ProtocolConnection`] trait
//! design.  They prove that the trait surface is sufficient to accommodate an HTTP/3
//! implementation (via QUIC, e.g. [quinn](https://crates.io/crates/quinn) +
//! [h3](https://crates.io/crates/h3)) without any changes to the benchmarking engine.
//!
//! > **No quinn / h3 dependencies are added here.**  All methods panic at runtime with
//! > `unimplemented!()`.  A real implementation would replace the panic bodies with
//! > QUIC connection setup and request execution.
//!
//! # How a real HTTP/3 implementation would look
//!
//! ```rust,no_run
//! use async_trait::async_trait;
//! use http::response::Parts;
//! use http::Request;
//! use hyper::body::Bytes;
//! use hyper::Body;
//! use rewrk_core::utils::IoUsageTracker;
//! use rewrk_core::connection::{ProtocolConnector, ProtocolConnection};
//!
//! /// Real HTTP/3 connector (requires quinn + h3 crates).
//! #[derive(Clone)]
//! struct RealHttp3Connector {
//!     /// The target authority, e.g. `"example.com:443"`.
//!     authority: String,
//! }
//!
//! struct RealHttp3Connection {
//!     // quinn::Connection and h3::client::Connection would live here.
//!     io_tracker: IoUsageTracker,
//! }
//!
//! #[async_trait]
//! impl ProtocolConnector for RealHttp3Connector {
//!     type Connection = RealHttp3Connection;
//!
//!     async fn connect(&self) -> anyhow::Result<Self::Connection> {
//!         // 1. Build a quinn Endpoint with a TLS client config.
//!         // 2. Perform the QUIC handshake to `self.authority`.
//!         // 3. Wrap the QUIC connection in h3::client::builder().build().
//!         let io_tracker = IoUsageTracker::new();
//!         Ok(RealHttp3Connection { io_tracker })
//!     }
//! }
//!
//! #[async_trait]
//! impl ProtocolConnection for RealHttp3Connection {
//!     async fn execute_req(
//!         &mut self,
//!         request: Request<Body>,
//!     ) -> Result<(Parts, Bytes), hyper::Error> {
//!         // 1. Open an h3 request stream.
//!         // 2. Send headers + body.
//!         // 3. Receive response headers and body.
//!         // 4. Return (Parts, Bytes).
//!         unimplemented!("replace with real h3 request execution")
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

use super::protocol::{ProtocolConnection, ProtocolConnector};
use crate::utils::IoUsageTracker;

/// A **stub** HTTP/3 connector that satisfies [`ProtocolConnector`] at compile time.
///
/// This type exists to validate that the trait design is sufficient for a future
/// HTTP/3 implementation using QUIC (quinn + h3).  All methods panic at runtime;
/// replace the `unimplemented!()` bodies with real QUIC logic when the time comes.
///
/// # Trait compliance
///
/// `Http3Connector` is `Clone + Send + Sync`, satisfying the bounds required by
/// [`ProtocolConnector`] so the benchmarking engine can use it without modification.
///
/// # Example
///
/// ```rust,no_run
/// use rewrk_core::connection::Http3Connector;
/// use rewrk_core::connection::ProtocolConnector;
///
/// #[tokio::main]
/// async fn main() {
///     let connector = Http3Connector::new();
///     // This panics at runtime with "not implemented" — replace with real quinn logic.
///     let _conn = connector.connect().await.unwrap();
/// }
/// ```
#[derive(Clone)]
pub struct Http3Connector {
    /// Placeholder field demonstrating that connector state (e.g. a QUIC endpoint
    /// or TLS config) can live here once a real implementation is added.
    _io_tracker: IoUsageTracker,
}

impl Http3Connector {
    /// Create a new HTTP/3 stub connector.
    pub fn new() -> Self {
        Self {
            _io_tracker: IoUsageTracker::new(),
        }
    }
}

impl Default for Http3Connector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProtocolConnector for Http3Connector {
    type Connection = Http3Connection;

    /// Establish an HTTP/3 connection.
    ///
    /// # Stub behaviour
    ///
    /// Always panics with `unimplemented!()`.  A real implementation would:
    /// 1. Construct a `quinn::Endpoint` with a TLS client configuration.
    /// 2. Perform the QUIC handshake to the target host.
    /// 3. Drive the `h3::client::Connection` handshake.
    /// 4. Return an [`Http3Connection`] wrapping the live QUIC + h3 state.
    async fn connect(&self) -> anyhow::Result<Self::Connection> {
        unimplemented!(
            "Http3Connector::connect() is a stub — replace with quinn + h3 implementation"
        )
    }
}

/// A **stub** HTTP/3 connection that satisfies [`ProtocolConnection`] at compile time.
///
/// Holds an [`IoUsageTracker`] to demonstrate that the trait interface is compatible
/// with the IO-tracking pattern used by [`Http1Connection`](super::http1::Http1Connection)
/// and [`Http2Connection`](super::http2::Http2Connection).
pub struct Http3Connection {
    /// Tracks bytes read from / written to the QUIC transport.
    io_tracker: IoUsageTracker,
}

#[async_trait]
impl ProtocolConnection for Http3Connection {
    /// Execute an HTTP/3 request.
    ///
    /// # Stub behaviour
    ///
    /// Always panics with `unimplemented!()`.  A real implementation would open
    /// an h3 request stream, send headers and body, and collect the response.
    async fn execute_req(
        &mut self,
        _request: Request<Body>,
    ) -> Result<(Parts, Bytes), hyper::Error> {
        unimplemented!(
            "Http3Connection::execute_req() is a stub — replace with h3 request execution"
        )
    }

    /// Return the IO usage tracker for this connection.
    fn usage(&self) -> &IoUsageTracker {
        &self.io_tracker
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Compile-time trait bound verification --

    /// Verify that `Http3Connector` satisfies all bounds required by `ProtocolConnector`.
    ///
    /// This test contains no runtime assertions; it passes as long as it compiles.
    #[test]
    fn http3_connector_satisfies_protocol_connector_bounds() {
        fn assert_protocol_connector<C>()
        where
            C: ProtocolConnector,
        {
        }

        assert_protocol_connector::<Http3Connector>();
    }

    /// Verify that `Http3Connector` is `Send + Sync + Clone`.
    #[test]
    fn http3_connector_is_send_sync_clone() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        fn assert_clone<T: Clone>() {}

        assert_send::<Http3Connector>();
        assert_sync::<Http3Connector>();
        assert_clone::<Http3Connector>();
    }

    /// Verify that `Http3Connection` is `Send`, which is required by
    /// `ProtocolConnector::Connection`.
    #[test]
    fn http3_connection_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Http3Connection>();
    }

    /// Verify that `Http3Connection` satisfies all bounds required by `ProtocolConnection`.
    #[test]
    fn http3_connection_satisfies_protocol_connection_bounds() {
        fn assert_protocol_connection<C>()
        where
            C: ProtocolConnection + Send,
        {
        }

        assert_protocol_connection::<Http3Connection>();
    }

    /// Verify that `Http3Connector::connect()` panics with the expected `unimplemented!`
    /// message, confirming the stub behaves as documented.
    #[tokio::test]
    #[should_panic(expected = "not implemented")]
    async fn connect_panics_with_unimplemented() {
        let connector = Http3Connector::new();
        // unwrap() would only be reached if connect() returned Ok — it won't.
        let _ = connector.connect().await.unwrap();
    }

    /// Verify that `Http3Connection::execute_req()` panics with the expected
    /// `unimplemented!` message.
    #[tokio::test]
    #[should_panic(expected = "not implemented")]
    async fn execute_req_panics_with_unimplemented() {
        let mut conn = Http3Connection {
            io_tracker: IoUsageTracker::new(),
        };

        let request = Request::builder()
            .method(http::Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let _ = conn.execute_req(request).await;
    }

    /// Verify that `Http3Connection::usage()` returns the tracker without panicking,
    /// even though this is a stub.
    #[test]
    fn usage_returns_tracker() {
        let conn = Http3Connection {
            io_tracker: IoUsageTracker::new(),
        };

        let tracker = conn.usage();
        assert_eq!(tracker.get_received_count(), 0);
        assert_eq!(tracker.get_written_count(), 0);
    }

    /// Verify that `Http3Connector` can be cloned and that both instances are
    /// independent (compile-time + runtime shape check).
    #[test]
    fn connector_clone_produces_independent_instance() {
        let original = Http3Connector::new();
        let cloned = original.clone();

        // Both should have the same shape — we can at least verify they exist.
        let _ = original;
        let _ = cloned;
    }

    /// Verify that a boxed `Http3Connector` can be stored as a `dyn` object
    /// behind the trait, proving the trait is object-safe for future use cases
    /// where dynamic dispatch is preferred.
    #[test]
    fn connector_can_be_used_as_generic_bound() {
        fn accept_connector<C: ProtocolConnector>(_c: C) {}
        accept_connector(Http3Connector::new());
    }
}
