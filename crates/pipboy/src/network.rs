//! Explicit network-engineering CLI. No background monitoring daemon.

use pipboy_core::network_probe::{scan, NetworkReport};
use pipboy_core::unifi::{self, UniFiInventory};
use std::error::Error;
use std::thread;
use std::time::Duration;

const USAGE: &str =
    "usage: pipboy net <scan|watch> <local|PRIVATE_CIDR> --confirm [--json] [--interval-sec 30..3600 --count 1..60 (watch only)] | net unifi <sites|clients SITE|compare SITE CIDR> --confirm [--json]";

fn display(report: &NetworkReport, json: bool) -> Result<(), Box<dyn Error>> {
    if json {
        println!("{}", serde_json::to_string(report)?);
    } else {
        println!("{}", report.render_text());
    }
    Ok(())
}


fn display_unifi(inventory: &UniFiInventory) {
    println!(
        "UNIFI / CONNECTED CLIENTS\nSITE: {}\nCONTROLLER CONNECTED: {} | CLIENTS WITH IP: {}\n",
        inventory.site_id, inventory.connected_clients, inventory.clients_with_ip
    );
    for client in inventory.clients.iter().take(60) {
        let ip = client.ip_address.map(|ip| ip.to_string())
            .unwrap_or_else(|| "IP unavailable".into());
        println!("{}  {}  {}", ip, client.connection_type, client.name);
    }
    if inventory.clients.len() > 60 {
        println!("DISPLAY LIMIT: first 60 of {} clients; use --json for full data",
            inventory.clients.len());
    }
    println!("{}", inventory.limitation);
}

fn run_unifi(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut args = args.into_iter();
    let action = args.next().ok_or(USAGE)?;
    let site = if matches!(action.as_str(), "clients" | "compare") {
        Some(args.next().ok_or(USAGE)?)
    } else { None };
    let cidr = if action == "compare" {
        Some(args.next().ok_or(USAGE)?)
    } else { None };
    if !matches!(action.as_str(), "sites" | "clients" | "compare") {
        return Err(USAGE.into());
    }
    let mut confirm = false;
    let mut json = false;
    for arg in args {
        match arg.as_str() {
            "--confirm" if !confirm => confirm = true,
            "--json" if !json => json = true,
            _ => return Err(USAGE.into()),
        }
    }
    if !confirm {
        return Err("UniFi read access requires --confirm for an owned/authorized controller".into());
    }
    // Validate all scopes before making even a read-only request to the controller.
    if let Some(site) = &site {
        if !unifi::valid_site_id(site) {
            return Err("UniFi site ID must be a canonical UUID".into());
        }
    }
    if let Some(cidr) = &cidr {
        pipboy_core::network_probe::NetworkScope::parse(cidr)?;
    }
    match action.as_str() {
        "sites" => {
            let sites = unifi::list_sites(true)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&sites)?);
            } else {
                println!("UNIFI / READ-ONLY SITE INVENTORY");
                for site in &sites.sites {
                    println!("{}  {}", site.id, site.name);
                }
                println!("SITES: {}", sites.sites.len());
            }
        }
        "clients" => {
            let inventory = unifi::list_clients(site.as_deref().ok_or(USAGE)?, true)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&inventory)?);
            } else {
                display_unifi(&inventory);
            }
        }
        "compare" => {
            let inventory = unifi::list_clients(site.as_deref().ok_or(USAGE)?, true)?;
            let report = scan(cidr.as_deref().ok_or(USAGE)?, true)?;
            let comparison = unifi::compare_inventory(&report, &inventory);
            if json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "controller": inventory, "probe": report, "comparison": comparison
                }))?);
            } else {
                println!("UNIFI / LIVE-PROBE CROSS-CHECK");
                println!("CONTROLLER CONNECTED: {}", comparison.controller_clients);
                println!("CONTROLLER IPv4: {}", comparison.controller_with_ipv4);
                println!("MATCHED ACTIVE RESPONSE: {}", comparison.matched_active_probe);
                println!("CONTROLLER IPV4 NOT PROBED: {}", comparison.controller_not_probed);
                println!("PROBE ONLY: {}", comparison.active_probe_only);
                println!("{}", comparison.note);
                println!("{}", report.render_text());
            }
        }
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if args.first().is_some_and(|action| action == "unifi") {
        return run_unifi(args.into_iter().skip(1).collect());
    }
    let mut iter = args.into_iter();
    let mode = iter.next().ok_or(USAGE)?;
    if !matches!(mode.as_str(), "scan" | "watch") {
        return Err(USAGE.into());
    }
    let cidr = iter.next().ok_or(USAGE)?;
    let mut confirm = false;
    let mut json = false;
    let mut interval = 60_u64;
    let mut count = 5_u8;
    let mut interval_set = false;
    let mut count_set = false;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--confirm" if !confirm => confirm = true,
            "--json" if !json => json = true,
            "--interval-sec" if mode == "watch" && !interval_set => {
                interval = iter.next().ok_or(USAGE)?.parse()?;
                interval_set = true;
            }
            "--count" if mode == "watch" && !count_set => {
                count = iter.next().ok_or(USAGE)?.parse()?;
                count_set = true;
            }
            _ => return Err(USAGE.into()),
        }
    }
    if !confirm {
        return Err("NET requires --confirm: actively probe only systems and networks you are authorized to test".into());
    }
    if mode == "watch" && (!(30..=3600).contains(&interval) || !(1..=60).contains(&count)) {
        return Err("watch interval must be 30..3600 seconds and count 1..60".into());
    }
    let passes = if mode == "scan" { 1 } else { usize::from(count) };
    for n in 0..passes {
        let report = scan(&cidr, confirm)?;
        if mode == "watch" && !json {
            println!("SWEEP {} / {}", n + 1, passes);
        }
        display(&report, json)?;
        if n + 1 < passes {
            thread::sleep(Duration::from_secs(interval));
        }
    }
    Ok(())
}
