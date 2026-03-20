use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[derive(Clone, Default)]
/// A utility for wrapping streams and measuring the number of
/// bytes being passed through the wrapped stream.
pub struct IoUsageTracker {
    received: Arc<AtomicU64>,
    written: Arc<AtomicU64>,
}

impl IoUsageTracker {
    /// Create a new usage tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wrap an existing stream with the usage tracker.
    pub(crate) fn wrap_stream<I>(&self, stream: I) -> RecordStream<I> {
        RecordStream::new(stream, self.clone())
    }

    /// Get the current received usage count.
    pub fn get_received_count(&self) -> u64 {
        self.received.load(Ordering::SeqCst)
    }
    /// Get the current written usage count.
    pub fn get_written_count(&self) -> u64 {
        self.written.load(Ordering::SeqCst)
    }
}

pin_project! {
    pub(crate) struct RecordStream<I> {
        #[pin]
        inner: I,
        usage: IoUsageTracker,
    }
}

impl<I> RecordStream<I> {
    fn new(inner: I, usage: IoUsageTracker) -> Self {
        Self { inner, usage }
    }
}

impl<I: AsyncRead> AsyncRead for RecordStream<I> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.project();
        let before = buf.filled().len();
        let poll_result = this.inner.poll_read(cx, buf);

        let newly_read = buf.filled().len() - before;
        if newly_read > 0 {
            this.usage
                .received
                .fetch_add(newly_read as u64, Ordering::SeqCst);
        }

        poll_result
    }
}

impl<I: AsyncWrite> AsyncWrite for RecordStream<I> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        let poll_result = this.inner.poll_write(cx, buf);

        if let Poll::Ready(Ok(n)) = &poll_result {
            this.usage
                .written
                .fetch_add(*n as u64, Ordering::SeqCst);
        }

        poll_result
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        self.project().inner.poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;

    #[test]
    fn test_new_tracker_zeroed() {
        let tracker = IoUsageTracker::new();
        assert_eq!(tracker.get_received_count(), 0);
        assert_eq!(tracker.get_written_count(), 0);
    }

    #[test]
    fn test_tracker_clone_shares_state() {
        let tracker = IoUsageTracker::new();
        let clone = tracker.clone();

        tracker.received.fetch_add(100, Ordering::SeqCst);
        tracker.written.fetch_add(200, Ordering::SeqCst);

        assert_eq!(clone.get_received_count(), 100);
        assert_eq!(clone.get_written_count(), 200);
    }

    #[tokio::test]
    async fn test_wrap_stream_tracks_writes() {
        let tracker = IoUsageTracker::new();
        let (client, _server) = tokio::io::duplex(1024);
        let mut stream = tracker.wrap_stream(client);

        let data = b"hello world";
        stream.write_all(data).await.unwrap();

        assert_eq!(tracker.get_written_count(), data.len() as u64);
    }

    #[tokio::test]
    async fn test_wrap_stream_tracks_reads() {
        let tracker = IoUsageTracker::new();
        let (mut server, client) = tokio::io::duplex(1024);
        let mut stream = tracker.wrap_stream(client);

        let data = b"hello world";
        server.write_all(data).await.unwrap();
        drop(server);

        let mut buf = vec![0u8; data.len()];
        stream.read_exact(&mut buf).await.unwrap();

        assert_eq!(tracker.get_received_count(), data.len() as u64);
    }

    #[tokio::test]
    async fn test_poll_read_counts_only_newly_read_bytes() {
        // Verifies that pre-existing buffer data is not counted
        let tracker = IoUsageTracker::new();
        let (mut server, client) = tokio::io::duplex(1024);
        let mut stream = tracker.wrap_stream(client);

        let first_chunk = b"hello";
        let second_chunk = b"world";

        server.write_all(first_chunk).await.unwrap();
        server.write_all(second_chunk).await.unwrap();
        drop(server);

        let mut buf = vec![0u8; first_chunk.len() + second_chunk.len()];
        stream.read_exact(&mut buf).await.unwrap();

        // Should count exactly the bytes read, not cumulative filled lengths
        assert_eq!(
            tracker.get_received_count(),
            (first_chunk.len() + second_chunk.len()) as u64
        );
    }

    #[tokio::test]
    async fn test_poll_write_counts_only_actually_written_bytes() {
        // Verifies that only bytes confirmed written (Poll::Ready(Ok(n))) are counted
        let tracker = IoUsageTracker::new();
        let (client, _server) = tokio::io::duplex(1024);
        let mut stream = tracker.wrap_stream(client);

        let data = b"hello world";
        let n = stream.write(data).await.unwrap();

        // Should count actual bytes written (n), not buf.len() unconditionally
        assert_eq!(tracker.get_written_count(), n as u64);
        assert_eq!(tracker.get_received_count(), 0);
    }
}
