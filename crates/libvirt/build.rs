//! Build script for libvirt crate.
//!
//! This script uses libvirt-codegen to generate Rust code from
//! the libvirt protocol definition files.

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();

    // Use CARGO_MANIFEST_DIR to get the correct path regardless of where cargo is invoked
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let proto_dir = std::path::Path::new(&manifest_dir).join("proto");

    // Parse all protocol files
    let mut bundle = libvirt_codegen::ProtocolBundle::new();

    // Parse the main remote protocol
    let remote_path = proto_dir.join("remote_protocol.x");
    bundle.remote = Some(
        libvirt_codegen::parse_file(remote_path.to_str().unwrap())
            .expect("failed to parse remote protocol"),
    );

    // Parse QEMU protocol (optional)
    let qemu_path = proto_dir.join("qemu_protocol.x");
    if qemu_path.exists() {
        bundle.qemu = Some(
            libvirt_codegen::parse_file(qemu_path.to_str().unwrap())
                .expect("failed to parse qemu protocol"),
        );
    }

    // Parse LXC protocol (optional)
    let lxc_path = proto_dir.join("lxc_protocol.x");
    if lxc_path.exists() {
        bundle.lxc = Some(
            libvirt_codegen::parse_file(lxc_path.to_str().unwrap())
                .expect("failed to parse lxc protocol"),
        );
    }

    // Generate Rust code from all protocols
    let code = libvirt_codegen::generate_bundle(&bundle);

    // Write to OUT_DIR
    let dest = std::path::Path::new(&out_dir).join("generated.rs");
    std::fs::write(&dest, code).expect("failed to write generated code");

    // Tell Cargo to rerun if these files change
    println!("cargo:rerun-if-changed=proto/remote_protocol.x");
    println!("cargo:rerun-if-changed=proto/qemu_protocol.x");
    println!("cargo:rerun-if-changed=proto/lxc_protocol.x");
    println!("cargo:rerun-if-changed=build.rs");
}
