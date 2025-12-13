//! Build script for libvirt crate.
//!
//! This script uses libvirt-codegen to generate Rust code from
//! the libvirt protocol definition files.

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    // Parse the remote protocol definition
    // Use CARGO_MANIFEST_DIR to get the correct path regardless of where cargo is invoked
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let proto_path = std::path::Path::new(&manifest_dir).join("proto/remote_protocol.x");

    let protocol = libvirt_codegen::parse_file(proto_path.to_str().unwrap())
        .expect("failed to parse protocol");

    // Generate Rust code
    let code = libvirt_codegen::generate(&protocol);

    // Write to OUT_DIR
    let dest = std::path::Path::new(&out_dir).join("generated.rs");
    std::fs::write(&dest, code).expect("failed to write generated code");

    // Tell Cargo to rerun if these files change
    println!("cargo:rerun-if-changed=proto/remote_protocol.x");
    println!("cargo:rerun-if-changed=build.rs");
}
