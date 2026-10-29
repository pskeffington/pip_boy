//! Compile-time first-party adapter roster. Never executable manifest discovery.
//! The initial passive radio catalogue uses pinned rust_radio metadata only.
//! Selecting it does not open devices, streams, sockets or background tasks.
#![forbid(unsafe_code)]

use crate::radio_directory::curated_stations;
use crate::universal_adapter::{
    AdapterCommand, AdapterEnvelope, AdapterError, AdapterReply, UniversalAdapterHost,
    UniversalPackageAdapter,
};
use serde_json::json;

pub const RADIO_PACKAGE_ID: &str = "org.rustboy.radio";
pub const RADIO_BUILTIN_MANIFEST: &str =
    include_str!("../../../catalog/builtin_packages/rust_radio.json");

/// This explicit construction path, not the reference manifest registry, owns
/// the only reviewed built-in package. There is no process/ABI loading here.
pub fn mount_approved_builtins(host: &mut UniversalAdapterHost) -> Result<(), AdapterError> {
    host.mount_trusted(
        RADIO_BUILTIN_MANIFEST,
        Box::new(RadioCatalogAdapter::default()),
    )
}

#[derive(Default)]
struct RadioCatalogAdapter {
    foreground: bool,
}

impl RadioCatalogAdapter {
    fn view() -> AdapterReply {
        let stations: Vec<_> = curated_stations()
            .iter()
            .map(|station| {
                json!({
                    "id": station.id,
                    "name": station.name,
                    "category": station.category,
                    "genre": station.genre
                })
            })
            .collect();
        AdapterReply::View {
            state: json!({
                "package_id": RADIO_PACKAGE_ID,
                "mode": "catalog_only",
                "player_state": "not_started",
                "network_state": "not_connected",
                "stations": stations
            }),
        }
    }
}

impl UniversalPackageAdapter for RadioCatalogAdapter {
    fn package_id(&self) -> &str {
        RADIO_PACKAGE_ID
    }

    fn handle(&mut self, request: AdapterEnvelope) -> Result<AdapterReply, AdapterError> {
        if request.package_id != RADIO_PACKAGE_ID {
            return Err(AdapterError::Denied("radio adapter package ID mismatch"));
        }
        match request.body {
            AdapterCommand::Activate { .. } | AdapterCommand::Resume { .. } => {
                self.foreground = true;
                Ok(Self::view())
            }
            AdapterCommand::Resize { .. } if self.foreground => Ok(Self::view()),
            AdapterCommand::Resize { .. } => {
                Err(AdapterError::Denied("radio adapter is not foreground"))
            }
            AdapterCommand::Suspend | AdapterCommand::Shutdown => {
                self.foreground = false;
                Ok(AdapterReply::Ack)
            }
            AdapterCommand::Intent { .. } => Err(AdapterError::Denied(
                "read-only radio catalog: use the existing explicit radio player",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package_socket::{HostSurface, PackageManifest};
    use crate::universal_adapter::{AdapterState, DenyAllScopes, Viewport};

    #[test]
    fn builtin_manifest_is_read_only() {
        let manifest = PackageManifest::parse(RADIO_BUILTIN_MANIFEST).unwrap();
        assert_eq!(manifest.package_id, RADIO_PACKAGE_ID);
        assert!(manifest.status.is_none());
        assert!(manifest.capabilities.is_empty());
        assert!(!manifest.background.allowed);
    }

    #[test]
    fn boot_is_idle_and_selection_preserves_host_settings() {
        let mut host = UniversalAdapterHost::new(Viewport::new(480, 320).unwrap());
        mount_approved_builtins(&mut host).unwrap();
        assert_eq!(host.len(), 1);
        assert_eq!(host.surface(), HostSurface::Empty);
        assert_eq!(host.state(RADIO_PACKAGE_ID), Some(AdapterState::Registered));
        assert!(host.foreground_view().is_none());
        assert!(mount_approved_builtins(&mut host).is_err());
        host.select(RADIO_PACKAGE_ID).unwrap();
        let view = host.foreground_view().unwrap();
        assert_eq!(view["mode"], "catalog_only");
        assert_eq!(view["player_state"], "not_started");
        assert_eq!(view["network_state"], "not_connected");
        assert_eq!(
            view["stations"].as_array().unwrap().len(),
            curated_stations().len()
        );
        assert!(view["stations"].as_array().unwrap().iter().all(|station| {
            station["id"].is_string()
                && station["name"].is_string()
                && station.get("playlist_url").is_none()
        }));
        host.open_settings();
        assert_eq!(host.surface(), HostSurface::Settings);
        assert!(host.foreground_view().is_none());
        host.close_settings().unwrap();
        assert_eq!(host.surface(), HostSurface::Package(RADIO_PACKAGE_ID));
        assert!(host.foreground_view().is_some());
    }

    #[test]
    fn no_ui_or_media_intents_are_granted() {
        let mut host = UniversalAdapterHost::new(Viewport::new(480, 320).unwrap());
        mount_approved_builtins(&mut host).unwrap();
        host.select(RADIO_PACKAGE_ID).unwrap();
        assert!(matches!(
            host.intent("ui.play", None, json!({}), &DenyAllScopes),
            Err(AdapterError::Denied(_))
        ));
        assert!(matches!(
            host.intent("media.play", Some("media.audio"), json!({}), &DenyAllScopes),
            Err(AdapterError::Denied(_))
        ));
        assert_eq!(host.state(RADIO_PACKAGE_ID), Some(AdapterState::Foreground));
    }
}
