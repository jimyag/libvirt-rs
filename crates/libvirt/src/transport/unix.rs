//! Unix socket transport implementation.

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use tokio::net::UnixStream;

use super::{read_framed, write_framed, Transport};
use crate::error::Result;

/// Unix socket transport.
pub struct UnixTransport {
    stream: UnixStream,
    read_buf: BytesMut,
}

impl UnixTransport {
    /// Connect to a Unix socket.
    pub async fn connect(path: &str) -> Result<Self> {
        let stream = UnixStream::connect(path).await?;
        Ok(Self {
            stream,
            read_buf: BytesMut::with_capacity(4096),
        })
    }
}

#[async_trait]
impl Transport for UnixTransport {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        write_framed(&mut self.stream, data).await
    }

    async fn recv(&mut self) -> Result<Bytes> {
        read_framed(&mut self.stream, &mut self.read_buf).await
    }

    async fn close(&mut self) -> Result<()> {
        // UnixStream doesn't have an explicit close, it's closed on drop.
        // We can shutdown the write half.
        use tokio::io::AsyncWriteExt;
        self.stream.shutdown().await?;
        Ok(())
    }
}
