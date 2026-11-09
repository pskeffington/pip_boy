//! Rust-Boy network engineering bench: deterministic, bounded, offline math.
//!
//! These operations never open a socket or inspect the host's network state.
//! Explicit single-endpoint TCP reachability lives in the binary, separately
//! gated by an operator confirmation. Network plans do not change OS settings.
use serde::Serialize;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

const MAX_INPUT: usize = 256;
const MAX_EXTRA: usize = 1024;
const MAX_ROUTE_ENTRIES: usize = 16;
const MAX_SPLIT_PREVIEW: u64 = 12;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Operation {
    pub id: &'static str,
    pub name: &'static str,
    pub input_label: &'static str,
    pub example: &'static str,
    pub extra_label: Option<&'static str>,
    pub extra_example: Option<&'static str>,
    pub active: bool,
}

pub const OPERATIONS: &[Operation] = &[
    Operation {
        id: "scan",
        name: "SCAN LAN",
        input_label: "PRIVATE CIDR / LOCAL",
        example: "local",
        extra_label: None,
        extra_example: None,
        active: true,
    },
    Operation {
        id: "unifi-sites",
        name: "UNIFI SITES",
        input_label: "",
        example: "",
        extra_label: None,
        extra_example: None,
        active: true,
    },
    Operation {
        id: "unifi-clients",
        name: "UNIFI CLIENTS",
        input_label: "SITE UUID",
        example: "",
        extra_label: None,
        extra_example: None,
        active: true,
    },
    Operation {
        id: "unifi-compare",
        name: "UNIFI / LAN",
        input_label: "PRIVATE CIDR / LOCAL",
        example: "local",
        extra_label: Some("SITE UUID"),
        extra_example: Some(""),
        active: true,
    },
    Operation {
        id: "icmp",
        name: "PING / JITTER",
        input_label: "GATEWAY / TARGET IP",
        example: "192.168.1.1",
        extra_label: Some("SAMPLES (3-8)"),
        extra_example: Some("5"),
        active: true,
    },
    Operation {
        id: "health",
        name: "NETWORK HEALTH",
        input_label: "TARGET IP",
        example: "192.168.1.1",
        extra_label: Some("TCP PORT"),
        extra_example: Some("443"),
        active: true,
    },
    Operation {
        id: "cidr4",
        name: "IPv4 CIDR",
        input_label: "IPv4 / PREFIX",
        example: "192.168.1.50/24",
        extra_label: None,
        extra_example: None,
        active: false,
    },
    Operation {
        id: "cidr6",
        name: "IPv6 CIDR",
        input_label: "IPv6 / PREFIX",
        example: "2001:db8:1::abcd/64",
        extra_label: None,
        extra_example: None,
        active: false,
    },
    Operation {
        id: "split4",
        name: "SUBNET SPLIT",
        input_label: "PARENT IPv4 / PREFIX",
        example: "10.40.0.0/24",
        extra_label: Some("NEW PREFIX"),
        extra_example: Some("26"),
        active: false,
    },
    Operation {
        id: "route",
        name: "ROUTE MATCH",
        input_label: "DESTINATION IP",
        example: "10.40.1.17",
        extra_label: Some("PREFIXES (COMMA SEPARATED)"),
        extra_example: Some("0.0.0.0/0,10.0.0.0/8,10.40.0.0/16"),
        active: false,
    },
    Operation {
        id: "aggregate4",
        name: "CIDR MERGE",
        input_label: "FIRST IPv4 / PREFIX",
        example: "10.40.0.0/24",
        extra_label: Some("SECOND IPv4 / PREFIX"),
        extra_example: Some("10.40.1.0/24"),
        active: false,
    },
    Operation {
        id: "overlap4",
        name: "OVERLAP",
        input_label: "FIRST IPv4 / PREFIX",
        example: "10.40.0.0/16",
        extra_label: Some("SECOND IPv4 / PREFIX"),
        extra_example: Some("10.40.1.0/24"),
        active: false,
    },
    Operation {
        id: "mac",
        name: "MAC / EUI-64",
        input_label: "MAC ADDRESS",
        example: "00:1a:2b:3c:4d:5e",
        extra_label: None,
        extra_example: None,
        active: false,
    },
    Operation {
        id: "vlan",
        name: "802.1Q VLAN",
        input_label: "VID, PCP, DEI",
        example: "42,5,0",
        extra_label: None,
        extra_example: None,
        active: false,
    },
    Operation {
        id: "mtu",
        name: "MTU / MSS",
        input_label: "IP MTU (BYTES)",
        example: "1500",
        extra_label: None,
        extra_example: None,
        active: false,
    },
    Operation {
        id: "tcp",
        name: "TCP CHECK",
        input_label: "LITERAL IP",
        example: "127.0.0.1",
        extra_label: Some("SINGLE TCP PORT"),
        extra_example: Some("8080"),
        active: true,
    },
];

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn bounded(value: &str, max: usize) -> io::Result<&str> {
    if value.is_empty()
        || value.len() > max
        || !value.is_ascii()
        || value.bytes().any(|c| c.is_ascii_control())
        || value.trim() != value
    {
        return Err(invalid("Invalid or oversized network bench input"));
    }
    Ok(value)
}

