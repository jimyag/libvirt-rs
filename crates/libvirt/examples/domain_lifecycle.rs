//! Example: Domain lifecycle management.
//!
//! This example demonstrates domain lifecycle operations:
//! - List domains
//! - Suspend/Resume
//! - Shutdown/Start
//! - Destroy
//!
//! Usage:
//!   domain_lifecycle list              - List all domains
//!   domain_lifecycle suspend <name>    - Suspend a domain
//!   domain_lifecycle resume <name>     - Resume a suspended domain
//!   domain_lifecycle shutdown <name>   - Graceful shutdown
//!   domain_lifecycle start <name>      - Start a stopped domain
//!   domain_lifecycle destroy <name>    - Force stop a domain
//!   domain_lifecycle reboot <name>     - Reboot a domain

use libvirt::{
    Client, ConnectListAllDomainsArgs, DomainCreateArgs, DomainDestroyArgs,
    DomainLookupByNameArgs, DomainRebootArgs, DomainResumeArgs, DomainShutdownArgs,
    DomainSuspendArgs, NonnullDomain,
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        return Ok(());
    }

    let command = &args[1];

    // Connect to libvirt
    let client = match Client::connect("qemu:///system").await {
        Ok(c) => {
            println!("[OK] Connected to qemu:///system");
            c
        }
        Err(e) => {
            println!("[WARN] System connection failed: {}", e);
            println!("[INFO] Trying qemu:///session...");
            Client::connect("qemu:///session").await?
        }
    };

    match command.as_str() {
        "list" => list_domains(&client).await?,
        "suspend" => {
            let name = args.get(2).ok_or("Missing domain name")?;
            suspend_domain(&client, name).await?;
        }
        "resume" => {
            let name = args.get(2).ok_or("Missing domain name")?;
            resume_domain(&client, name).await?;
        }
        "shutdown" => {
            let name = args.get(2).ok_or("Missing domain name")?;
            shutdown_domain(&client, name).await?;
        }
        "start" => {
            let name = args.get(2).ok_or("Missing domain name")?;
            start_domain(&client, name).await?;
        }
        "destroy" => {
            let name = args.get(2).ok_or("Missing domain name")?;
            destroy_domain(&client, name).await?;
        }
        "reboot" => {
            let name = args.get(2).ok_or("Missing domain name")?;
            reboot_domain(&client, name).await?;
        }
        _ => {
            println!("Unknown command: {}", command);
            print_usage(&args[0]);
        }
    }

    client.close().await?;
    Ok(())
}

fn print_usage(program: &str) {
    println!("Usage: {} <command> [args]", program);
    println!();
    println!("Commands:");
    println!("  list              - List all domains with status");
    println!("  suspend <name>    - Suspend a running domain");
    println!("  resume <name>     - Resume a suspended domain");
    println!("  shutdown <name>   - Graceful shutdown of a domain");
    println!("  start <name>      - Start a stopped domain");
    println!("  destroy <name>    - Force stop a domain");
    println!("  reboot <name>     - Reboot a domain");
}

async fn list_domains(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Domain List ===\n");

    let args = ConnectListAllDomainsArgs {
        need_results: 1,
        flags: 0, // All domains
    };

    match client.rpc().connect_list_all_domains(args).await {
        Ok(ret) => {
            if ret.domains.is_empty() {
                println!("No domains found.");
                return Ok(());
            }

            println!(
                "{:<20} {:<38} {:<10}",
                "NAME", "UUID", "STATE"
            );
            println!("{}", "-".repeat(70));

            for dom in &ret.domains {
                let state = if dom.id == -1 { "shut off" } else { "running" };
                println!(
                    "{:<20} {:<38} {:<10}",
                    dom.name,
                    dom.uuid,
                    state
                );
            }
            println!("\nTotal: {} domain(s)", ret.domains.len());
        }
        Err(e) => {
            println!("Failed to list domains: {}", e);
        }
    }

    Ok(())
}

async fn lookup_domain(
    client: &Client,
    name: &str,
) -> Result<NonnullDomain, Box<dyn std::error::Error>> {
    let args = DomainLookupByNameArgs {
        name: name.to_string(),
    };

    let ret = client
        .rpc()
        .domain_lookup_by_name(args)
        .await
        .map_err(|e| format!("Failed to find domain '{}': {}", name, e))?;

    Ok(ret.dom)
}

async fn suspend_domain(client: &Client, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Suspending domain '{}'...", name);

    let dom = lookup_domain(client, name).await?;
    let args = DomainSuspendArgs { dom };

    client
        .rpc()
        .domain_suspend(args)
        .await
        .map_err(|e| format!("Failed to suspend: {}", e))?;

    println!("[OK] Domain '{}' suspended.", name);
    Ok(())
}

async fn resume_domain(client: &Client, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Resuming domain '{}'...", name);

    let dom = lookup_domain(client, name).await?;
    let args = DomainResumeArgs { dom };

    client
        .rpc()
        .domain_resume(args)
        .await
        .map_err(|e| format!("Failed to resume: {}", e))?;

    println!("[OK] Domain '{}' resumed.", name);
    Ok(())
}

async fn shutdown_domain(client: &Client, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Shutting down domain '{}'...", name);

    let dom = lookup_domain(client, name).await?;
    let args = DomainShutdownArgs { dom };

    client
        .rpc()
        .domain_shutdown(args)
        .await
        .map_err(|e| format!("Failed to shutdown: {}", e))?;

    println!("[OK] Shutdown signal sent to '{}'.", name);
    Ok(())
}

async fn start_domain(client: &Client, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting domain '{}'...", name);

    let dom = lookup_domain(client, name).await?;
    let args = DomainCreateArgs { dom };

    client
        .rpc()
        .domain_create(args)
        .await
        .map_err(|e| format!("Failed to start: {}", e))?;

    println!("[OK] Domain '{}' started.", name);
    Ok(())
}

async fn destroy_domain(client: &Client, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Destroying domain '{}'...", name);

    let dom = lookup_domain(client, name).await?;
    let args = DomainDestroyArgs { dom };

    client
        .rpc()
        .domain_destroy(args)
        .await
        .map_err(|e| format!("Failed to destroy: {}", e))?;

    println!("[OK] Domain '{}' destroyed.", name);
    Ok(())
}

async fn reboot_domain(client: &Client, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Rebooting domain '{}'...", name);

    let dom = lookup_domain(client, name).await?;
    let args = DomainRebootArgs { dom, flags: 0 };

    client
        .rpc()
        .domain_reboot(args)
        .await
        .map_err(|e| format!("Failed to reboot: {}", e))?;

    println!("[OK] Reboot signal sent to '{}'.", name);
    Ok(())
}
