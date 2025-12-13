//! libvirt RPC packet encoding/decoding.
//!
//! The libvirt RPC protocol uses a simple packet format:
//!
//! ```plaintext
//! +------------+------------+------------+------------+
//! | length (4) | program(4) | version(4) |procedure(4)|
//! +------------+------------+------------+------------+
//! |  type (4)  | serial (4) | status (4) |   payload  |
//! +------------+------------+------------+------------+
//! ```
//!
//! All multi-byte values are big-endian.

use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::generated::{REMOTE_PROGRAM, REMOTE_PROTOCOL_VERSION};

/// Packet header size in bytes (not including length field).
pub const HEADER_SIZE: usize = 24;

/// Maximum packet size (4 MB).
pub const MAX_PACKET_SIZE: usize = 4 * 1024 * 1024;

/// RPC message type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MessageType {
    /// Request/call message.
    Call = 0,
    /// Reply message.
    Reply = 1,
    /// Async event message.
    Message = 2,
    /// Stream data.
    Stream = 3,
}

impl MessageType {
    fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::Call),
            1 => Some(Self::Reply),
            2 => Some(Self::Message),
            3 => Some(Self::Stream),
            _ => None,
        }
    }
}

/// RPC message status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Status {
    /// Success.
    Ok = 0,
    /// Error.
    Error = 1,
    /// Continue (for streams).
    Continue = 2,
}

impl Status {
    fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::Ok),
            1 => Some(Self::Error),
            2 => Some(Self::Continue),
            _ => None,
        }
    }
}

/// An RPC packet.
#[derive(Debug, Clone)]
pub struct Packet {
    /// Program ID.
    pub program: u32,
    /// Protocol version.
    pub version: u32,
    /// Procedure number (unsigned in protocol).
    pub procedure: u32,
    /// Message type.
    pub msg_type: MessageType,
    /// Request serial number (signed in protocol).
    pub serial: i32,
    /// Response status.
    pub status: Status,
    /// Payload data.
    pub payload: Bytes,
}

impl Packet {
    /// Create a new call packet.
    pub fn new_call(procedure: u32, serial: i32, payload: Bytes) -> Self {
        Self {
            program: REMOTE_PROGRAM as u32,
            version: REMOTE_PROTOCOL_VERSION as u32,
            procedure,
            msg_type: MessageType::Call,
            serial,
            status: Status::Ok,
            payload,
        }
    }

    /// Encode the packet to bytes.
    pub fn encode(&self) -> BytesMut {
        let payload_len = self.payload.len();
        // Length field includes: Len(4) + Header(24) + Payload
        let total_len = 4 + HEADER_SIZE + payload_len;

        let mut buf = BytesMut::with_capacity(total_len);

        // Length (including the length field itself!)
        buf.put_u32(total_len as u32);

        // Header (matches go-libvirt's Header struct)
        buf.put_u32(self.program);
        buf.put_u32(self.version);
        buf.put_u32(self.procedure);  // unsigned
        buf.put_u32(self.msg_type as u32);
        buf.put_i32(self.serial);     // signed
        buf.put_u32(self.status as u32);

        // Payload
        buf.extend_from_slice(&self.payload);

        buf
    }

    /// Decode a packet from bytes.
    ///
    /// The input should NOT include the length prefix.
    pub fn decode(mut data: Bytes) -> Result<Self, PacketError> {
        if data.len() < HEADER_SIZE {
            return Err(PacketError::TooShort);
        }

        let program = data.get_u32();
        let version = data.get_u32();
        let procedure = data.get_u32();  // unsigned
        let msg_type = data.get_u32();
        let serial = data.get_i32();     // signed
        let status = data.get_u32();

        let msg_type =
            MessageType::from_u32(msg_type).ok_or(PacketError::InvalidMessageType(msg_type))?;
        let status = Status::from_u32(status).ok_or(PacketError::InvalidStatus(status))?;

        let payload = data;

        Ok(Self {
            program,
            version,
            procedure,
            msg_type,
            serial,
            status,
            payload,
        })
    }
}

/// Packet parsing/encoding error.
#[derive(Debug, thiserror::Error)]
pub enum PacketError {
    #[error("packet too short")]
    TooShort,
    #[error("invalid message type: {0}")]
    InvalidMessageType(u32),
    #[error("invalid status: {0}")]
    InvalidStatus(u32),
    #[error("packet too large: {0} bytes")]
    TooLarge(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_encode_decode() {
        let payload = Bytes::from_static(b"hello");
        let packet = Packet::new_call(42, 1, payload.clone());

        let encoded = packet.encode();

        // Skip length prefix
        let data = Bytes::copy_from_slice(&encoded[4..]);
        let decoded = Packet::decode(data).unwrap();

        assert_eq!(decoded.program, REMOTE_PROGRAM as u32);
        assert_eq!(decoded.version, REMOTE_PROTOCOL_VERSION as u32);
        assert_eq!(decoded.procedure, 42);
        assert_eq!(decoded.msg_type, MessageType::Call);
        assert_eq!(decoded.serial, 1);
        assert_eq!(decoded.status, Status::Ok);
        assert_eq!(decoded.payload, payload);
    }
}