fn number_u8(value: &str, maximum: u8) -> io::Result<u8> {
    if value.is_empty() || !value.bytes().all(|c| c.is_ascii_digit()) {
        return Err(invalid("Expected unsigned decimal number"));
    }
    let number = value
        .parse::<u8>()
        .map_err(|_| invalid("Invalid decimal number"))?;
    if number > maximum {
        return Err(invalid("Numeric value exceeds range"));
    }
    Ok(number)
}

#[derive(Clone, Copy, Debug)]
struct V4 {
    network: u32,
    prefix: u8,
    mask: u32,
}

impl V4 {
    fn text(self) -> String {
        format!("{}/{}", Ipv4Addr::from(self.network), self.prefix)
    }
    fn contains(self, ip: u32) -> bool {
        ip & self.mask == self.network
    }
}

fn v4(value: &str) -> io::Result<V4> {
    let value = bounded(value, MAX_INPUT)?;
    let (addr, bits) = value
        .split_once('/')
        .ok_or_else(|| invalid("Expected IPv4/prefix"))?;
    let ip: Ipv4Addr = addr.parse().map_err(|_| invalid("Invalid IPv4 address"))?;
    let prefix = number_u8(bits, 32)?;
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    Ok(V4 {
        network: u32::from(ip) & mask,
        mask,
        prefix,
    })
}

#[derive(Clone, Copy, Debug)]
struct V6 {
    network: u128,
    prefix: u8,
    mask: u128,
}

impl V6 {
    fn text(self) -> String {
        format!("{}/{}", Ipv6Addr::from(self.network), self.prefix)
    }
    fn contains(self, ip: u128) -> bool {
        ip & self.mask == self.network
    }
}

fn v6(value: &str) -> io::Result<V6> {
    let value = bounded(value, MAX_INPUT)?;
    let (addr, bits) = value
        .split_once('/')
        .ok_or_else(|| invalid("Expected IPv6/prefix"))?;
    let ip: Ipv6Addr = addr.parse().map_err(|_| invalid("Invalid IPv6 address"))?;
    let prefix = number_u8(bits, 128)?;
    let mask = if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - prefix)
    };
    Ok(V6 {
        network: u128::from(ip) & mask,
        mask,
        prefix,
    })
}

fn cidr6(input: &str) -> io::Result<String> {
    let net = v6(input)?;
    let supplied = input
        .split_once('/')
        .map(|(addr, _)| addr)
        .unwrap_or_default();
    let end = net.network | !net.mask;
    Ok(format!(
        "IPv6 / OFFLINE\nInput: {supplied}\nNetwork: {}\nRange: {} .. {}\nAddresses: 2^{}\nIPv6 has no broadcast address.\nNo packets sent.",
        net.text(), Ipv6Addr::from(net.network), Ipv6Addr::from(end), 128 - net.prefix,
    ))
}

