//! Example: Show detailed domain information.
//!
//! This example connects to the local libvirt daemon and shows
//! detailed information about all domains using the auto-generated API.

use libvirt::{Client, ConnectListAllDomainsArgs, ConnectListDomainsArgs};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== libvirt-rs Domain Info ===\n");

    // Try system connection first, fall back to session
    let client = match Client::connect("qemu:///system").await {
        Ok(c) => {
            println!("[OK] Connected to qemu:///system");
            c
        }
        Err(e) => {
            println!("[WARN] System connection failed: {}", e);
            println!("[INFO] Trying qemu:///session...");
            match Client::connect("qemu:///session").await {
                Ok(c) => {
                    println!("[OK] Connected to qemu:///session");
                    c
                }
                Err(e2) => {
                    eprintln!("[ERROR] Session connection also failed: {}", e2);
                    eprintln!("\nPlease make sure libvirtd is running:");
                    eprintln!("  sudo systemctl start libvirtd");
                    return Err(e2.into());
                }
            }
        }
    };

    println!();

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
            println!("Hypervisor version: (failed to get: {})", e);
        }
    }

    // Get number of active domains using generated API
    let active_count = match client.rpc().connect_num_of_domains().await {
        Ok(ret) => {
            println!("Active domains: {}", ret.num);
            ret.num
        }
        Err(e) => {
            println!("Active domains: (failed to get: {})", e);
            0
        }
    };

    println!();

    if active_count == 0 {
        println!("No active domains found.");
        println!("\nTo create a test domain, you can use:");
        println!("  virsh create /path/to/domain.xml");
    } else {
        // List domain IDs using generated API
        let args = ConnectListDomainsArgs { maxids: 100 };
        match client.rpc().connect_list_domains(args).await {
            Ok(ret) => {
                println!("=== Active Domains ===\n");
                for (idx, id) in ret.ids.iter().enumerate() {
                    println!("Domain #{}", idx + 1);
                    println!("  ID: {}", id);
                    println!();
                }
            }
            Err(e) => {
                println!("Failed to list domain IDs: {}", e);
            }
        }
    }

    // Try to list all domains (including inactive) using generated API
    println!("=== All Domains (active + inactive) ===\n");
    let args = ConnectListAllDomainsArgs {
        need_results: 1,
        flags: 0, // 0 = all domains
    };
    match client.rpc().connect_list_all_domains(args).await {
        Ok(ret) => {
            println!("Total domains: {}", ret.domains.len());
            for (idx, dom) in ret.domains.iter().enumerate() {
                println!("\nDomain #{}", idx + 1);
                println!("  Name: {}", dom.name);
                println!("  UUID: {}", dom.uuid);
                println!("  ID: {}", if dom.id == -1 { "inactive".to_string() } else { dom.id.to_string() });
            }
        }
        Err(e) => {
            println!("Failed to list all domains: {}", e);
        }
    }

    // Close connection
    if let Err(e) = client.close().await {
        eprintln!("\nWarning: Failed to close connection cleanly: {}", e);
    }

    println!("\n=== Done ===");
    Ok(())
}
