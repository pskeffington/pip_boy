//! Host compatibility path for the standalone rust_radio module.
pub use rust_radio::directory::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    struct MockHardware(bool);
    impl RadioDiscovery for MockHardware {
        fn real_receiver_available(&self) -> bool {
            self.0
        }
    }
    struct MockIp(IpReachability);
    impl RadioConnectivity for MockIp {
        fn approved_provider_reachable(&self) -> IpReachability {
            self.0
        }
    }

    #[test]
    fn lookup_is_exact_and_allowlisted() {
        assert_eq!(
            station_by_id("skylab").map(|s| s.name),
            Some("Skylab Radio (UK)")
        );
        assert!(station_by_id("https://attacker.test").is_none());
        assert_eq!(
            station_tcp_reachable("https://attacker.test", Duration::ZERO),
            None
        );
    }
    #[test]
    fn sdr_precedes_network() {
        assert_eq!(
            select_source(&MockHardware(true), &MockIp(IpReachability::Unreachable)),
            RadioSource::HardwareSdr
        );
    }
    #[test]
    fn ip_fallback_requires_reachable_provider() {
        assert_eq!(
            select_source(&MockHardware(false), &MockIp(IpReachability::Reachable)),
            RadioSource::InternetDirectory
        );
        assert_eq!(
            select_source(&MockHardware(false), &MockIp(IpReachability::Unreachable)),
            RadioSource::Offline
        );
        assert_eq!(
            select_source(&MockHardware(false), &UncheckedConnectivity),
            RadioSource::Offline
        );
    }
    #[test]
    fn curated_urls_have_publisher_provenance() {
        assert_eq!(curated_stations().len(), 10);
        let mut ids = std::collections::BTreeSet::new();
        for station in curated_stations() {
            assert!(ids.insert(station.id), "duplicate station ID");
            assert!(["CHILL", "AMBIENT", "ELECTRONIC", "RETRO"].contains(&station.category));
            assert!(!station.genre.is_empty());
            assert!(station.playlist_url.starts_with("https://"));
            assert!(station.publisher_reference_url.starts_with("https://"));
            if station.id == "skylab" {
                assert!(station
                    .playlist_url
                    .starts_with("https://uksoutha.streaming.broadcast.radio:29690/"));
                assert_eq!(
                    station.publisher_reference_url,
                    "https://www.skylabradio.com/listen"
                );
            } else {
                assert!(station.playlist_url.starts_with("https://somafm.com/m3u/"));
                assert!(station.playlist_url.ends_with(".m3u"));
                assert_eq!(station.format, "MP3");
                assert_eq!(station.nominal_kbps, 128);
            }
            assert!(!station.name.is_empty());
        }
    }
}
