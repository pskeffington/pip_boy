//! Explicit, bounded LAN discovery using ICMP echo and TCP connection checks.
//! No scan occurs on module load, profile selection, or dashboard navigation.
//! Scopes must be RFC1918 IPv4 /24 to /32 and operator-confirmed.
//! Detected hosts are not an authoritative inventory.

use serde::{Deserialize, Serialize};
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Mutex,
};

static SWEEP_ACTIVE: AtomicBool = AtomicBool::new(false);
struct SweepLease;
impl Drop for SweepLease {
    fn drop(&mut self) {
        SWEEP_ACTIVE.store(false, Ordering::SeqCst);
    }
}
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const MAX_TARGETS: usize = 254;
const WORKERS: usize = 24;
const TCP_PORTS: [u16; 5] = [22, 80, 443, 445, 8080];
const TCP_TIMEOUT: Duration = Duration::from_millis(140);
const PING_DEADLINE: Duration = Duration::from_millis(1100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkScope {
    network: Ipv4Addr,
    prefix: u8,
}
impl NetworkScope {
    pub fn parse(input: &str) -> Result<Self, String> {
        let (ip, prefix) = if input.trim() == "local" {
            (
                local_private_ip()
                    .ok_or("no private LAN route found; enter an explicit /24..../32 CIDR")?,
                24,
            )
        } else {
            let (ip, bits) = input
                .trim()
                .split_once('/')
                .ok_or("enter local or an IPv4 CIDR")?;
            (
                ip.parse::<Ipv4Addr>().map_err(|_| "invalid IPv4 address")?,
                bits.parse::<u8>().map_err(|_| "invalid IPv4 prefix")?,
            )
        };
        if !(24..=32).contains(&prefix) {
            return Err("active discovery is limited to /24..../32 (max 254 hosts)".into());
        }
        let mask = u32::MAX << (32 - prefix);
        let start = u32::from(ip) & mask;
        let end = start | !mask;
        if !Ipv4Addr::from(start).is_private() || !Ipv4Addr::from(end).is_private() {
            return Err("scope must stay entirely within RFC1918 private IPv4 space".into());
        }
        Ok(Self {
            network: start.into(),
            prefix,
        })
    }
    pub fn cidr(self) -> String {
        format!("{}/{}", self.network, self.prefix)
    }
    pub fn targets(self) -> Vec<Ipv4Addr> {
        let start = u32::from(self.network);
        let end = start | !(u32::MAX << (32 - self.prefix));
        let range = match self.prefix {
            32 => start..=start,
            31 => start..=end,
            _ => (start + 1)..=(end - 1),
        };
        range.map(Ipv4Addr::from).collect()
    }
    fn contains(self, ip: Ipv4Addr) -> bool {
        let mask = u32::MAX << (32 - self.prefix);
        (u32::from(ip) & mask) == u32::from(self.network)
    }
}

// UDP connect chooses a local route without sending a datagram. /24 is only a
// conservative suggestion, not a discovered interface mask.
fn local_private_ip() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).ok()?;
    let IpAddr::V4(ip) = socket.local_addr().ok()?.ip() else {
        return None;
    };
    ip.is_private().then_some(ip)
}
pub fn suggested_scope() -> Option<String> {
    NetworkScope::parse("local").ok().map(NetworkScope::cidr)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Evidence {
    IcmpEcho,
    TcpConnected,
    TcpRejected,
    NeighborCache,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceObservation {
    pub address: Ipv4Addr,
    pub evidence: Evidence,
    // ICMP RTT or TCP connection/refusal response time, depending on evidence.
    pub latency_ms: Option<f64>,
    // Absolute difference between two successful ICMP RTTs, not statistical jitter.
    pub icmp_rtt_delta_ms: Option<f64>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkReport {
    pub scope: String,
    pub scanned_hosts: usize,
    pub responding_devices: usize,
    pub cached_only_devices: usize,
    pub total_observed_devices: usize,
    pub elapsed_ms: u64,
    pub observed_unix_ms: u64,
    pub devices: Vec<DeviceObservation>,
    pub caveat: String,
}
impl NetworkReport {
    pub fn render_text(&self) -> String {
        use std::fmt::Write as _;
        let mut text = format!(
            "NET ENGINEER / LIVE PRIVATE LAN\nSCOPE: {}\nSCANNED: {}  RESPONDING: {}  CACHE-ONLY: {}  OBSERVED: {}\nSWEEP: {} ms\n",
            self.scope, self.scanned_hosts, self.responding_devices,
            self.cached_only_devices, self.total_observed_devices, self.elapsed_ms);
        for device in &self.devices {
            let kind = match device.evidence {
                Evidence::IcmpEcho => "ICMP",
                Evidence::TcpConnected => "TCP OPEN",
                Evidence::TcpRejected => "TCP REFUSED",
                Evidence::NeighborCache => "NEIGHBOR CACHE",
            };
            let latency = device
                .latency_ms
                .map(|ms| format!("{ms:.2}ms"))
                .unwrap_or_else(|| "--".to_string());
            let port = device.port.map(|p| format!(" :{p}")).unwrap_or_default();
            let _ = writeln!(text, "{}  {}{}  {}", device.address, kind, port, latency);
            if let Some(delta) = device.icmp_rtt_delta_ms {
                let _ = writeln!(text, "  ICMP 2-sample RTT delta: {delta:.2}ms");
            }
        }
        if self.responding_devices == 0 {
            text.push_str("NO ACTIVE RESPONDERS: this does not prove the network is empty.\n");
        }
        text.push_str(&self.caveat);
        text
    }
}

fn parse_ping_rtt(output: &str) -> Option<f64> {
    let start = output.find("time=").or_else(|| output.find("time<"))? + 5;
    let suffix = &output[start..];
    let end = suffix.find(|c: char| !c.is_ascii_digit() && c != '.')?;
    let number = suffix[..end].parse::<f64>().ok()?;
    number.is_finite().then_some(number)
}

fn ping_once(address: Ipv4Addr) -> Option<f64> {
    let mut cmd = Command::new("ping");
    #[cfg(target_os = "windows")]
    cmd.args(["-n", "1", "-w", "900"]);
    #[cfg(target_os = "macos")]
    cmd.args(["-n", "-c", "1", "-W", "900"]);
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    cmd.args(["-n", "-c", "1", "-W", "1"]);
    let mut child = cmd
        .arg(address.to_string())
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output().ok()?;
                return status
                    .success()
                    .then(|| parse_ping_rtt(&String::from_utf8_lossy(&output.stdout)))
                    .flatten();
            }
            Ok(None) if started.elapsed() < PING_DEADLINE => {
                std::thread::sleep(Duration::from_millis(15));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

fn probe_tcp(address: Ipv4Addr, port: u16) -> Option<(Evidence, f64)> {
    let start = Instant::now();
    let addr = SocketAddr::from((address, port));
    let result = TcpStream::connect_timeout(&addr, TCP_TIMEOUT);
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    match result {
        Ok(stream) => {
            drop(stream);
            Some((Evidence::TcpConnected, elapsed))
        }
        Err(error) if error.kind() == io::ErrorKind::ConnectionRefused => {
            Some((Evidence::TcpRejected, elapsed))
        }
        _ => None,
    }
}

fn probe(address: Ipv4Addr) -> Option<DeviceObservation> {
    if let Some(first) = ping_once(address) {
        let delta = ping_once(address).map(|second| (second - first).abs());
        return Some(DeviceObservation {
            address,
            evidence: Evidence::IcmpEcho,
            latency_ms: Some(first),
            icmp_rtt_delta_ms: delta,
            port: None,
        });
    }
    for port in TCP_PORTS {
        if let Some((evidence, latency)) = probe_tcp(address, port) {
            return Some(DeviceObservation {
                address,
                evidence,
                latency_ms: Some(latency),
                icmp_rtt_delta_ms: None,
                port: Some(port),
            });
        }
    }
    None
}

fn run_parallel<F>(targets: &[Ipv4Addr], probe_fn: F) -> Vec<DeviceObservation>
where
    F: Fn(Ipv4Addr) -> Option<DeviceObservation> + Sync,
{
    let index = AtomicUsize::new(0);
    let results = Mutex::new(Vec::<DeviceObservation>::new());
    std::thread::scope(|scope| {
        for _ in 0..WORKERS.min(targets.len()) {
            let index = &index;
            let results = &results;
            let probe_fn = &probe_fn;
            scope.spawn(move || loop {
                let i = index.fetch_add(1, Ordering::Relaxed);
                let Some(ip) = targets.get(i) else {
                    break;
                };
                if let Some(found) = probe_fn(*ip) {
                    results.lock().expect("network scan lock").push(found);
                }
            });
        }
    });
    let mut observed = results.into_inner().expect("network scan lock");
    observed.sort_by_key(|device| u32::from(device.address));
    observed
}

fn parse_neighbor_ips(text: &str, scope: NetworkScope) -> Vec<Ipv4Addr> {
    let mut output = Vec::new();
    for line in text.lines() {
        if line.contains("FAILED")
            || line.contains("INCOMPLETE")
            || line.contains("(incomplete)")
            || line.contains("00:00:00:00:00:00")
        {
            continue;
        }
        if line.contains("0x") {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 4 && fields[2] != "0x2" && fields[2] != "0x6" {
                continue;
            }
        }
        for part in line.split_whitespace() {
            let token = part.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
            if let Ok(ip) = token.parse::<Ipv4Addr>() {
                if scope.contains(ip) && !output.contains(&ip) {
                    output.push(ip);
                }
                break;
            }
        }
    }
    output
}

fn neighbors(scope: NetworkScope) -> Vec<Ipv4Addr> {
    #[cfg(target_os = "linux")]
    if let Ok(text) = std::fs::read_to_string("/proc/net/arp") {
        return parse_neighbor_ips(&text, scope);
    }
    let mut command = Command::new("arp");
    #[cfg(target_os = "windows")]
    command.arg("-a");
    #[cfg(not(target_os = "windows"))]
    command.arg("-an");
    command
        .stdin(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success() && o.stdout.len() < 256 * 1024)
        .map(|o| parse_neighbor_ips(&String::from_utf8_lossy(&o.stdout), scope))
        .unwrap_or_default()
}

/// Explicit, one-shot scan. Consent is checked before any ping or socket.
pub fn scan(cidr: &str, confirm_authorized: bool) -> Result<NetworkReport, String> {
    if !confirm_authorized {
        return Err("explicit authorization acknowledgment is required to scan a LAN".into());
    }
    let scope = NetworkScope::parse(cidr)?;
    let targets = scope.targets();
    if targets.is_empty() || targets.len() > MAX_TARGETS {
        return Err("invalid scan size".into());
    }
    SWEEP_ACTIVE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map_err(|_| "another LAN sweep is already running")?;
    let _lease = SweepLease;
    let started = Instant::now();
    let mut devices = run_parallel(&targets, probe);
    let responding = devices.len();
    for ip in neighbors(scope) {
        if targets.contains(&ip) && !devices.iter().any(|d| d.address == ip) {
            devices.push(DeviceObservation {
                address: ip,
                evidence: Evidence::NeighborCache,
                latency_ms: None,
                icmp_rtt_delta_ms: None,
                port: None,
            });
        }
    }
    devices.sort_by_key(|d| u32::from(d.address));
    let cached = devices.len() - responding;
    let observed_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX);
    Ok(NetworkReport {
        scope: scope.cidr(),
        scanned_hosts: targets.len(),
        responding_devices: responding,
        cached_only_devices: cached,
        total_observed_devices: devices.len(),
        elapsed_ms: started.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
        observed_unix_ms,
        devices,
        caveat: "DETECTED != ALL CONNECTED. ICMP filtering, sleeping clients and VLANs hide devices. Cached ARP entries may be stale. TCP refusals are weaker evidence than ICMP replies; latencies are probe-specific and two-ping deltas are not statistical jitter. A controller integration is required for managed-client inventory. No external Internet addresses or credentials were accessed.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_scopes_are_bounded() {
        assert_eq!(
            NetworkScope::parse("192.168.10.55/24").unwrap().cidr(),
            "192.168.10.0/24"
        );
        assert_eq!(
            NetworkScope::parse("10.1.1.23/32").unwrap().targets().len(),
            1
        );
        assert_eq!(
            NetworkScope::parse("172.16.2.10/31")
                .unwrap()
                .targets()
                .len(),
            2
        );
        assert_eq!(
            NetworkScope::parse("192.168.1.0/24")
                .unwrap()
                .targets()
                .len(),
            254
        );
        for invalid in [
            "",
            "127.0.0.1/32",
            "8.8.8.8/32",
            "169.254.0.1/24",
            "192.168.1.0/23",
            "10.0.0.0/8",
            "192.168.0.0/0",
            "192.168.1.0/33",
            "192.168.1.1",
            "localhost/24",
        ] {
            assert!(NetworkScope::parse(invalid).is_err(), "{invalid}");
        }
    }
    #[test]
    fn ping_samples_are_parsed() {
        assert_eq!(parse_ping_rtt("64 bytes time=0.41 ms ttl=64"), Some(0.41));
        assert_eq!(
            parse_ping_rtt("Reply: bytes=32 time<1ms TTL=128"),
            Some(1.0)
        );
        assert_eq!(parse_ping_rtt("100% packet loss"), None);
    }
    #[test]
    fn simulated_probe_results_are_distinct_and_sorted() {
        let targets = NetworkScope::parse("192.168.2.0/29").unwrap().targets();
        let results = run_parallel(&targets, |ip| {
            if ip.octets()[3] % 2 == 0 {
                Some(DeviceObservation {
                    address: ip,
                    evidence: Evidence::IcmpEcho,
                    latency_ms: Some(0.0),
                    icmp_rtt_delta_ms: Some(0.0),
                    port: None,
                })
            } else {
                None
            }
        });
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].address, Ipv4Addr::new(192, 168, 2, 2));
    }
    #[test]
    fn neighbor_cache_not_equal_to_online() {
        let scope = NetworkScope::parse("192.168.1.0/24").unwrap();
        let text = "Address HWtype Flags HWaddress Mask Device\n\
            192.168.1.2 0x1 0x2 aa:bb:cc:dd:ee:ff * eth0\n\
            192.168.1.3 0x1 0x0 00:00:00:00:00:00 * eth0\n\
            ? (192.168.1.4) at (incomplete) on en0\n\
            ? (192.168.1.5) at 11:22:33:44:55:66 on en0\n";
        assert_eq!(
            parse_neighbor_ips(text, scope),
            vec![Ipv4Addr::new(192, 168, 1, 2), Ipv4Addr::new(192, 168, 1, 5)]
        );
    }
    #[test]
    fn loopback_tcp_listener_is_actually_detected() {
        let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let (evidence, rtt) = probe_tcp(Ipv4Addr::LOCALHOST, port).unwrap();
        assert_eq!(evidence, Evidence::TcpConnected);
        assert!(rtt >= 0.0);
    }
    #[test]
    fn no_ack_means_no_probe() {
        assert!(scan("192.168.1.0/24", false).is_err());
    }
}
