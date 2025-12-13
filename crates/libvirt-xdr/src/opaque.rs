//! Fixed-length opaque serialization helpers.
//!
//! XDR fixed-length opaque data (like UUID) doesn't have a length prefix,
//! just the raw bytes with padding to 4-byte alignment.
//!
//! This module provides `FixedOpaque16` type that correctly handles
//! XDR serialization for 16-byte fixed opaque data (UUID).

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// Wrapper type for 16-byte fixed-length opaque data (UUID).
///
/// In XDR, fixed-length opaque data is serialized as raw bytes without
/// a length prefix, padded to 4-byte alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FixedOpaque16(pub [u8; 16]);

impl FixedOpaque16 {
    /// Create a new FixedOpaque16 from a byte array.
    pub fn new(data: [u8; 16]) -> Self {
        Self(data)
    }

    /// Get the inner byte array.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Format as UUID string (lowercase hex with dashes).
    pub fn to_uuid_string(&self) -> String {
        format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3],
            self.0[4], self.0[5],
            self.0[6], self.0[7],
            self.0[8], self.0[9],
            self.0[10], self.0[11], self.0[12], self.0[13], self.0[14], self.0[15]
        )
    }
}

impl std::fmt::Display for FixedOpaque16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_uuid_string())
    }
}

impl Serialize for FixedOpaque16 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Use newtype_struct to signal we want raw bytes without length prefix
        // The inner type is just the byte slice wrapped in a helper
        serializer.serialize_newtype_struct("FixedOpaque16", &FixedOpaqueBytes(&self.0))
    }
}

/// Helper to serialize raw bytes for fixed opaque.
struct FixedOpaqueBytes<'a>(&'a [u8]);

impl Serialize for FixedOpaqueBytes<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(self.0)
    }
}

impl<'de> Deserialize<'de> for FixedOpaque16 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FixedOpaque16Visitor;

        impl<'de> de::Visitor<'de> for FixedOpaque16Visitor {
            type Value = FixedOpaque16;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("16 bytes of opaque data")
            }

            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                if v.len() >= 16 {
                    let mut arr = [0u8; 16];
                    arr.copy_from_slice(&v[..16]);
                    Ok(FixedOpaque16(arr))
                } else {
                    Err(E::invalid_length(v.len(), &self))
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut arr = [0u8; 16];
                for i in 0..16 {
                    arr[i] = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::invalid_length(i, &self))?;
                }
                Ok(FixedOpaque16(arr))
            }
        }

        // Use newtype_struct to signal we want raw bytes without length prefix
        deserializer.deserialize_newtype_struct("FixedOpaque16", FixedOpaque16Visitor)
    }
}