fn split4(input: &str, child_text: &str) -> io::Result<String> {
    let parent = v4(input)?;
    let child = number_u8(bounded(child_text, 3)?, 32)?;
    if child < parent.prefix {
        return Err(invalid("New prefix must be at least the parent prefix"));
    }
    let count = 1u64 << (child - parent.prefix);
    let step = 1u64 << (32 - child);
    let mut output = format!(
        "SUBNET SPLIT / OFFLINE\nParent: {}\nNew prefix: /{child}\nTotal subnets: {count}\n",
        parent.text()
    );
    for n in 0..count.min(MAX_SPLIT_PREVIEW) {
        let address = u64::from(parent.network) + step * n;
        output.push_str(&format!(
            "{:>2}: {}/{}\n",
            n + 1,
            Ipv4Addr::from(address as u32),
            child,
        ));
    }
    if count > MAX_SPLIT_PREVIEW {
        output.push_str(&format!(
            "... {} more (preview limit 12)\n",
            count - MAX_SPLIT_PREVIEW
        ));
    }
    output.push_str("No network changes made.");
    Ok(output)
}

fn route(input: &str, extra: &str) -> io::Result<String> {
    let target: IpAddr = bounded(input, 64)?
        .parse()
        .map_err(|_| invalid("Enter a literal IPv4 or IPv6 destination"))?;
    let entries = bounded(extra, MAX_EXTRA)?.split(',');
    let entry_count = entries.clone().count();
    if entry_count == 0 || entry_count > MAX_ROUTE_ENTRIES {
        return Err(invalid("Specify 1..16 comma-separated route prefixes"));
    }
    let mut best: Option<(u8, String)> = None;
    let mut matches = 0usize;
    for prefix in entries {
        let prefix = prefix.trim();
        let (bits, value) = match target {
            IpAddr::V4(ip) => {
                let net = v4(prefix)?;
                (
                    net.prefix,
                    (net.contains(u32::from(ip))).then(|| net.text()),
                )
            }
            IpAddr::V6(ip) => {
                let net = v6(prefix)?;
                (
                    net.prefix,
                    (net.contains(u128::from(ip))).then(|| net.text()),
                )
            }
        };
        if let Some(network) = value {
            matches += 1;
            if best.as_ref().is_none_or(|(previous, _)| bits > *previous) {
                best = Some((bits, network));
            }
        }
    }
    Ok(format!(
        "ROUTE MATCH / OFFLINE\nDestination: {target}\nEvaluated: {} prefixes\nMatching: {matches}\nLongest prefix: {}\nThis is a what-if comparison, not an OS routing-table lookup.",
        entry_count,
        best.map(|(_, route)| route).unwrap_or_else(|| "NONE".to_string())
    ))
}

fn aggregate4(input: &str, other: &str) -> io::Result<String> {
    let a = v4(input)?;
    let b = v4(other)?;
    if a.prefix != b.prefix || a.prefix == 0 || a.network == b.network {
        return Err(invalid(
            "CIDR merge needs two distinct same-length IPv4 siblings",
        ));
    }
    let parent_bits = a.prefix - 1;
    let parent_mask = if parent_bits == 0 {
        0
    } else {
        u32::MAX << (32 - parent_bits)
    };
    if a.network & parent_mask != b.network & parent_mask {
        return Err(invalid(
            "CIDR networks are not siblings under a single parent",
        ));
    }
    let parent = Ipv4Addr::from(a.network & parent_mask);
    Ok(format!(
        "CIDR MERGE / OFFLINE\n{} + {}\nExact aggregate: {parent}/{parent_bits}\nNo additional addresses covered.",
        a.text(), b.text(),
    ))
}

fn overlap4(input: &str, other: &str) -> io::Result<String> {
    let a = v4(input)?;
    let b = v4(other)?;
    let overlap = a.contains(b.network) || b.contains(a.network);
    let covered = if overlap {
        if a.prefix >= b.prefix {
            a.text()
        } else {
            b.text()
        }
    } else {
        "NONE".to_string()
    };
    Ok(format!(
        "OVERLAP / OFFLINE\nA: {}\nB: {}\nOverlap: {}\nIntersection: {covered}\nNo packets sent.",
        a.text(),
        b.text(),
        if overlap { "YES" } else { "NO" },
    ))
}

