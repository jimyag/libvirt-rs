//! XDR (External Data Representation) serialization for libvirt protocol.
//!
//! This crate provides serde-based serialization and deserialization
//! for the XDR binary format used by libvirt's RPC protocol.

mod de;
mod error;
pub mod opaque;
mod ser;

pub use de::XdrDeserializer;
pub use error::{Error, Result};
pub use ser::XdrSerializer;

use serde::{de::DeserializeOwned, Serialize};

/// Serialize a value to XDR bytes.
pub fn to_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut serializer = XdrSerializer::new();
    value.serialize(&mut serializer)?;
    Ok(serializer.into_bytes())
}

/// Deserialize a value from XDR bytes.
pub fn from_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let mut deserializer = XdrDeserializer::new(bytes);
    T::deserialize(&mut deserializer)
}
