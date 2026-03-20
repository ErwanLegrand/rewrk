//! Shared base types for protocol-specific connectors.
//!
//! Provides [`BaseConnector`] and [`BaseConnection`] — a generic, protocol-agnostic
//! implementation parameterized by a [`BuilderConfig`] that controls how the hyper
//! connection builder is configured before the handshake.
//!
//! HTTP/1.1 uses the default builder; HTTP/2 sets `http2_only(true)`.

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

/// Trait for configuring the hyper connection builder before handshake.
///
/// HTTP/1 uses the default builder; HTTP/2 sets `http2_only(true)`.
pub trait BuilderConfig: Clone + Send + Sync + 'static {
    fn configure(builder: &mut conn::Builder);
}

/// A protocol-agnostic connector parameterized by builder configuration.
#[derive(Clone)]
pub struct BaseConnector<C: BuilderConfig> {
    addr: SocketAddr,
    scheme: Scheme,
    host: String,
    uri_scheme: HttpScheme,
    uri_authority: Authority,
    host_header: HeaderValue,
    _config: std::marker::PhantomData<C>,
}

impl<C: BuilderConfig> BaseConnector<C> {
    /// Create a new connector.
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
            _config: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<C: BuilderConfig> ProtocolConnector for BaseConnector<C> {
    type Connection = BaseConnection;

    async fn connect(&self) -> anyhow::Result<Self::Connection> {
        let mut conn_builder = conn::Builder::new();
        C::configure(&mut conn_builder);

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

        Ok(BaseConnection {
            uri_scheme: self.uri_scheme.clone(),
            uri_authority: self.uri_authority.clone(),
            host_header: self.host_header.clone(),
            stream,
            io_tracker: usage_tracker,
        })
    }
}

/// A protocol-agnostic connection (shared by HTTP/1 and HTTP/2).
pub struct BaseConnection {
    uri_scheme: HttpScheme,
    uri_authority: Authority,
    host_header: HeaderValue,
    stream: HttpStream,
    io_tracker: IoUsageTracker,
}

#[async_trait]
impl ProtocolConnection for BaseConnection {
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
