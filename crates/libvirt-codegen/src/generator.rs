//! Rust code generator from XDR AST.

use crate::ast::*;
use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generate Rust code from a protocol definition.
pub fn generate(protocol: &Protocol) -> String {
    let mut tokens = TokenStream::new();

    // Generate prelude
    tokens.extend(generate_prelude());

    // Generate constants
    for constant in &protocol.constants {
        tokens.extend(generate_constant(constant));
    }

    // Generate types
    for type_def in &protocol.types {
        tokens.extend(generate_type(type_def));
    }

    // Generate RPC client methods
    tokens.extend(generate_client_methods(&protocol.procedures));

    // Format the output
    let file = syn::parse2(tokens).expect("generated invalid Rust code");
    prettyplease::unparse(&file)
}

fn generate_prelude() -> TokenStream {
    // Note: This code is included into a submodule via include!(),
    // so we cannot use inner attributes (like #![allow(...)]).
    // The parent module should add the necessary attributes.
    quote! {
        // Generated code from libvirt protocol definition.
        // Do not edit manually.

        use serde::{Serialize, Deserialize};

        // Well-known libvirt constants from libvirt.h
        pub const VIR_UUID_BUFLEN: usize = 16;
        pub const VIR_UUID_STRING_BUFLEN: usize = 37;

        // Re-export fixed opaque type for UUID
        pub use libvirt_xdr::opaque::FixedOpaque16;
    }
}
fn generate_constant(constant: &Constant) -> TokenStream {
    let name = format_ident!("{}", constant.name);

    // Only generate constants with literal integer values.
    // Skip constants that reference external symbols (like VIR_* from libvirt.h)
    // since we don't have their definitions.
    match &constant.value {
        ConstValue::Int(n) => {
            quote! {
                pub const #name: i64 = #n;
            }
        }
        ConstValue::Ident(_) => {
            // Skip - references external constant we don't have
            TokenStream::new()
        }
    }
}

fn generate_type(type_def: &TypeDef) -> TokenStream {
    match type_def {
        TypeDef::Struct(s) => generate_struct(s),
        TypeDef::Enum(e) => generate_enum(e),
        TypeDef::Union(u) => generate_union(u),
        TypeDef::Typedef(t) => generate_typedef(t),
    }
}

fn generate_struct(s: &StructDef) -> TokenStream {
    let name = format_ident!("{}", to_rust_type_name(&s.name));

    let fields: Vec<_> = s
        .fields
        .iter()
        .map(|f| {
            let field_name = format_ident!("{}", to_rust_field_name(&f.name));
            let field_type = type_to_tokens(&f.ty);
            quote! {
                pub #field_name: #field_type
            }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct #name {
            #(#fields),*
        }
    }
}

