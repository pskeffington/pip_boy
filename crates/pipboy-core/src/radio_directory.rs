//! Curated, receive-only internet radio fallback.
//!
//! Source selection is pure and testable. Actual SDR discovery and station
//! playback are independently gated; TCP reachability is not stream validation.

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// A first-party, publisher-documented stream directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InternetStation {
    pub id: &'static str,
    pub name: &'static str,
    pub genre: &'static str,
    pub playlist_url: &'static str,
    pub publisher_reference_url: &'static str,
    pub nominal_kbps: u16,
    pub format: &'static str,
}

/// Curated broadcaster links checked against publisher pages 2026-10-26.
/// Directory inclusion is *not* proof the station is online at runtime.
pub const CURATED_STATIONS: [InternetStation; 4] = [
    InternetStation {
        id: "skylab",
        name: "Skylab Radio (UK)",
        genre: "Chillout / lounge / ambient",
        playlist_url: "https://uksoutha.streaming.broadcast.radio:29690/skylab-radio-limited",
        publisher_reference_url: "https://www.skylabradio.com/listen",
        nominal_kbps: 0, // bitrate not independently verified
        format: "Streaming audio (verify at runtime)",
    },
    InternetStation {
        id: "groovesalad",
        name: "Groove Salad",
        genre: "Ambient / downtempo",
        playlist_url: "https://somafm.com/m3u/groovesalad.m3u",
        publisher_reference_url: "https://somafm.com/groovesalad/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "dronezone",
        name: "Drone Zone",
        genre: "Ambient",
        playlist_url: "https://somafm.com/m3u/dronezone.m3u",
        publisher_reference_url: "https://somafm.com/dronezone/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
    InternetStation {
        id: "deepspaceone",
        name: "Deep Space One",
        genre: "Space / experimental",
        playlist_url: "https://somafm.com/m3u/deepspaceone.m3u",
        publisher_reference_url: "https://somafm.com/deepspaceone/directstreamlinks.html",
        nominal_kbps: 128,
        format: "MP3",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioSource {
    HardwareSdr,
    InternetDirectory,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpReachability {
    Reachable,
    Unreachable,
    NotChecked,
}

/// Distinguish a detected hardware device from a simulation fixture.
pub trait RadioDiscovery {
    fn real_receiver_available(&self) -> bool;
}

/// Availability checks may resolve and connect to *approved radio provider*
/// hosts only. Do not use untrusted remote station-provided hostnames.
pub trait RadioConnectivity {
    fn approved_provider_reachable(&self) -> IpReachability;
}

pub fn select_source<D: RadioDiscovery, C: RadioConnectivity>(
    device: &D,
    connectivity: &C,
) -> RadioSource {
    if device.real_receiver_available() {
        RadioSource::HardwareSdr
    } else if connectivity.approved_provider_reachable() == IpReachability::Reachable {
        RadioSource::InternetDirectory
    } else {
        RadioSource::Offline
    }
}

/// The core currently has no physical SDR backend. Never report the simulator
/// as a receiver: the eventual SoapySDR enumerator replaces this implementation.
pub struct NoHardwareEnumerator;

impl RadioDiscovery for NoHardwareEnumerator {
    fn real_receiver_available(&self) -> bool {
        false
    }
}

pub struct UncheckedConnectivity;
impl RadioConnectivity for UncheckedConnectivity {
    fn approved_provider_reachable(&self) -> IpReachability {
        IpReachability::NotChecked
    }
}

/// Bounded TCP connectivity check to the publisher's HTTPS endpoint.
/// It proves only that a TCP route exists, NOT that HTTPS, the playlist,
/// a codec or the station stream is usable.
pub struct PublisherTcpCheck {
    pub timeout: Duration,
}

impl Default for PublisherTcpCheck {
    fn default() -> Self {
        Self { timeout: Duration::from_secs(2) }
    }
}

impl RadioConnectivity for PublisherTcpCheck {
    fn approved_provider_reachable(&self) -> IpReachability {
        let timeout = self.timeout.min(Duration::from_secs(3));
        let Ok(addresses) = ("somafm.com", 443).to_socket_addrs() else {
            return IpReachability::Unreachable;
        };
        // Bound probe time even if DNS returns multiple addresses.
        if let Some(address) = addresses.take(2).find(|address| TcpStream::connect_timeout(address, timeout).is_ok()) {
            let _ = address;
            IpReachability::Reachable
        } else {
            IpReachability::Unreachable
        }
    }
}

pub fn curated_stations() -> &'static [InternetStation] {
    &CURATED_STATIONS
}

pub fn station_by_id(id: &str) -> Option<&'static InternetStation> {
    curated_stations().iter().find(|station| station.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct MockHardware(bool);
    impl RadioDiscovery for MockHardware {
        fn real_receiver_available(&self) -> bool { self.0 }
    }
    struct MockIp(IpReachability);
    impl RadioConnectivity for MockIp {
        fn approved_provider_reachable(&self) -> IpReachability { self.0 }
    }

    #[test]
    fn lookup_is_exact_and_allowlisted() {
        assert_eq!(station_by_id("skylab").map(|s| s.name), Some("Skylab Radio (UK)"));
        assert!(station_by_id("https://attacker.test").is_none());
    }
    #[test]
    fn sdr_precedes_network() {
        assert_eq!(select_source(&MockHardware(true), &MockIp(IpReachability::Unreachable)), RadioSource::HardwareSdr);
    }
    #[test]
    fn ip_fallback_requires_reachable_provider() {
        assert_eq!(select_source(&MockHardware(false), &MockIp(IpReachability::Reachable)), RadioSource::InternetDirectory);
        assert_eq!(select_source(&MockHardware(false), &MockIp(IpReachability::Unreachable)), RadioSource::Offline);
        assert_eq!(select_source(&MockHardware(false), &UncheckedConnectivity), RadioSource::Offline);
    }
    #[test]
    fn curated_urls_have_publisher_provenance() {
        for station in curated_stations() {
            assert!(station.playlist_url.starts_with("https://"));
            assert!(station.publisher_reference_url.starts_with("https://"));
            if station.id == "skylab" {
                assert!(station.playlist_url.starts_with("https://uksoutha.streaming.broadcast.radio:29690/"));
                assert_eq!(station.publisher_reference_url, "https://www.skylabradio.com/listen");
            } else {
                assert!(station.playlist_url.starts_with("https://somafm.com/m3u/"));
                assert!(station.playlist_url.ends_with(".m3u"));
            }
            assert!(!station.name.is_empty());
        }
    }
}
