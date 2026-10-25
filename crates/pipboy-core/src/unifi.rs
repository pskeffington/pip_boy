//! Read-only UniFi Network local Integration API adapter.
//!
//! This adapter never performs HTTP writes. The controller is a private IPv4
//! address, TLS verification is mandatory, and API keys are passed on curl's
//! stdin (not in argv, URLs, logs, or browser JSON).
//!
//! Official API: GET /integration/v1/sites and
//! GET /integration/v1/sites/{uuid}/clients (paginated).
//! UniFi OS adds /proxy/network before /integration/v1.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const PAGE_SIZE: usize = 200;
const MAX_CLIENTS: usize = 2000;
const MAX_SITES: usize = 200;
const MAX_RESPONSE_BYTES: usize = 512 * 1024;

pub struct UniFiConfig {
    controller: Ipv4Addr,
    port: u16,
    prefix: &'static str,
    api_key: String,
    ca_cert: Option<PathBuf>,
}
impl UniFiConfig {
    pub fn new(
        controller: Ipv4Addr,
        port: u16,
        mode: &str,
        api_key: String,
        ca_cert: Option<PathBuf>,
    ) -> Result<Self, String> {
        if !controller.is_private()
            || controller.is_loopback()
            || controller.is_link_local()
            || controller.is_broadcast()
            || controller.is_unspecified()
        {
            return Err("UniFi controller must be a private IPv4 LAN address".into());
        }
        if !matches!(port, 443 | 8443) {
            return Err("UniFi controller port must be 443 or 8443".into());
        }
        let prefix = match mode {
            "unifi-os" => "/proxy/network/integration/v1",
            "standalone" => "/integration/v1",
            _ => return Err("UniFi mode must be unifi-os or standalone".into()),
        };
        if !(16..=512).contains(&api_key.len())
            || !api_key
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_./+=:".contains(&c))
        {
            return Err("invalid API key format; use a valid UniFi integration API key".into());
        }
        if let Some(path) = &ca_cert {
            if !path.is_file() {
                return Err("configured UniFi CA certificate file is unavailable".into());
            }
        }
        Ok(Self {
            controller,
            port,
            prefix,
            api_key,
            ca_cert,
        })
    }

    pub fn from_environment() -> Result<Self, String> {
        let controller = env::var("RUSTBOY_UNIFI_HOST")
            .map_err(|_| "RUSTBOY_UNIFI_HOST not configured")?
            .parse::<Ipv4Addr>()
            .map_err(|_| "RUSTBOY_UNIFI_HOST must be a private IPv4 address")?;
        let port = env::var("RUSTBOY_UNIFI_PORT")
            .unwrap_or_else(|_| "443".to_string())
            .parse::<u16>()
            .map_err(|_| "RUSTBOY_UNIFI_PORT must be 443 or 8443")?;
        let mode = env::var("RUSTBOY_UNIFI_MODE").unwrap_or_else(|_| "unifi-os".into());
        let api_key = env::var("RUSTBOY_UNIFI_API_KEY")
            .map_err(|_| "RUSTBOY_UNIFI_API_KEY not configured")?;
        let ca_cert = env::var_os("RUSTBOY_UNIFI_CA_CERT").map(PathBuf::from);
        Self::new(controller, port, &mode, api_key, ca_cert)
    }

    fn get(&self, relative: &str) -> Result<String, String> {
        // The only relative path comes from hardcoded, allowlisted endpoints
        // and a strictly verified UUID. Never accept a user-supplied URL.
        if !relative.starts_with("/sites")
            || relative.contains("..")
            || relative.contains("//")
            || relative.contains('%')
        {
            return Err("invalid UniFi API path".into());
        }
        let url = format!(
            "https://{}:{}{}{}",
            self.controller, self.port, self.prefix, relative
        );
        let mut command = Command::new("curl");
        command.args([
            "--disable",
            "--silent",
            "--show-error",
            "--fail",
            "--proto",
            "=https",
            "--noproxy",
            "*",
            "--connect-timeout",
            "3",
            "--max-time",
            "10",
            "--max-filesize",
            "524288",
            "--request",
            "GET",
            "--header",
            "@-",
            "--header",
            "Accept: application/json",
        ]);
        if let Some(ca) = &self.ca_cert {
            command.arg("--cacert").arg(ca);
        }
        command
            .arg(&url)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .env_remove("RUSTBOY_UNIFI_API_KEY");
        let mut child = command
            .spawn()
            .map_err(|_| "curl unavailable; install a trusted HTTPS curl executable")?;
        let header = format!("X-API-Key: {}\n", self.api_key);
        let Some(mut input) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err("cannot provide UniFi API credentials to HTTPS client".into());
        };
        if input.write_all(header.as_bytes()).is_err() {
            drop(input);
            let _ = child.kill();
            let _ = child.wait();
            return Err("unable to send UniFi authorization header".into());
        }
        drop(input);
        let output = child
            .wait_with_output()
            .map_err(|_| "UniFi read request was interrupted")?;
        if !output.status.success() {
            return Err("UniFi HTTPS GET failed: check reachability, CA trust, API key, and controller version".into());
        }
        if output.stdout.len() > MAX_RESPONSE_BYTES {
            return Err("UniFi response exceeded size limit".into());
        }
        String::from_utf8(output.stdout).map_err(|_| "UniFi response is not UTF-8".into())
    }
}