fn mac(input: &str) -> io::Result<String> {
    let input = bounded(input, 32)?;
    let normalized = if input.contains(':') {
        let parts = input.split(':').collect::<Vec<_>>();
        if parts.len() != 6 || parts.iter().any(|p| p.len() != 2) {
            return Err(invalid("MAC requires six hexadecimal octets"));
        }
        parts.concat()
    } else if input.contains('-') {
        let parts = input.split('-').collect::<Vec<_>>();
        if parts.len() != 6 || parts.iter().any(|p| p.len() != 2) {
            return Err(invalid("MAC requires six hexadecimal octets"));
        }
        parts.concat()
    } else if input.contains('.') {
        let parts = input.split('.').collect::<Vec<_>>();
        if parts.len() != 3 || parts.iter().any(|p| p.len() != 4) {
            return Err(invalid("MAC requires three 4-digit hexadecimal groups"));
        }
        parts.concat()
    } else {
        input.to_string()
    };
    if normalized.len() != 12 || !normalized.bytes().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(invalid(
            "MAC must contain exactly twelve hexadecimal digits",
        ));
    }
    let mut octets = [0u8; 6];
    for (i, byte) in octets.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&normalized[i * 2..i * 2 + 2], 16)
            .map_err(|_| invalid("Invalid MAC octet"))?;
    }
    let group = octets[0] & 0x01 != 0;
    let local = octets[0] & 0x02 != 0;
    let mut output = format!(
        "MAC / OFFLINE\nAddress: {}\nType: {}\nAdministration: {}\n",
        octets
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(":"),
        if group {
            "GROUP / MULTICAST"
        } else {
            "INDIVIDUAL / UNICAST"
        },
        if local {
            "LOCALLY ADMINISTERED"
        } else {
            "UNIVERSALLY ADMINISTERED"
        },
    );
    if group {
        output.push_str("No unicast EUI-64 derivation for a group MAC.");
    } else {
        let eui = [
            octets[0] ^ 0x02,
            octets[1],
            octets[2],
            0xff,
            0xfe,
            octets[3],
            octets[4],
            octets[5],
        ];
        let mut bytes = [0u8; 16];
        bytes[0] = 0xfe;
        bytes[1] = 0x80;
        bytes[8..].copy_from_slice(&eui);
        output.push_str(&format!(
            "Modified EUI-64: {}\nIllustrative IPv6 link-local: {}\nNot a privacy address; no interface configured.",
            eui.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(":"),
            Ipv6Addr::from(bytes),
        ));
    }
    Ok(output)
}

fn vlan(input: &str) -> io::Result<String> {
    let parts = bounded(input, 24)?
        .split(',')
        .map(str::trim)
        .collect::<Vec<_>>();
    if parts.len() != 3
        || parts
            .iter()
            .any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(invalid("Enter VID,PCP,DEI, e.g. 42,5,0"));
    }
    let vid = parts[0]
        .parse::<u16>()
        .map_err(|_| invalid("Invalid VLAN ID"))?;
    let pcp = number_u8(parts[1], 7)?;
    let dei = number_u8(parts[2], 1)?;
    if !(1..=4094).contains(&vid) {
        return Err(invalid("VLAN ID must be 1..4094 (0/4095 reserved)"));
    }
    let tci = (u16::from(pcp) << 13) | (u16::from(dei) << 12) | vid;
    Ok(format!(
        "802.1Q TAG / OFFLINE\nVLAN ID: {vid}\nPriority (PCP): {pcp}\nDrop eligible (DEI): {dei}\nTPID: 0x8100\nTCI: 0x{tci:04x}\n802.1Q tag: 81 00 {:02x} {:02x}\nAdds 4 bytes to Ethernet header; no frame transmitted.",
        (tci >> 8) as u8, (tci & 0xff) as u8,
    ))
}

fn mtu(input: &str) -> io::Result<String> {
    let input = bounded(input, 6)?;
    if !input.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid("MTU must be an integer"));
    }
    let bytes = input.parse::<u32>().map_err(|_| invalid("Invalid MTU"))?;
    if !(1280..=9216).contains(&bytes) {
        return Err(invalid(
            "MTU must be 1280..9216 bytes for dual-stack estimate",
        ));
    }
    Ok(format!(
        "MTU / MSS ESTIMATE / OFFLINE\nIP MTU: {bytes} B\nIPv4 TCP MSS: {} B\nIPv6 TCP MSS: {} B\nIPv4 UDP payload: {} B\nIPv6 UDP payload: {} B\nAssumes base IP/TCP/UDP headers; options, extensions and tunnels reduce available payload.",
        bytes - 40, bytes - 60, bytes - 28, bytes - 48,
    ))
}

