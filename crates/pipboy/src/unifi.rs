//! Local-only, read-only UniFi Network Integration API adapter.
//!
//! Deliberate fetch only: an authenticated GET of /sites and, if present,
//! /sites/{uuid}/devices. No cloud path, redirects, shell, hidden proxy,
//! API-key persistence, TLS verification bypass, or writes to the controller.
//! curl is a local prerequisite; secrets pass over its stdin, not argv.
use serde_json::{json, Value};
use std::env;
use std::io::{self, Write};
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const MAX_RESPONSE: usize = 64 * 1024;
const MAX_DEVICES: usize = 50;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn unavailable(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::NotConnected, message)
}

fn valid_name(name: &str) -> bool {
    name.len() <= 253
        && !name.is_empty()
        && name.split('.').all(|label| {
            label.len() <= 63
                && !label.is_empty()
                && label
                    .bytes()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphanumeric())
                && label
                    .bytes()
                    .last()
                    .is_some_and(|c| c.is_ascii_alphanumeric())
                && label
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        })
}
fn uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, ch)| {
            if [8, 13, 18, 23].contains(&i) {
                ch == b'-'
            } else {
                ch.is_ascii_hexdigit()
            }
        })
}
fn safe_label(value: Option<&str>, max: usize) -> String {
    value
        .unwrap_or("UNKNOWN")
        .chars()
        .filter(|c| !c.is_control())
        .take(max)
        .collect::<String>()
}

#[derive(Debug)]
struct Config {
    ip: Ipv4Addr,
    port: u16,
    tls_name: String,
    key: String,
    ca_file: Option<PathBuf>,
    api_base: &'static str,
}
impl Config {
    fn read() -> io::Result<Self> {
        let ip_text = env::var("RUSTBOY_UNIFI_IP")
            .map_err(|_| unavailable("Set RUSTBOY_UNIFI_IP to your local UniFi console IP"))?;
        let key = env::var("RUSTBOY_UNIFI_API_KEY")
            .map_err(|_| unavailable("Set RUSTBOY_UNIFI_API_KEY before connecting"))?;
        let port = env::var("RUSTBOY_UNIFI_PORT").unwrap_or_else(|_| "443".to_string());
        let tls_name = env::var("RUSTBOY_UNIFI_TLS_NAME").unwrap_or_else(|_| ip_text.clone());
        let ca_path = env::var("RUSTBOY_UNIFI_CA_FILE").ok();
        let mode = env::var("RUSTBOY_UNIFI_API_PATH").unwrap_or_else(|_| "os".to_string());
        Self::parse(&ip_text, &key, &port, &tls_name, ca_path.as_deref(), &mode)
    }
    fn parse(
        ip_text: &str,
        key: &str,
        port_text: &str,
        tls_name: &str,
        ca_path: Option<&str>,
        mode: &str,
    ) -> io::Result<Self> {
        let ip: Ipv4Addr = ip_text
            .parse()
            .map_err(|_| invalid("UniFi target must be an IPv4 literal"))?;
        // Never let the web UI or a typo route an API key to the public internet.
        if !(ip.is_private() || ip.is_loopback()) {
            return Err(invalid(
                "UniFi console must have a private or loopback IPv4 address",
            ));
        }
        let port = port_text
            .parse::<u16>()
            .map_err(|_| invalid("Invalid UniFi HTTPS port"))?;
        if port == 0 {
            return Err(invalid("UniFi port must be 1..65535"));
        }
        if tls_name.parse::<Ipv4Addr>().is_err() && !valid_name(tls_name) {
            return Err(invalid("Invalid UniFi TLS certificate hostname"));
        }
        if key.len() < 8
            || key.len() > 512
            || !key.bytes().all(|b| {
                b.is_ascii_alphanumeric()
                    || matches!(b, b'-' | b'_' | b'.' | b'+' | b'/' | b'=' | b':')
            })
        {
            return Err(invalid(
                "Invalid API key format; only printable header-safe bytes accepted",
            ));
        }
        let api_base = match mode {
            "os" => "/proxy/network/integration/v1",
            "direct" => "/integration/v1",
            _ => return Err(invalid("API path must be os or direct")),
        };
        let ca_file = ca_path.map(PathBuf::from);
        if ca_file.as_ref().is_some_and(|p| !p.is_absolute()) {
            return Err(invalid("UniFi CA file must be an absolute path"));
        }
        Ok(Self {
            ip,
            port,
            tls_name: tls_name.to_owned(),
            key: key.to_owned(),
            ca_file,
            api_base,
        })
    }

