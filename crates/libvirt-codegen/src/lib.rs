//! Code generator for libvirt protocol (.x files).
//!
//! This crate parses XDR protocol definition files and generates Rust code
//! for types and RPC methods.

pub mod ast;
pub mod generator;
pub mod parser;

pub use ast::Protocol;
pub use generator::generate;
pub use parser::parse_file;
