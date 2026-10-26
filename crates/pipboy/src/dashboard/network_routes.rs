//! Network Engineer HTTP adapters: explicit private-LAN scan and read-only UniFi.
//!
//! Parent dashboard is the sole loopback HTTP server and performs host/origin
//! checks before calling either handler. Private scope and authorization are
//! validated before any request reaches network_probe or the UniFi adapter.
use super::{parse_json_body, str_field, Request, Response};
use pipboy_core::network_probe;
use pipboy_core::unifi;
use pipboy_core::workbench::WorkbenchStore;
use serde_json::{json, Value};

pub(super) fn scan_network(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let body = parse_json_body(req, &["cidr", "confirm"])?;
    if body.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err(Response::error(
            422,
            "Confirm authorization for this private LAN scope",
        ));
    }
    let cidr = str_field(&body, "cidr")?;
    if cidr.len() > 32 {
        return Err(Response::error(422, "LAN scope exceeds 32 bytes"));
    }
    let board = store
        .load()
        .map_err(|_| Response::error(409, "Workbench unavailable"))?;
    if !board.slots.iter().any(|slot| slot.tool == "net-lab") {
        return Err(Response::error(
            409,
            "Activate a role with NET ENGINEER first",
        ));
    }
    let report = network_probe::scan(cidr, true)
        .map_err(|_| Response::error(422, "Invalid network scope or scan already active"))?;
    Ok(Response::json(
        200,
        &json!({
            "kind": "network_scan",
            "output": report.render_text(),
            "report": report
        }),
    ))
}

pub(super) fn unifi_request(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let body = parse_json_body(req, &["operation", "site", "cidr", "confirm"])?;
    if body.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err(Response::error(
            422,
            "Authorize this read-only UniFi controller request",
        ));
    }
    let board = store
        .load()
        .map_err(|_| Response::error(409, "Workbench unavailable"))?;
    if !board.slots.iter().any(|slot| slot.tool == "net-lab") {
        return Err(Response::error(409, "Activate NET ENGINEER first"));
    }
    let operation = str_field(&body, "operation")?;
    let site = if matches!(operation, "clients" | "compare") {
        let value = str_field(&body, "site")?;
        if !unifi::valid_site_id(value) {
            return Err(Response::error(422, "Enter a valid UniFi site UUID"));
        }
        Some(value)
    } else {
        None
    };
    let scope = if operation == "compare" {
        let value = str_field(&body, "cidr")?;
        if value.len() > 32 || network_probe::NetworkScope::parse(value).is_err() {
            return Err(Response::error(
                422,
                "Compare requires a private /24..../32 IPv4 CIDR",
            ));
        }
        Some(value)
    } else {
        None
    };
    let read_error = |_| {
        Response::error(
        409,
        "UniFi read-only HTTPS query failed; verify private controller configuration, trusted CA, API key and site ID"
    )
    };
    match operation {
        "sites" => {
            let data = unifi::list_sites(true).map_err(read_error)?;
            let mut output = format!("UNIFI / CONTROLLER SITES\nSITES: {}\n", data.sites.len());
            for row in &data.sites {
                output.push_str(&format!("{}  {}\n", row.id, row.name));
            }
            Ok(Response::json(
                200,
                &json!({
                    "kind": "unifi_sites", "output": output, "inventory": data
                }),
            ))
        }
        "clients" => {
            let data = unifi::list_clients(
                site.ok_or_else(|| Response::error(422, "Site ID required"))?,
                true,
            )
            .map_err(read_error)?;
            let mut output = format!(
                "UNIFI / CONTROLLER CONNECTED\nCLIENTS: {}  WITH IP: {}\n",
                data.connected_clients, data.clients_with_ip
            );
            for client in data.clients.iter().take(60) {
                output.push_str(&format!(
                    "{}  {}  {}\n",
                    client
                        .ip_address
                        .map(|ip| ip.to_string())
                        .unwrap_or_else(|| "IP unknown".into()),
                    client.connection_type,
                    client.name
                ));
            }
            if data.clients.len() > 60 {
                output.push_str("Only first 60 shown; CLI --json provides complete inventory.\n");
            }
            output.push_str(&data.limitation);
            Ok(Response::json(
                200,
                &json!({
                    "kind": "unifi_clients", "output": output, "inventory": data
                }),
            ))
        }
        "compare" => {
            let data = unifi::list_clients(
                site.ok_or_else(|| Response::error(422, "Site ID required"))?,
                true,
            )
            .map_err(read_error)?;
            let report = network_probe::scan(
                scope.ok_or_else(|| Response::error(422, "Private subnet required"))?,
                true,
            )
            .map_err(|_| Response::error(409, "Private LAN scan unavailable or in progress"))?;
            let comparison = unifi::compare_inventory(&report, &data);
            let output = format!(
                "NET ENGINEER / UNIFI CROSS-CHECK\nCONTROLLER CONNECTED: {}\nCONTROLLER IPv4: {}\nMATCHED RESPONSES: {}\nCONTROLLER IPV4 NOT PROBED: {}\nPROBED / NO CONTROLLER IP MATCH: {}\n\n{}\n\n{}",
                comparison.controller_clients,
                comparison.controller_with_ipv4,
                comparison.matched_active_probe,
                comparison.controller_not_probed,
                comparison.active_probe_only,
                comparison.note,
                report.render_text()
            );
            Ok(Response::json(
                200,
                &json!({
                    "kind": "unifi_comparison", "output": output,
                    "inventory": data, "report": report, "comparison": comparison
                }),
            ))
        }
        _ => Err(Response::error(
            422,
            "Unsupported UniFi read-only operation",
        )),
    }
}
