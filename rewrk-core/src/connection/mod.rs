use tokio_native_tls::TlsConnector;

mod conn;
mod http1;
mod http2;
mod http3;
mod protocol;

pub use self::conn::{ReWrkConnection, ReWrkConnector};
pub use self::http1::{Http1Connection, Http1Connector};
pub use self::http2::{Http2Connection, Http2Connector};
pub use self::http3::{Http3Connection, Http3Connector};
pub use self::protocol::{ProtocolConnection, ProtocolConnector};

/// The type of bench that is being ran.
#[derive(Clone, Copy, Debug)]
pub enum HttpProtocol {
    /// Sets the http protocol to be used as h1
    HTTP1,

    /// Sets the http protocol to be used as h2
    HTTP2,
}

impl HttpProtocol {
    pub fn is_http1(&self) -> bool {
        matches!(self, Self::HTTP1)
    }

    pub fn is_http2(&self) -> bool {
        matches!(self, Self::HTTP2)
    }
}

#[derive(Clone)]
/// The HTTP scheme used for the connection.
pub enum Scheme {
    Http,
    Https(TlsConnector),
}

impl Scheme {
    pub fn default_port(&self) -> u16 {
        match self {
            Self::Http => 80,
            Self::Https(_) => 443,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_protocol_is_http1() {
        let proto = HttpProtocol::HTTP1;
        assert!(proto.is_http1());
        assert!(!proto.is_http2());
    }

    #[test]
    fn test_http_protocol_is_http2() {
        let proto = HttpProtocol::HTTP2;
        assert!(proto.is_http2());
        assert!(!proto.is_http1());
    }

    #[test]
    fn test_scheme_default_port_http() {
        let scheme = Scheme::Http;
        assert_eq!(scheme.default_port(), 80);
    }
}
