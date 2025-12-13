# libvirt-xdr

XDR (External Data Representation) serialization/deserialization for libvirt protocol.

This crate provides serde-based XDR encoding/decoding as specified in [RFC 4506](https://tools.ietf.org/html/rfc4506), specifically tailored for the libvirt RPC protocol.

## Features

- Serde-based serializer and deserializer
- Support for all XDR primitive types (int, uint, hyper, bool, string, opaque, etc.)
- Special handling for fixed-length opaque data (UUID)
- 4-byte alignment and padding

## Usage

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct MyStruct {
    name: String,
    value: i32,
}

// Serialize to XDR
let data = MyStruct { name: "test".into(), value: 42 };
let bytes = libvirt_xdr::to_bytes(&data)?;

// Deserialize from XDR
let decoded: MyStruct = libvirt_xdr::from_bytes(&bytes)?;
```

## License

MIT OR Apache-2.0