fn generate_enum(e: &EnumDef) -> TokenStream {
    let name = format_ident!("{}", to_rust_type_name(&e.name));

    let variants: Vec<_> = e
        .variants
        .iter()
        .filter_map(|v| {
            let variant_name = format_ident!("{}", to_rust_variant_name(&v.name, &e.name));

            match &v.value {
                Some(ConstValue::Int(n)) => {
                    let n = *n as i32;
                    Some(quote! { #variant_name = #n })
                }
                Some(ConstValue::Ident(_)) => {
                    // Skip variants that reference other constants
                    None
                }
                None => Some(quote! { #variant_name }),
            }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[repr(i32)]
        pub enum #name {
            #(#variants),*
        }
    }
}

fn generate_union(u: &UnionDef) -> TokenStream {
    let name = format_ident!("{}", to_rust_type_name(&u.name));

    let variants: Vec<_> = u
        .cases
        .iter()
        .filter_map(|case| {
            let variant_name = match &case.values.first()? {
                ConstValue::Int(n) => format_ident!("V{}", *n as u64),
                ConstValue::Ident(s) => format_ident!("{}", to_rust_variant_name(s, &u.name)),
            };

            match &case.field {
                Some(f) => {
                    let field_type = type_to_tokens(&f.ty);
                    Some(quote! { #variant_name(#field_type) })
                }
                None => Some(quote! { #variant_name }),
            }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub enum #name {
            #(#variants),*
        }
    }
}

fn generate_typedef(t: &TypedefDef) -> TokenStream {
    let name = format_ident!("{}", to_rust_type_name(&t.name));
    let target = type_to_tokens(&t.target);

    quote! {
        pub type #name = #target;
    }
}

fn type_to_tokens(ty: &Type) -> TokenStream {
    match ty {
        Type::Void => quote! { () },
        Type::Int => quote! { i32 },
        Type::UInt => quote! { u32 },
        Type::Hyper => quote! { i64 },
        Type::UHyper => quote! { u64 },
        Type::Float => quote! { f32 },
        Type::Double => quote! { f64 },
        Type::Bool => quote! { bool },
        Type::String { .. } => quote! { String },
        Type::Opaque { len } => match len {
            LengthSpec::Fixed(n) => {
                let n = *n as usize;
                // Use FixedOpaque16 for 16-byte opaque (UUID) to handle XDR correctly
                if n == 16 {
                    quote! { FixedOpaque16 }
                } else {
                    quote! { [u8; #n] }
                }
            }
            LengthSpec::Variable { .. } => quote! { Vec<u8> },
        },
        Type::Array { elem, len } => {
            let elem_type = type_to_tokens(elem);
            match len {
                LengthSpec::Fixed(n) => {
                    let n = *n as usize;
                    quote! { [#elem_type; #n] }
                }
                LengthSpec::Variable { .. } => quote! { Vec<#elem_type> },
            }
        }
        Type::Optional(inner) => {
            let inner_type = type_to_tokens(inner);
            quote! { Option<#inner_type> }
        }
        Type::Named(name) => {
            let ident = format_ident!("{}", to_rust_type_name(name));
            quote! { #ident }
        }
    }
}

/// Convert XDR type name to Rust type name (PascalCase).
fn to_rust_type_name(name: &str) -> String {
    // Preserve Rust primitive types as-is
    match name {
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
        "f32" | "f64" | "bool" | "char" | "str" | "String" => {
            return name.to_string();
        }
        _ => {}
    }

    // Remove common prefixes
    let name = name
        .strip_prefix("remote_")
        .or_else(|| name.strip_prefix("virNet"))
        .unwrap_or(name);

    let converted = name.to_upper_camel_case();

    // Avoid collision with Rust standard types
    match converted.as_str() {
        "String" => "RemoteString".to_string(),
        "Vec" => "RemoteVec".to_string(),
        "Option" => "RemoteOption".to_string(),
        "Box" => "RemoteBox".to_string(),
        "Result" => "RemoteResult".to_string(),
        _ => converted,
    }
}

/// Convert XDR field name to Rust field name (snake_case).
/// Handles Rust keywords by appending underscore.
fn to_rust_field_name(name: &str) -> String {
    let name = name.to_snake_case();

    // Handle Rust keywords
    match name.as_str() {
        "type" => "r#type".to_string(),
        "match" => "r#match".to_string(),
        "ref" => "r#ref".to_string(),
        "mod" => "r#mod".to_string(),
        "fn" => "r#fn".to_string(),
        "struct" => "r#struct".to_string(),
        "enum" => "r#enum".to_string(),
        "trait" => "r#trait".to_string(),
        "impl" => "r#impl".to_string(),
        "self" => "r#self".to_string(),
        "super" => "r#super".to_string(),
        "crate" => "r#crate".to_string(),
        "use" => "r#use".to_string(),
        "pub" => "r#pub".to_string(),
        "in" => "r#in".to_string(),
        "where" => "r#where".to_string(),
        "async" => "r#async".to_string(),
        "await" => "r#await".to_string(),
        "dyn" => "r#dyn".to_string(),
        "loop" => "r#loop".to_string(),
        "move" => "r#move".to_string(),
        "return" => "r#return".to_string(),
        "static" => "r#static".to_string(),
        "const" => "r#const".to_string(),
        "unsafe" => "r#unsafe".to_string(),
        "extern" => "r#extern".to_string(),
        "let" => "r#let".to_string(),
        "mut" => "r#mut".to_string(),
        "if" => "r#if".to_string(),
        "else" => "r#else".to_string(),
        "for" => "r#for".to_string(),
        "while" => "r#while".to_string(),
        "break" => "r#break".to_string(),
        "continue" => "r#continue".to_string(),
        "as" => "r#as".to_string(),
        "box" => "r#box".to_string(),
        "priv" => "r#priv".to_string(),
        "abstract" => "r#abstract".to_string(),
        "final" => "r#final".to_string(),
        "override" => "r#override".to_string(),
        "virtual" => "r#virtual".to_string(),
        "yield" => "r#yield".to_string(),
        "become" => "r#become".to_string(),
        "macro" => "r#macro".to_string(),
        "typeof" => "r#typeof".to_string(),
        "try" => "r#try".to_string(),
        "union" => "r#union".to_string(),
        _ => name,
    }
}

/// Convert XDR enum variant name to Rust variant name.
fn to_rust_variant_name(name: &str, enum_name: &str) -> String {
    // Try to strip the enum name prefix
    let name = name
        .strip_prefix(&format!("{}_", enum_name.to_uppercase()))
        .or_else(|| name.strip_prefix("REMOTE_"))
        .or_else(|| name.strip_prefix("VIR_"))
        .unwrap_or(name);

    name.to_upper_camel_case()
}

/// Generate RPC client methods from procedure definitions.
fn generate_client_methods(procedures: &[Procedure]) -> TokenStream {
    let methods: Vec<_> = procedures
        .iter()
        .map(|proc| generate_client_method(proc))
        .collect();

    quote! {
        /// Trait for making RPC calls to libvirt daemon.
        /// This trait is implemented by the Connection type.
        #[allow(async_fn_in_trait)]
        pub trait LibvirtRpc {
            /// Make an RPC call with the given procedure number and payload.
            async fn rpc_call(&self, procedure: u32, payload: Vec<u8>) -> Result<Vec<u8>, RpcError>;
        }

        /// Error type for RPC operations.
        #[derive(Debug)]
        pub enum RpcError {
            /// XDR encoding error
            Encode(String),
            /// XDR decoding error
            Decode(String),
            /// Transport/connection error
            Transport(String),
            /// Server returned an error
            Server(Error),
        }

        impl std::fmt::Display for RpcError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    RpcError::Encode(e) => write!(f, "XDR encode error: {}", e),
                    RpcError::Decode(e) => write!(f, "XDR decode error: {}", e),
                    RpcError::Transport(e) => write!(f, "Transport error: {}", e),
                    RpcError::Server(e) => write!(f, "Server error: {:?}", e),
                }
            }
        }

        impl std::error::Error for RpcError {}

        /// Generated RPC client methods for libvirt protocol.
        pub struct GeneratedClient<T: LibvirtRpc> {
            inner: T,
        }

        impl<T: LibvirtRpc> GeneratedClient<T> {
            /// Create a new GeneratedClient wrapping an RPC transport.
            pub fn new(inner: T) -> Self {
                Self { inner }
            }

            /// Get a reference to the inner transport.
            pub fn inner(&self) -> &T {
                &self.inner
            }

            /// Get a mutable reference to the inner transport.
            pub fn inner_mut(&mut self) -> &mut T {
                &mut self.inner
            }

            #(#methods)*
        }
    }
}

/// Generate a single RPC method for a procedure.
fn generate_client_method(proc: &Procedure) -> TokenStream {
    // Convert REMOTE_PROC_CONNECT_LIST_DOMAINS to connect_list_domains
    let method_name = proc
        .name
        .strip_prefix("REMOTE_PROC_")
        .unwrap_or(&proc.name)
        .to_lowercase();
    let method_ident = format_ident!("{}", method_name);

    // Convert to Procedure enum variant name: ProcConnectListDomains
    let proc_variant = format_ident!(
        "Proc{}",
        proc.name
            .strip_prefix("REMOTE_PROC_")
            .unwrap_or(&proc.name)
            .to_upper_camel_case()
    );

    match (&proc.args, &proc.ret) {
        (Some(args_name), Some(ret_name)) => {
            // Has both args and return
            let args_type = format_ident!("{}", to_rust_type_name(args_name));
            let ret_type = format_ident!("{}", to_rust_type_name(ret_name));

            quote! {
                /// RPC method for procedure #method_name.
                pub async fn #method_ident(&self, args: #args_type) -> Result<#ret_type, RpcError> {
                    let payload = libvirt_xdr::to_bytes(&args)
                        .map_err(|e| RpcError::Encode(e.to_string()))?;
                    let response = self.inner.rpc_call(Procedure::#proc_variant as u32, payload).await?;
                    libvirt_xdr::from_bytes(&response)
                        .map_err(|e| RpcError::Decode(e.to_string()))
                }
            }
        }
        (Some(args_name), None) => {
            // Has args but no return
            let args_type = format_ident!("{}", to_rust_type_name(args_name));

            quote! {
                /// RPC method for procedure #method_name.
                pub async fn #method_ident(&self, args: #args_type) -> Result<(), RpcError> {
                    let payload = libvirt_xdr::to_bytes(&args)
                        .map_err(|e| RpcError::Encode(e.to_string()))?;
                    let _ = self.inner.rpc_call(Procedure::#proc_variant as u32, payload).await?;
                    Ok(())
                }
            }
        }
        (None, Some(ret_name)) => {
            // No args but has return
            let ret_type = format_ident!("{}", to_rust_type_name(ret_name));

            quote! {
                /// RPC method for procedure #method_name.
                pub async fn #method_ident(&self) -> Result<#ret_type, RpcError> {
                    let response = self.inner.rpc_call(Procedure::#proc_variant as u32, Vec::new()).await?;
                    libvirt_xdr::from_bytes(&response)
                        .map_err(|e| RpcError::Decode(e.to_string()))
                }
            }
        }
        (None, None) => {
            // No args and no return
            quote! {
                /// RPC method for procedure #method_name.
                pub async fn #method_ident(&self) -> Result<(), RpcError> {
                    let _ = self.inner.rpc_call(Procedure::#proc_variant as u32, Vec::new()).await?;
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_rust_type_name() {
        assert_eq!(to_rust_type_name("remote_domain"), "Domain");
        assert_eq!(to_rust_type_name("remote_nonnull_domain"), "NonnullDomain");
        assert_eq!(to_rust_type_name("foo_bar"), "FooBar");
    }

    #[test]
    fn test_to_rust_field_name() {
        assert_eq!(to_rust_field_name("maxMem"), "max_mem");
        assert_eq!(to_rust_field_name("nrVirtCpu"), "nr_virt_cpu");
    }

    #[test]
    fn test_generate_struct() {
        let s = StructDef {
            name: "remote_domain".to_string(),
            fields: vec![
                Field {
                    name: "name".to_string(),
                    ty: Type::String { max_len: None },
                },
                Field {
                    name: "id".to_string(),
                    ty: Type::Int,
                },
            ],
        };

        let code = generate_struct(&s).to_string();
        assert!(code.contains("struct Domain"));
        assert!(code.contains("name : String"));
        assert!(code.contains("id : i32"));
    }

    #[test]
    fn test_generate_enum() {
        let e = EnumDef {
            name: "remote_domain_state".to_string(),
            variants: vec![
                EnumVariant {
                    name: "VIR_DOMAIN_NOSTATE".to_string(),
                    value: Some(ConstValue::Int(0)),
                },
                EnumVariant {
                    name: "VIR_DOMAIN_RUNNING".to_string(),
                    value: Some(ConstValue::Int(1)),
                },
            ],
        };

        let code = generate_enum(&e).to_string();
        assert!(code.contains("enum DomainState"));
        assert!(code.contains("DomainNostate"));
        assert!(code.contains("DomainRunning"));
    }
}
