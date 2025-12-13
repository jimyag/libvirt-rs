//! Pure Rust libvirt client library.
//!
//! This library provides a native Rust implementation for communicating
//! with libvirt daemons using the libvirt RPC protocol.
//!
//! # Example
//!
//! ```ignore
//! use libvirt::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::connect("qemu:///system").await?;
//!     let domains = client.list_all_domains(0).await?;
//!
//!     for domain in domains {
//!         println!("Domain: {}", domain.name().await?);
//!     }
//!
//!     Ok(())
//! }
//! ```

mod connection;
mod error;
mod packet;
mod transport;

/// Generated types and constants from libvirt protocol definition.
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[allow(clippy::all)]
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}

pub use connection::Connection;
pub use error::{Error, Result};
pub use generated::*;

/// Re-export GeneratedClient for convenient API access.
pub type LibvirtClient = GeneratedClient<Connection>;

/// High-level libvirt client that wraps the generated API.
///
/// This client provides a convenient interface for connecting to libvirt
/// and using the auto-generated RPC methods.
///
/// # Example
///
/// ```ignore
/// use libvirt::Client;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = Client::connect("qemu:///system").await?;
///
///     // Use generated API methods
///     let version = client.rpc().connect_get_version().await?;
///     println!("Hypervisor version: {}", version.hv_ver);
///
///     // List domains
///     let ret = client.rpc().connect_list_all_domains(
///         ConnectListAllDomainsArgs { need_results: 1, flags: 0 }
///     ).await?;
///     println!("Found {} domains", ret.domains.len());
///
///     client.close().await?;
///     Ok(())
/// }
/// ```
pub struct Client {
    rpc: GeneratedClient<Connection>,
}

impl Client {
    /// Connect to a libvirt daemon.
    ///
    /// # Supported URIs
    ///
    /// - `qemu:///system` - Connect to system QEMU/KVM daemon
    /// - `qemu:///session` - Connect to session QEMU/KVM daemon
    /// - Custom Unix socket paths
    pub async fn connect(uri: &str) -> Result<Self> {
        let conn = if uri.contains("///system") {
            Connection::connect_system().await?
        } else if uri.contains("///session") {
            Connection::connect_session().await?
        } else if uri.starts_with('/') || uri.starts_with("unix://") {
            let path = uri.strip_prefix("unix://").unwrap_or(uri);
            Connection::connect_unix(path).await?
        } else {
            return Err(Error::UnsupportedUri(uri.to_string()));
        };

        let rpc = GeneratedClient::new(conn);

        // Perform authentication (AUTH_NONE for local connections)
        let _ = rpc.auth_list().await
            .map_err(|e| Error::Protocol(format!("auth_list failed: {}", e)))?;

        // Open the connection
        let args = ConnectOpenArgs {
            name: Some(uri.to_string()),
            flags: 0,
        };
        rpc.connect_open(args).await
            .map_err(|e| Error::Protocol(format!("connect_open failed: {}", e)))?;

        Ok(Self { rpc })
    }

    /// Get access to all generated RPC methods.
    ///
    /// This returns a reference to the `GeneratedClient` which provides
    /// all 453+ auto-generated libvirt RPC methods.
    pub fn rpc(&self) -> &GeneratedClient<Connection> {
        &self.rpc
    }

    /// Close the connection.
    pub async fn close(&self) -> Result<()> {
        self.rpc.connect_close().await
            .map_err(|e| Error::Protocol(format!("connect_close failed: {}", e)))?;
        Ok(())
    }
}
