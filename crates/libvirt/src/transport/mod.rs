//! Transport layer for libvirt RPC communication.
//!
//! This module provides different transport implementations:
//! - Unix socket (default for local connections)
//! - TCP (for remote connections)
//! - TLS (for secure remote connections)

mod unix;

pub use unix::UnixTransport;

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};

use crate::error::Result;

/// Trait for transport implementations.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send data to the remote.
    async fn send(&mut self, data: &[u8]) -> Result<()>;

    /// Receive a complete packet from the remote.
    ///
    /// This reads the length prefix and then reads the complete packet.
    async fn recv(&mut self) -> Result<Bytes>;

    /// Close the transport.
    async fn close(&mut self) -> Result<()>;
}

/// Read a complete framed message.
///
/// The libvirt protocol uses a 4-byte big-endian length prefix.
/// The length value includes the 4 bytes of the length field itself.
async fn read_framed<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
    buf: &mut BytesMut,
) -> Result<Bytes> {
    use tokio::io::AsyncReadExt;

    // Read length prefix (4 bytes)
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let total_len = u32::from_be_bytes(len_buf) as usize;

    if total_len > crate::packet::MAX_PACKET_SIZE {
        return Err(crate::error::Error::PacketTooLarge(total_len));
    }

    // The length includes the 4-byte length field, so body is len - 4
    let body_len = total_len.saturating_sub(4);
    if body_len == 0 {
        return Ok(Bytes::new());
    }

    // Read the packet body (header + payload)
    buf.resize(body_len, 0);
    reader.read_exact(buf).await?;

    Ok(Bytes::copy_from_slice(buf))
}

/// Write a framed message.
///
/// The data should already include the length prefix.
async fn write_framed<W: tokio::io::AsyncWrite + Unpin>(writer: &mut W, data: &[u8]) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    writer.write_all(data).await?;
    writer.flush().await?;

    Ok(())
}