/// Intentionally excludes active socket probes. Input length and ASCII guards
/// are identical for web and CLI callers. Legacy "cidr" alias is preserved.
pub fn run_offline(mode: &str, input: &str, extra: Option<&str>) -> io::Result<String> {
    bounded(input, MAX_INPUT)?;
    if let Some(other) = extra {
        bounded(other, MAX_EXTRA)?;
    }
    let other = || extra.ok_or_else(|| invalid("This tool requires a second input"));
    match mode {
        "cidr" | "cidr4" => crate::workbench::ipv4_cidr(input),
        "cidr6" => cidr6(input),
        "split4" => split4(input, other()?),
        "route" => route(input, other()?),
        "aggregate4" => aggregate4(input, other()?),
        "overlap4" => overlap4(input, other()?),
        "mac" => mac(input),
        "vlan" => vlan(input),
        "mtu" => mtu(input),
        _ => Err(invalid("Unknown or active network operation")),
    }
}

/// Single TCP endpoint only, literal IP + port; no DNS or range expansion.
/// The caller is responsible for explicit authorization and timeout.
pub fn tcp_target(input: &str, port: &str) -> io::Result<std::net::SocketAddr> {
    let ip: IpAddr = bounded(input, 64)?
        .parse()
        .map_err(|_| invalid("Use a literal IP address (DNS disabled)"))?;
    let port_text = bounded(port, 5)?;
    if !port_text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid("Invalid TCP port"));
    }
    let port = port_text
        .parse::<u16>()
        .map_err(|_| invalid("Invalid TCP port"))?;
    if port == 0
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip == IpAddr::V4(Ipv4Addr::BROADCAST)
    {
        return Err(invalid("TCP target must be a unicast IP and port 1..65535"));
    }
    if let IpAddr::V6(ipv6) = ip {
        // A link-local address requires a scope_id; never guess an interface.
        if ipv6.octets()[0] == 0xfe && ipv6.octets()[1] & 0xc0 == 0x80 {
            return Err(invalid(
                "IPv6 link-local probes need an interface scope; not supported",
            ));
        }
    }
    Ok(std::net::SocketAddr::new(ip, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_and_ipv6_prefix_boundaries() {
        assert!(run_offline("cidr4", "10.0.0.4/31", None)
            .unwrap()
            .contains("Usable addresses: 2"));
        assert!(run_offline("cidr6", "2001:0db8::1/64", None)
            .unwrap()
            .contains("Network: 2001:db8::/64"));
        assert!(run_offline("cidr6", "::/0", None)
            .unwrap()
            .contains("Addresses: 2^128"));
        assert!(run_offline("cidr6", "::1/128", None)
            .unwrap()
            .contains("Addresses: 2^0"));
        for invalid_value in ["::1/129", "not-ip/64", "::1/-1", "::1/64\nunsafe"] {
            assert!(
                run_offline("cidr6", invalid_value, None).is_err(),
                "{invalid_value}"
            );
        }
    }

    #[test]
    fn subnet_splitting_bounded_and_aligned() {
        let output = run_offline("split4", "10.40.0.9/24", Some("26")).unwrap();
        assert!(output.contains("Parent: 10.40.0.0/24"));
        for address in [
            "10.40.0.0/26",
            "10.40.0.64/26",
            "10.40.0.128/26",
            "10.40.0.192/26",
        ] {
            assert!(output.contains(address));
        }
        let huge = run_offline("split4", "0.0.0.0/0", Some("32")).unwrap();
        assert!(huge.contains("Total subnets: 4294967296"));
        assert!(huge.contains("preview limit 12"));
        assert!(run_offline("split4", "10.0.0.0/24", Some("23")).is_err());
    }

    #[test]
    fn route_matching_v4_v6_uses_longest_prefix() {
        let output = run_offline(
            "route",
            "10.40.8.1",
            Some("0.0.0.0/0,10.0.0.0/8,10.40.0.0/16"),
        )
        .unwrap();
        assert!(output.contains("Longest prefix: 10.40.0.0/16"));
        assert!(run_offline(
            "route",
            "2001:db8:1::4",
            Some("::/0,2001:db8::/32,2001:db8:1::/48")
        )
        .unwrap()
        .contains("Longest prefix: 2001:db8:1::/48"));
        assert!(run_offline("route", "10.0.0.1", Some("192.0.2.0/24"))
            .unwrap()
            .contains("Longest prefix: NONE"));
        assert!(run_offline("route", "10.0.0.1", Some("192.0.2.0/24,2001:db8::/32")).is_err());
    }

    #[test]
    fn route_entry_limit_and_whitespace_parity() {
        let sixteen = std::iter::repeat_n("10.0.0.0/8", MAX_ROUTE_ENTRIES)
            .collect::<Vec<_>>()
            .join(",");
        let result = run_offline("route", "10.1.2.3", Some(&sixteen)).unwrap();
        assert!(result.contains("Evaluated: 16 prefixes"));
        assert!(result.contains("Matching: 16"));
        let seventeen = format!("{sixteen},10.0.0.0/8");
        assert!(run_offline("route", "10.1.2.3", Some(&seventeen)).is_err());
        assert!(run_offline("route", "10.1.2.3", Some("10.0.0.0/8,")).is_err());
        assert!(
            run_offline("route", "10.1.2.3", Some("10.0.0.0/8, 192.0.2.0/24"))
                .unwrap()
                .contains("Evaluated: 2 prefixes")
        );
    }

    #[test]
    fn merge_must_cover_exactly_two_siblings() {
        assert!(
            run_offline("aggregate4", "10.40.0.0/24", Some("10.40.1.0/24"))
                .unwrap()
                .contains("Exact aggregate: 10.40.0.0/23")
        );
        assert!(run_offline("aggregate4", "0.0.0.0/1", Some("128.0.0.0/1"))
            .unwrap()
            .contains("0.0.0.0/0"));
        assert!(run_offline("aggregate4", "10.40.0.0/24", Some("10.40.2.0/24")).is_err());
        assert!(run_offline("aggregate4", "10.40.0.0/24", Some("10.40.0.0/24")).is_err());
        assert!(
            run_offline("overlap4", "10.40.0.0/16", Some("10.40.5.0/24"))
                .unwrap()
                .contains("Intersection: 10.40.5.0/24")
        );
        assert!(
            run_offline("overlap4", "10.40.0.0/24", Some("10.40.1.0/24"))
                .unwrap()
                .contains("Overlap: NO")
        );
    }

    #[test]
    fn mac_vlan_and_mtu_fields_are_not_live_telemetry() {
        let info = run_offline("mac", "00:1A:2B:3C:4D:5E", None).unwrap();
        assert!(info.contains("00:1a:2b:3c:4d:5e"));
        assert!(info.contains("fe80::21a:2bff:fe3c:4d5e"));
        assert!(run_offline("mac", "01-00-5e-00-00-01", None)
            .unwrap()
            .contains("GROUP / MULTICAST"));
        assert!(run_offline("mac", "00:11:22:33:44", None).is_err());
        let tag = run_offline("vlan", "42,5,0", None).unwrap();
        assert!(tag.contains("TCI: 0xa02a"));
        assert!(run_offline("vlan", "4095,5,0", None).is_err());
        assert!(run_offline("vlan", "0,0,0", None).is_err());
        let payload = run_offline("mtu", "1500", None).unwrap();
        assert!(payload.contains("IPv4 TCP MSS: 1460"));
        assert!(payload.contains("IPv6 TCP MSS: 1440"));
        assert!(run_offline("mtu", "576", None).is_err());
    }

    #[test]
    fn active_mode_cannot_run_in_offline_core_and_targets_are_strict() {
        assert!(run_offline("tcp", "127.0.0.1", Some("443")).is_err());
        assert!(tcp_target("127.0.0.1", "443").is_ok());
        assert!(tcp_target("::1", "443").is_ok());
        for (ip, port) in [
            ("example.org", "443"),
            ("0.0.0.0", "80"),
            ("224.0.0.1", "80"),
            ("255.255.255.255", "80"),
            ("fe80::1", "80"),
            ("127.0.0.1", "0"),
            ("127.0.0.1", "65536"),
            ("127.0.0.1", "443;echo"),
        ] {
            assert!(tcp_target(ip, port).is_err(), "{ip}:{port}");
        }
        assert!(run_offline("route", "10.0.0.1", Some(&"10.0.0.0/8,".repeat(17))).is_err());
        assert!(run_offline("cidr4", "10.0.0.1/24\nNEXT", None).is_err());
        assert_eq!(OPERATIONS.iter().filter(|op| op.active).count(), 7);
    }
}