    fn fetch(&self, endpoint: &str) -> io::Result<Value> {
        if !matches!(endpoint, "sites?limit=25&offset=0")
            && !(endpoint.starts_with("sites/")
                && endpoint.ends_with("/devices?limit=50&offset=0")
                && endpoint
                    .strip_prefix("sites/")
                    .and_then(|rest| rest.strip_suffix("/devices?limit=50&offset=0"))
                    .is_some_and(uuid))
        {
            return Err(invalid("UniFi API endpoint not allowlisted"));
        }
        let url = format!(
            "https://{}:{}{}/{}",
            self.tls_name, self.port, self.api_base, endpoint,
        );
        // Fixed binary and fixed argv: no shell interpolation or untrusted URL.
        // API key is passed through a validated curl config on stdin only.
        let mut command = Command::new("curl");
        command.args([
            "--silent",
            "--fail",
            "--request",
            "GET",
            "--proto",
            "=https",
            "--noproxy",
            "*",
            "--connect-timeout",
            "2",
            "--max-time",
            "4",
            "--max-filesize",
            "65536",
            "--max-redirs",
            "0",
            "--resolve",
            &format!("{}:{}:{}", self.tls_name, self.port, self.ip),
            "--url",
            &url,
            "--config",
            "-",
        ]);
        if let Some(ca) = &self.ca_file {
            command.arg("--cacert").arg(ca);
        }
        command
            .env_remove("RUSTBOY_UNIFI_API_KEY")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command
            .spawn()
            .map_err(|_| unavailable("curl is required to read UniFi"))?;
        if let Some(mut input) = child.stdin.take() {
            // Quotes/line breaks are excluded at Config::parse; no CRLF injection.
            let config = format!(
                "header = \"X-API-Key: {}\"\nheader = \"Accept: application/json\"\n",
                self.key
            );
            input.write_all(config.as_bytes())?;
        }
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(unavailable("UniFi HTTPS GET failed: check API key, TLS certificate, controller version and route"));
        }
        if output.stdout.len() > MAX_RESPONSE {
            return Err(invalid("UniFi response exceeds local limit"));
        }
        serde_json::from_slice::<Value>(&output.stdout)
            .map_err(|_| invalid("Invalid UniFi API JSON response"))
    }
}

pub fn configured() -> bool {
    env::var_os("RUSTBOY_UNIFI_IP").is_some() && env::var_os("RUSTBOY_UNIFI_API_KEY").is_some()
}

