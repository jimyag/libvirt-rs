# libvirt-codegen

Code generator for libvirt protocol from XDR (.x) definition files.

This crate parses libvirt's `.x` protocol definition files and generates Rust types and RPC client methods.

## Features

- XDR file parser using nom
- Rust code generator using quote
- Generates structs, enums, unions, typedefs
- Generates async RPC client methods for all 453+ libvirt procedures

## Usage

This crate is typically used as a build dependency:

```rust
// build.rs
use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    let protocol = libvirt_codegen::parse_file("proto/remote_protocol.x")
        .expect("failed to parse protocol");

    let code = libvirt_codegen::generate(&protocol);

    let dest = Path::new(&out_dir).join("generated.rs");
    fs::write(&dest, code).unwrap();

    println!("cargo:rerun-if-changed=proto/remote_protocol.x");
}
```

## License

MIT OR Apache-2.0