pub fn valid_site_id(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

fn valid_label(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && s.chars().all(|c| !c.is_control())
}

fn timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[derive(Deserialize)]
struct Page<T> {
    data: Vec<T>,
    count: usize,
    offset: usize,
    limit: usize,
    #[serde(rename = "totalCount")]
    total_count: usize,
}

fn collect_pages<T: DeserializeOwned, F>(
    path: &str,
    maximum: usize,
    mut fetch: F,
) -> Result<Vec<T>, String>
where
    F: FnMut(&str) -> Result<String, String>,
{
    let mut offset = 0;
    let mut expected_total = None;
    let mut items = Vec::new();
    loop {
        let requested = PAGE_SIZE.min(maximum - items.len());
        if requested == 0 {
            return Err("UniFi results exceed inventory limit".into());
        }
        let url = format!("{path}?offset={offset}&limit={requested}");
        let body = fetch(&url)?;
        if body.len() > MAX_RESPONSE_BYTES {
            return Err("UniFi response exceeds size limit".into());
        }
        let page: Page<T> =
            serde_json::from_str(&body).map_err(|_| "invalid UniFi paginated JSON response")?;
        if page.offset != offset
            || (page.limit == 0 && page.total_count != 0)
            || page.limit > requested
            || page.count != page.data.len()
            || page.count > page.limit
            || page.total_count > maximum
            || offset.saturating_add(page.count) > page.total_count
        {
            return Err("inconsistent UniFi pagination metadata".into());
        }
        if expected_total.is_some_and(|total| total != page.total_count) {
            return Err("UniFi inventory changed during paging; retry a fresh snapshot".into());
        }
        expected_total = Some(page.total_count);
        if page.count == 0 && offset < page.total_count {
            return Err("UniFi pagination ended before totalCount".into());
        }
        offset += page.count;
        items.extend(page.data);
        if offset == page.total_count {
            return Ok(items);
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct SiteSource {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UniFiSite {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SiteInventory {
    pub source: String,
    pub observed_unix_ms: u64,
    pub sites: Vec<UniFiSite>,
}

fn read_sites<F: FnMut(&str) -> Result<String, String>>(fetch: F) -> Result<SiteInventory, String> {
    let raw: Vec<SiteSource> = collect_pages("/sites", MAX_SITES, fetch)?;
    let mut sites = Vec::new();
    let mut seen = HashSet::new();
    for entry in raw {
        if !valid_site_id(&entry.id)
            || !valid_label(&entry.name, 96)
            || !seen.insert(entry.id.clone())
        {
            return Err("invalid or duplicate site in UniFi response".into());
        }
        sites.push(UniFiSite {
            id: entry.id,
            name: entry.name,
        });
    }
    Ok(SiteInventory {
        source: "unifi_local_integration_v1".into(),
        observed_unix_ms: timestamp_ms(),
        sites,
    })
}

pub fn list_sites(confirm: bool) -> Result<SiteInventory, String> {
    if !confirm {
        return Err("UniFi inventory requires explicit authorization confirmation".into());
    }
    let cfg = UniFiConfig::from_environment()?;
    read_sites(|path| cfg.get(path))
}

#[derive(Debug, Clone, Deserialize)]
struct ClientSource {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default, rename = "type")]
    kind: Option<String>,
    #[serde(default, rename = "ipAddress")]
    address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UniFiClient {
    pub id: String,
    pub name: String,
    pub connection_type: String,
    pub ip_address: Option<IpAddr>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UniFiInventory {
    pub source: String,
    pub site_id: String,
    pub connected_clients: usize,
    pub clients_with_ip: usize,
    pub observed_unix_ms: u64,
    pub clients: Vec<UniFiClient>,
    pub limitation: String,
}

fn read_clients<F: FnMut(&str) -> Result<String, String>>(
    site: &str,
    fetch: F,
) -> Result<UniFiInventory, String> {
    if !valid_site_id(site) {
        return Err("UniFi site ID must be a canonical UUID".into());
    }
    let path = format!("/sites/{site}/clients");
    let raw: Vec<ClientSource> = collect_pages(&path, MAX_CLIENTS, fetch)?;
    let mut seen = HashSet::new();
    let mut clients = Vec::new();
    for row in raw {
        if !valid_site_id(&row.id) || !seen.insert(row.id.clone()) {
            return Err("invalid or duplicate UniFi client identifier".into());
        }
        let name = row
            .name
            .as_deref()
            .filter(|s| valid_label(s, 96))
            .unwrap_or("Unnamed client")
            .to_string();
        let connection_type = row
            .kind
            .as_deref()
            .filter(|s| valid_label(s, 24))
            .unwrap_or("UNKNOWN")
            .to_string();
        let ip_address = row.address.as_deref().and_then(|s| s.parse().ok());
        clients.push(UniFiClient {
            id: row.id,
            name,
            connection_type,
            ip_address,
        });
    }
    let clients_with_ip = clients.iter().filter(|c| c.ip_address.is_some()).count();
    Ok(UniFiInventory {
        source: "unifi_local_integration_v1".into(),
        site_id: site.into(),
        connected_clients: clients.len(),
        clients_with_ip,
        observed_unix_ms: timestamp_ms(),
        clients,
        limitation: "This is the UniFi controller's connected-client view, not all devices ever seen or all devices in every LAN/VLAN. Clients without an IP cannot be reconciled to a probe. No remote probing or configuration changes were made.".into(),
    })
}

pub fn list_clients(site: &str, confirm: bool) -> Result<UniFiInventory, String> {
    if !confirm {
        return Err("UniFi inventory requires explicit authorization confirmation".into());
    }
    if !valid_site_id(site) {
        return Err("invalid UniFi site ID".into());
    }
    let cfg = UniFiConfig::from_environment()?;
    read_clients(site, |path| cfg.get(path))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Comparison {
    pub controller_clients: usize,
    pub controller_with_ipv4: usize,
    pub matched_active_probe: usize,
    pub controller_not_probed: usize,
    pub active_probe_only: usize,
    pub note: String,
}

pub fn compare_inventory(
    report: &crate::network_probe::NetworkReport,
    inventory: &UniFiInventory,
) -> Comparison {
    let responsive: HashSet<Ipv4Addr> = report
        .devices
        .iter()
        .filter(|d| d.evidence != crate::network_probe::Evidence::NeighborCache)
        .map(|d| d.address)
        .collect();
    let controller_ips: HashSet<Ipv4Addr> = inventory
        .clients
        .iter()
        .filter_map(|c| match c.ip_address {
            Some(IpAddr::V4(ip)) => Some(ip),
            _ => None,
        })
        .collect();
    Comparison {
        controller_clients: inventory.connected_clients,
        controller_with_ipv4: controller_ips.len(),
        matched_active_probe: controller_ips.intersection(&responsive).count(),
        controller_not_probed: controller_ips.difference(&responsive).count(),
        active_probe_only: responsive.difference(&controller_ips).count(),
        note: "Counts use distinct IPv4 addresses for comparison, not unique device identities; NAT, cached clients, VLAN boundaries and controller/probe sample times differ. Matching does not guarantee physical identity.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network_probe::{DeviceObservation, Evidence, NetworkReport};

    const SITE: &str = "00000000-0000-0000-0000-000000000001";

    #[test]
    fn rejects_untrusted_endpoints_and_credentials_before_network() {
        assert!(!valid_site_id("../config"));
        assert!(!valid_site_id("default"));
        assert!(valid_site_id(SITE));
        assert!(UniFiConfig::new(
            "8.8.8.8".parse().unwrap(),
            443,
            "unifi-os",
            "veryvalidapikey123".into(),
            None
        )
        .is_err());
        assert!(UniFiConfig::new(
            "192.168.1.1".parse().unwrap(),
            80,
            "unifi-os",
            "veryvalidapikey123".into(),
            None
        )
        .is_err());
        assert!(UniFiConfig::new(
            "192.168.1.1".parse().unwrap(),
            443,
            "unifi-os",
            "bad\nX-Other: 1".into(),
            None
        )
        .is_err());
        assert!(list_clients("../etc/passwd", false).is_err());
        assert!(list_sites(false).is_err());
    }

    #[test]
    fn multiple_pages_return_exact_connected_clients() {
        let mut calls = 0;
        let inventory = read_clients(SITE, |path| {
            calls += 1;
            if calls == 1 {
                assert!(path.contains("offset=0&limit=200"));
                Ok(r#"{"data":[{"id":"00000000-0000-0000-0000-000000000101","name":"Laptop","type":"WIRELESS","ipAddress":"192.168.1.10"}],"count":1,"offset":0,"limit":200,"totalCount":2}"#.into())
            } else {
                assert!(path.contains("offset=1&limit=200"));
                Ok(r#"{"data":[{"id":"00000000-0000-0000-0000-000000000102","name":"Printer","type":"WIRED"}],"count":1,"offset":1,"limit":200,"totalCount":2}"#.into())
            }
        }).unwrap();
        assert_eq!(inventory.connected_clients, 2);
        assert_eq!(inventory.clients_with_ip, 1);
        assert_eq!(calls, 2);
        assert_eq!(inventory.clients[1].ip_address, None);
    }

    #[test]
    fn malformed_pagination_fails_closed() {
        assert!(read_sites(|_| Ok(
            r#"{"data":[],"count":0,"offset":0,"limit":200,"totalCount":1}"#.into()
        ))
        .is_err());
        assert!(read_sites(|_| Ok(
            r#"{"data":[],"count":0,"offset":1,"limit":200,"totalCount":0}"#.into()
        ))
        .is_err());
        assert!(read_sites(|_| Ok(
            r#"{"data":[],"count":0,"offset":0,"limit":200,"totalCount":4000}"#.into()
        ))
        .is_err());
        let sites = read_sites(|_| Ok(format!(
            r#"{{"data":[{{"id":"{SITE}","name":"Main"}}],"count":1,"offset":0,"limit":200,"totalCount":1}}"#
        ))).unwrap();
        assert_eq!(sites.sites.len(), 1);
    }

    #[test]
    fn reconciliation_does_not_confuse_client_count_with_ip_count() {
        let inventory = read_clients(SITE, |_| Ok(
            r#"{"data":[{"id":"00000000-0000-0000-0000-000000000101","name":"Laptop","ipAddress":"192.168.1.10"},{"id":"00000000-0000-0000-0000-000000000102","name":"Phone","ipAddress":"192.168.1.20"},{"id":"00000000-0000-0000-0000-000000000103","name":"VPN"}],"count":3,"offset":0,"limit":200,"totalCount":3}"#.into()
        )).unwrap();
        let report = NetworkReport {
            scope: "192.168.1.0/24".into(),
            scanned_hosts: 254,
            responding_devices: 2,
            cached_only_devices: 1,
            total_observed_devices: 3,
            elapsed_ms: 10,
            observed_unix_ms: 0,
            devices: vec![
                DeviceObservation {
                    address: "192.168.1.10".parse().unwrap(),
                    evidence: Evidence::IcmpEcho,
                    latency_ms: Some(1.0),
                    icmp_rtt_delta_ms: None,
                    port: None,
                },
                DeviceObservation {
                    address: "192.168.1.30".parse().unwrap(),
                    evidence: Evidence::TcpConnected,
                    latency_ms: Some(4.0),
                    icmp_rtt_delta_ms: None,
                    port: Some(443),
                },
                DeviceObservation {
                    address: "192.168.1.20".parse().unwrap(),
                    evidence: Evidence::NeighborCache,
                    latency_ms: None,
                    icmp_rtt_delta_ms: None,
                    port: None,
                },
            ],
            caveat: "fixture".into(),
        };
        let comparison = compare_inventory(&report, &inventory);
        assert_eq!(comparison.controller_clients, 3);
        assert_eq!(comparison.controller_with_ipv4, 2);
        assert_eq!(comparison.matched_active_probe, 1);
        assert_eq!(comparison.controller_not_probed, 1);
        assert_eq!(comparison.active_probe_only, 1);
    }
}