/// A manual, read-only 2-request snapshot. No background work, persistence
/// or account/cloud access. Untrusted API strings are trimmed and sanitized.
pub fn snapshot(requested_site: Option<&str>) -> io::Result<Value> {
    let config = Config::read()?;
    let sites = config.fetch("sites?limit=25&offset=0")?;
    let all_sites = page(&sites, 25)?;
    let mut visible_sites = Vec::new();
    for raw in all_sites {
        let Some(id) = raw.get("id").and_then(Value::as_str).filter(|id| uuid(id)) else {
            continue;
        };
        visible_sites.push(json!({
            "id":id,
            "name":safe_label(raw.get("name").and_then(Value::as_str), 48)
        }));
    }
    let site = match requested_site {
        Some(id) => {
            if !uuid(id) {
                return Err(invalid("Invalid site UUID"));
            }
            visible_sites
                .iter()
                .find(|item| item["id"] == id)
                .ok_or_else(|| invalid("Site must be in the first 25 local sites"))?
        }
        None => match visible_sites.first() {
            Some(site) => site,
            None => {
                return Ok(json!({
                    "kind":"unifi", "site":null, "sites":[], "devices":[],
                    "devices_listed":0, "devices_total":0, "online_on_page":0,
                    "offline_on_page":0, "output":"UNIFI / LOCAL API\nConnected; no sites returned.\nRead-only; no devices changed."
                }))
            }
        },
    };
    let site_id = site["id"]
        .as_str()
        .ok_or_else(|| invalid("Missing site ID"))?;
    let site_name = site["name"].as_str().unwrap_or("UNKNOWN");
    let endpoint = format!("sites/{site_id}/devices?limit=50&offset=0");
    let devices = config.fetch(&endpoint)?;
    let entries = page(&devices, MAX_DEVICES)?;
    let total = devices["totalCount"]
        .as_u64()
        .unwrap_or(entries.len() as u64);
    let mut items = Vec::new();
    let mut online = 0usize;
    let mut offline = 0usize;
    for raw in entries {
        let state = safe_label(raw.get("state").and_then(Value::as_str), 24);
        if state.eq_ignore_ascii_case("ONLINE") {
            online += 1;
        }
        if state.eq_ignore_ascii_case("OFFLINE") {
            offline += 1;
        }
        let ip = raw
            .get("ipAddress")
            .and_then(Value::as_str)
            .filter(|value| value.parse::<std::net::IpAddr>().is_ok())
            .unwrap_or("");
        items.push(json!({
            "name":safe_label(raw.get("name").and_then(Value::as_str), 44),
            "model":safe_label(raw.get("model").and_then(Value::as_str), 30),
            "state":state,
            "ip":ip
        }));
    }
    let output = format!(
        "UNIFI / LOCAL READ-ONLY\nSite: {site_name}\nSites listed: {}\nDevices: {} shown of {} total\n\
         Online (shown): {online}\nOffline (shown): {offline}\nNo changes applied. \
         Counts are limited to the fetched page; not a link latency or packet-loss test.",
        visible_sites.len(), items.len(), total
    );
    Ok(json!({
        "kind":"unifi",
        "sites":visible_sites,
        "site":site,
        "devices":items,
        "devices_listed":items.len(),
        "devices_total":total,
        "online_on_page":online,
        "offline_on_page":offline,
        "output":output
    }))
}

fn page(value: &Value, max: usize) -> io::Result<&[Value]> {
    let entries = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("Unexpected UniFi API page: missing data array"))?;
    if entries.len() > max {
        return Err(invalid("UniFi API returned more results than requested"));
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_local_tls_configuration_and_secret_validation() {
        let absolute_ca = if cfg!(windows) {
            "C:\\unifi-ca.pem"
        } else {
            "/tmp/unifi-ca.pem"
        };
        assert!(Config::parse(
            "192.168.1.1",
            "abcd-01234567890",
            "443",
            "unifi.example.net",
            Some(absolute_ca),
            "os"
        )
        .is_ok());
        assert!(Config::parse(
            "10.1.1.1",
            "abcd-01234567890",
            "8443",
            "10.1.1.1",
            None,
            "direct"
        )
        .is_ok());
        for ip in ["8.8.8.8", "169.254.10.1", "224.0.0.1", "unifi.example"] {
            assert!(Config::parse(ip, "abcdefghijk", "443", "unifi.local", None, "os").is_err());
        }
        for key in ["unsafe\nheader", "bad\"quote", "abc", "invalid\\slash"] {
            assert!(Config::parse("192.168.1.1", key, "443", "unifi.local", None, "os").is_err());
        }
        assert!(Config::parse(
            "192.168.1.1",
            "abcdefghijk",
            "443",
            "evil.com/x",
            None,
            "os"
        )
        .is_err());
        assert!(Config::parse(
            "192.168.1.1",
            "abcdefghijk",
            "443",
            "unifi.local",
            None,
            "cloud"
        )
        .is_err());
    }

    #[test]
    fn uuid_paths_and_response_bounds_are_strict() {
        assert!(uuid("01234567-89ab-cdef-0123-456789abcdef"));
        assert!(!uuid("../../../admin"));
        assert!(page(&json!({"data":[{"id":"a"}]}), 25).is_ok());
        assert!(page(&json!({"foo":[]}), 25).is_err());
        assert!(page(&json!({"data":[0,1,2]}), 2).is_err());
        assert_eq!(safe_label(Some("A\nB"), 16), "AB");
    }
}
