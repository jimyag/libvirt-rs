//! Error types for the libvirt client.

/// Result type for libvirt operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during libvirt operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// XDR serialization/deserialization error.
    #[error("XDR error: {0}")]
    Xdr(#[from] libvirt_xdr::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Connection error.
    #[error("connection error: {0}")]
    Connection(String),

    /// Unsupported URI scheme.
    #[error("unsupported URI: {0}")]
    UnsupportedUri(String),

    /// Connection closed unexpectedly.
    #[error("connection closed")]
    ConnectionClosed,

    /// RPC error from libvirt daemon.
    #[error("RPC error {code}: {message}")]
    Rpc {
        code: i32,
        domain: i32,
        message: String,
    },

    /// Authentication failed.
    #[error("authentication failed: {0}")]
    AuthFailed(String),

    /// Protocol error.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// Timeout error.
    #[error("operation timed out")]
    Timeout,

    /// Packet too large.
    #[error("packet too large: {0} bytes")]
    PacketTooLarge(usize),

    /// Remote error from libvirt daemon.
    #[error("remote error: {0}")]
    RemoteError(String),

    /// Packet parsing error.
    #[error("packet error: {0}")]
    Packet(#[from] crate::packet::PacketError),
}
