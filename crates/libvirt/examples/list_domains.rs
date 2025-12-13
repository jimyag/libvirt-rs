//! Example: List all active domains.
//!
//! This example connects to the local libvirt daemon and lists
//! all running virtual machines.
//!
//! # Usage
//!
//! ```sh
//! cargo run --example list_domains
//! ```
//!
//! Note: Requires libvirtd to be running.

use libvirt::{Client, ConnectListDomainsArgs};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to libvirt...");

    // Try system connection first, fall back to session
    let client = match Client::connect("qemu:///system").await {
        Ok(c) => {
            println!("Connected to system daemon");
            c
        }
        Err(e) => {
            println!("Failed to connect to system daemon: {}", e);
            println!("Trying session daemon...");
            Client::connect("qemu:///session").await?
        }
    };

    // Get number of active domains using generated API
    match client.rpc().connect_num_of_domains().await {
        Ok(ret) => {
            println!("Number of active domains: {}", ret.num);
        }
        Err(e) => {
            println!("Failed to get domain count: {}", e);
        }
    }

    // List domain IDs using generated API
    let args = ConnectListDomainsArgs { maxids: 100 };
    match client.rpc().connect_list_domains(args).await {
        Ok(ret) => {
            println!("Active domain IDs: {:?}", ret.ids);
        }
        Err(e) => {
            println!("Failed to list domains: {}", e);
        }
    }

    // Get libvirt version using generated API
    match client.rpc().connect_get_version().await {
        Ok(ret) => {
            let version = ret.hv_ver;
            let major = version / 1_000_000;
            let minor = (version / 1_000) % 1_000;
            let micro = version % 1_000;
            println!("Hypervisor version: {}.{}.{}", major, minor, micro);
        }
        Err(e) => {
            println!("Failed to get version: {}", e);
        }
    }

    // Close connection
    if let Err(e) = client.close().await {
        println!("Failed to close connection: {}", e);
    }

    println!("Done!");
    Ok(())
}
