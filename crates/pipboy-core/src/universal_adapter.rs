//! Universal first-party package adapter socket for Rust-Boy.
//!
//! A package contributes a *single* foreground view to the host. Discovery,
//! installation approval, Settings, and capability decisions belong to the
//! host. This code does not load executables, listen on a socket, grant device
//! permissions, or sandbox in-process adapters.
//!
//! External modules should target the versioned serializable envelope; only
//! explicitly registered trusted built-in adapters can run in this phase.

use crate::package_socket::{
    EntrypointKind, ForegroundSelection, HostSurface, PackageManifest, PackageRegistry,
    RegistryError,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const ADAPTER_PROTOCOL: u32 = 1;
pub const MAX_WIRE_BYTES: usize = 16 * 1024;
pub const MAX_INTENT_BYTES: usize = 4 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Viewport {
    pub width: u16,
    pub height: u16,
}

impl Viewport {
    pub fn new(width: u16, height: u16) -> Result<Self, AdapterError> {
        if !(320..=4096).contains(&width) || !(240..=4096).contains(&height) {
            return Err(AdapterError::Invalid("unsupported viewport dimensions"));
        }
        Ok(Self { width, height })
    }
}

/// Serializable command vocabulary; no native handles or browser code cross
/// the interface. Intents are data, not automatically authorized operations.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum AdapterCommand {
    Activate { viewport: Viewport },
    Suspend,
    Resume { viewport: Viewport },
    Resize { viewport: Viewport },
    Intent {
        action: String,
        required_scope: Option<String>,
        payload: Value,
    },
    Shutdown,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AdapterEnvelope {
    pub protocol: u32,
    pub package_id: String,
    pub request_id: u64,
    pub body: AdapterCommand,
}

impl AdapterEnvelope {
    /// The encoder can later be used by a bounded IPC transport. Encoding
    /// never executes a command or grants the requested scope.
    pub fn encode(&self) -> Result<Vec<u8>, AdapterError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|_| AdapterError::Invalid("cannot encode adapter request"))?;
        if bytes.len() > MAX_WIRE_BYTES {
            return Err(AdapterError::Invalid("adapter request exceeds wire limit"));
        }
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, AdapterError> {
        if bytes.len() > MAX_WIRE_BYTES {
            return Err(AdapterError::Invalid("adapter request exceeds wire limit"));
        }
        let decoded: Self = serde_json::from_slice(bytes)
            .map_err(|_| AdapterError::Invalid("invalid adapter request"))?;
        decoded.validate()?;
        Ok(decoded)
    }

    fn validate(&self) -> Result<(), AdapterError> {
        if self.protocol != ADAPTER_PROTOCOL || self.request_id == 0 {
            return Err(AdapterError::Invalid("incompatible adapter envelope"));
        }
        // The host's package IDs are already validated by PackageManifest.
        if self.package_id.is_empty() || self.package_id.len() > 96 {
            return Err(AdapterError::Invalid("invalid envelope package ID"));
        }
        match &self.body {
            AdapterCommand::Activate { viewport }
            | AdapterCommand::Resume { viewport }
            | AdapterCommand::Resize { viewport } => {
                Viewport::new(viewport.width, viewport.height)?;
            }
            AdapterCommand::Intent {
                action,
                required_scope,
                payload,
            } => {
                if !valid_action(action) {
                    return Err(AdapterError::Invalid("invalid package intent action"));
                }
                if required_scope
                    .as_deref()
                    .is_some_and(|scope| scope.is_empty() || scope.len() > 80)
                {
                    return Err(AdapterError::Invalid("invalid requested scope"));
                }
                if serde_json::to_vec(payload)
                    .map_err(|_| AdapterError::Invalid("cannot encode package intent"))?
                    .len()
                    > MAX_INTENT_BYTES
                {
                    return Err(AdapterError::Invalid("package intent exceeds 4 KiB"));
                }
            }
            AdapterCommand::Suspend | AdapterCommand::Shutdown => {}
        }
        Ok(())
    }
}

fn valid_action(action: &str) -> bool {
    !action.is_empty()
        && action.len() <= 64
        && action
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_lowercase)
        && action.bytes().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'.' | b'-' | b'_')
        })
}

/// Replies contain inert JSON state. The host renderer must never interpret
/// view data as executable script, raw HTML, a Host Settings panel, or a URL.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum AdapterReply {
    Ack,
    View { state: Value },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdapterError {
    Invalid(&'static str),
    Denied(&'static str),
    Unavailable(String),
    Fault(String),
    Registry(String),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) | Self::Denied(message) => write!(f, "{message}"),
            Self::Unavailable(message) | Self::Fault(message) | Self::Registry(message) => {
                write!(f, "{message}")
            }
        }
    }
}
impl Error for AdapterError {}

impl From<RegistryError> for AdapterError {
    fn from(value: RegistryError) -> Self {
        Self::Registry(value.to_string())
    }
}

/// For trusted *built-in* Rust adapters only. This is not a stable Rust ABI,
/// plugin loader, sandbox, or permission boundary. External modules will use
/// a separately reviewed IPC runner for the same serialized wire vocabulary.
pub trait UniversalPackageAdapter: Send {
    fn package_id(&self) -> &str;
    fn handle(&mut self, request: AdapterEnvelope) -> Result<AdapterReply, AdapterError>;
}

/// Only the host's already-authorized policy authority may approve a scope.
/// The default policy is deny-all. A manifest declaration is never a grant.
pub trait ScopeAuthority {
    fn is_granted(&self, package_id: &str, scope: &str) -> bool;
}

pub struct DenyAllScopes;
impl ScopeAuthority for DenyAllScopes {
    fn is_granted(&self, _package_id: &str, _scope: &str) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterState {
    Registered,
    Foreground,
    Suspended,
    Stopped,
    Faulted,
}

struct MountedAdapter {
    manifest: PackageManifest,
    runner: Box<dyn UniversalPackageAdapter>,
    state: AdapterState,
    last_view: Option<Value>,
}

/// Host-owned foreground manager. Every adapter follows the same protocol;
/// no MAP, RADIO, TV, or COMMS imports are needed in this layer.
pub struct UniversalAdapterHost {
    registry: PackageRegistry,
    mounted: BTreeMap<String, MountedAdapter>,
    foreground: ForegroundSelection,
    viewport: Viewport,
    next_request_id: u64,
}

impl UniversalAdapterHost {
    pub fn new(viewport: Viewport) -> Self {
        Self {
            registry: PackageRegistry::default(),
            mounted: BTreeMap::new(),
            foreground: ForegroundSelection::default(),
            viewport,
            next_request_id: 0,
        }
    }

    /// Only call from trusted host bootstrap with an explicitly reviewed
    /// adapter and its installed manifest. Scanned design fixtures are refused.
    /// Registration calls no adapter lifecycle or network/device functions.
    pub fn mount_trusted(
        &mut self,
        manifest_json: &str,
        runner: Box<dyn UniversalPackageAdapter>,
    ) -> Result<(), AdapterError> {
        let manifest = PackageManifest::parse(manifest_json)?;
        if manifest.status.is_some() || manifest.entrypoint.kind != EntrypointKind::BuiltinAdapter {
            return Err(AdapterError::Denied(
                "only approved installed built-in adapters can be mounted",
            ));
        }
        if runner.package_id() != manifest.package_id {
            return Err(AdapterError::Invalid("adapter ID does not match manifest"));
        }
        self.registry.insert_json(manifest_json)?;
        self.mounted.insert(
            manifest.package_id.clone(),
            MountedAdapter {
                manifest,
                runner,
                state: AdapterState::Registered,
                last_view: None,
            },
        );
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.mounted.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mounted.is_empty()
    }

    pub fn package_ids(&self) -> impl Iterator<Item = &str> {
        self.mounted.keys().map(String::as_str)
    }

    pub fn surface(&self) -> HostSurface<'_> {
        self.foreground.surface()
    }

    pub fn state(&self, package_id: &str) -> Option<AdapterState> {
        self.mounted.get(package_id).map(|entry| entry.state)
    }

    /// Inert view model only; no embedded HTML/scripts and no privileged
    /// module controls. Settings is always rendered separately by the host.
    pub fn foreground_view(&self) -> Option<&Value> {
        match self.foreground.surface() {
            HostSurface::Package(id) => self
                .mounted
                .get(id)
                .filter(|entry| entry.state == AdapterState::Foreground)
                .and_then(|entry| entry.last_view.as_ref()),
            HostSurface::Empty | HostSurface::Settings => None,
        }
    }

    fn fail(&mut self, package_id: &str) {
        if let Some(entry) = self.mounted.get_mut(package_id) {
            entry.state = AdapterState::Faulted;
            entry.last_view = None;
        }
    }

    fn invoke(&mut self, package_id: &str, body: AdapterCommand) -> Result<(), AdapterError> {
        let sequence = self
            .next_request_id
            .checked_add(1)
            .ok_or(AdapterError::Invalid("adapter sequence overflow"))?;
        let message = AdapterEnvelope {
            protocol: ADAPTER_PROTOCOL,
            package_id: package_id.to_owned(),
            request_id: sequence,
            body,
        };
        // Enforce message bounds even for an in-process implementation.
        message.encode()?;
        self.next_request_id = sequence;
        let entry = self
            .mounted
            .get_mut(package_id)
            .ok_or_else(|| AdapterError::Unavailable("package is not mounted".into()))?;
        let reply = entry.runner.handle(message)?;
        let encoded = serde_json::to_vec(&reply)
            .map_err(|_| AdapterError::Invalid("invalid package reply"))?;
        if encoded.len() > MAX_WIRE_BYTES {
            return Err(AdapterError::Invalid("package reply exceeds wire limit"));
        }
        if let AdapterReply::View { state } = reply {
            entry.last_view = Some(state);
        }
        Ok(())
    }

    fn viewport_supported(&self, package_id: &str) -> Result<(), AdapterError> {
        let entry = self
            .mounted
            .get(package_id)
            .ok_or_else(|| AdapterError::Unavailable("package is not mounted".into()))?;
        if self.viewport.width < entry.manifest.ui.min_width
            || self.viewport.height < entry.manifest.ui.min_height
        {
            return Err(AdapterError::Unavailable(
                "package does not fit this viewport".into(),
            ));
        }
        if entry.state == AdapterState::Faulted {
            return Err(AdapterError::Unavailable(
                "package is faulted; explicit reset is required".into(),
            ));
        }
        Ok(())
    }

    /// Select an already-mounted trusted adapter. Only one adapter owns the
    /// foreground; an unsuccessful switch attempts to restore the old view.
    pub fn select(&mut self, package_id: &str) -> Result<(), AdapterError> {
        self.viewport_supported(package_id)?;
        let prior = self.foreground.active_package_id().map(str::to_owned);
        let from_settings = self.foreground.settings_open();
        if prior.as_deref() == Some(package_id) {
            if from_settings {
                self.close_settings()?;
            }
            return Ok(());
        }
        if let Some(ref id) = prior {
            if !from_settings {
                if let Err(error) = self.invoke(id, AdapterCommand::Suspend) {
                    self.fail(id);
                    self.foreground.clear_package();
                    return Err(error);
                }
                if let Some(entry) = self.mounted.get_mut(id) {
                    entry.state = AdapterState::Suspended;
                }
            }
        }
        let status = self.state(package_id).expect("verified mounted package");
        let activate = if status == AdapterState::Suspended {
            AdapterCommand::Resume {
                viewport: self.viewport,
            }
        } else {
            AdapterCommand::Activate {
                viewport: self.viewport,
            }
        };
        if let Err(error) = self.invoke(package_id, activate) {
            self.fail(package_id);
            if let Some(old) = prior {
                if !from_settings {
                    if self
                        .invoke(
                            &old,
                            AdapterCommand::Resume {
                                viewport: self.viewport,
                            },
                        )
                        .is_ok()
                    {
                        if let Some(entry) = self.mounted.get_mut(&old) {
                            entry.state = AdapterState::Foreground;
                        }
                    } else {
                        self.fail(&old);
                        self.foreground.clear_package();
                    }
                }
            }
            return Err(error);
        }
        let installed: BTreeSet<_> = self.mounted.keys().cloned().collect();
        // This is a trusted bootstrap roster, never the discovery roster.
        self.foreground
            .select(&self.registry, &installed, package_id)?;
        if let Some(entry) = self.mounted.get_mut(package_id) {
            entry.state = AdapterState::Foreground;
        }
        Ok(())
    }

    /// Settings is host-owned and remains reachable after adapter failures.
    pub fn open_settings(&mut self) {
        if self.foreground.settings_open() {
            return;
        }
        if let Some(id) = self.foreground.active_package_id().map(str::to_owned) {
            if self.invoke(&id, AdapterCommand::Suspend).is_err() {
                self.fail(&id);
                self.foreground.clear_package();
            } else if let Some(entry) = self.mounted.get_mut(&id) {
                entry.state = AdapterState::Suspended;
            }
        }
        self.foreground.open_settings();
    }

    pub fn close_settings(&mut self) -> Result<(), AdapterError> {
        if !self.foreground.settings_open() {
            return Ok(());
        }
        if let Some(id) = self.foreground.active_package_id().map(str::to_owned) {
            self.viewport_supported(&id)?;
            if let Err(error) = self.invoke(
                &id,
                AdapterCommand::Resume {
                    viewport: self.viewport,
                },
            ) {
                self.fail(&id);
                self.foreground.clear_package();
                return Err(error);
            }
            if let Some(entry) = self.mounted.get_mut(&id) {
                entry.state = AdapterState::Foreground;
            }
        }
        self.foreground.close_settings();
        Ok(())
    }

    pub fn resize(&mut self, width: u16, height: u16) -> Result<(), AdapterError> {
        let viewport = Viewport::new(width, height)?;
        self.viewport = viewport;
        if let HostSurface::Package(id) = self.foreground.surface() {
            let id = id.to_owned();
            if let Err(error) = self.invoke(&id, AdapterCommand::Resize { viewport }) {
                self.fail(&id);
                self.foreground.clear_package();
                return Err(error);
            }
        }
        Ok(())
    }

    /// Safe UI-only actions may use ui.* without a scope; all other intents
    /// must request a declared capability and pass independent host policy.
    /// There is no actual broker here: even allowed data intents do not grant
    /// device handles or transport authority to an untrusted module.
    pub fn intent(
        &mut self,
        action: &str,
        requested_scope: Option<&str>,
        payload: Value,
        policy: &dyn ScopeAuthority,
    ) -> Result<(), AdapterError> {
        if !valid_action(action) {
            return Err(AdapterError::Invalid("invalid package intent action"));
        }
        let active = match self.foreground.surface() {
            HostSurface::Package(id) => id.to_owned(),
            HostSurface::Settings | HostSurface::Empty => {
                return Err(AdapterError::Denied("no foreground package"));
            }
        };
        if let Some(scope) = requested_scope {
            let manifest = &self
                .mounted
                .get(&active)
                .expect("selected package is mounted")
                .manifest;
            if !manifest.capabilities.iter().any(|declared| declared == scope)
                || !policy.is_granted(&active, scope)
            {
                return Err(AdapterError::Denied("scope is not authorized by host policy"));
            }
        } else if !action.starts_with("ui.") {
            return Err(AdapterError::Denied(
                "non-UI actions require an explicit capability scope",
            ));
        }
        if let Err(error) = self.invoke(
            &active,
            AdapterCommand::Intent {
                action: action.to_owned(),
                required_scope: requested_scope.map(str::to_owned),
                payload,
            },
        ) {
            // Malformed input should not take a healthy adapter offline;
            // a real adapter fault or bad reply must fail closed.
            if matches!(error, AdapterError::Fault(_) | AdapterError::Registry(_)) {
                self.fail(&active);
                self.foreground.clear_package();
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn shutdown(&mut self, package_id: &str) -> Result<(), AdapterError> {
        let status = self
            .state(package_id)
            .ok_or_else(|| AdapterError::Unavailable("package is not mounted".into()))?;
        if status != AdapterState::Stopped && status != AdapterState::Registered {
            if let Err(error) = self.invoke(package_id, AdapterCommand::Shutdown) {
                self.fail(package_id);
                if self.foreground.active_package_id() == Some(package_id) {
                    self.foreground.clear_package();
                }
                return Err(error);
            }
        }
        if let Some(entry) = self.mounted.get_mut(package_id) {
            entry.last_view = None;
            entry.state = AdapterState::Stopped;
        }
        if self.foreground.active_package_id() == Some(package_id) {
            self.foreground.clear_package();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    struct Fixture {
        id: String,
        calls: Arc<Mutex<Vec<AdapterCommand>>>,
        fail: Option<&'static str>,
    }

    impl UniversalPackageAdapter for Fixture {
        fn package_id(&self) -> &str {
            &self.id
        }

        fn handle(&mut self, request: AdapterEnvelope) -> Result<AdapterReply, AdapterError> {
            let action = match &request.body {
                AdapterCommand::Activate { .. } => "activate",
                AdapterCommand::Suspend => "suspend",
                AdapterCommand::Resume { .. } => "resume",
                AdapterCommand::Resize { .. } => "resize",
                AdapterCommand::Intent { .. } => "intent",
                AdapterCommand::Shutdown => "shutdown",
            };
            self.calls.lock().unwrap().push(request.body);
            if self.fail == Some(action) {
                return Err(AdapterError::Fault("synthetic adapter fault".into()));
            }
            Ok(AdapterReply::View {
                state: json!({"ready": true, "module": self.id}),
            })
        }
    }

    fn manifest(id: &str) -> String {
        let source = crate::package_socket::reference_catalog().unwrap();
        let mut value = serde_json::to_value(source.get(id).unwrap()).unwrap();
        value.as_object_mut().unwrap().remove("status");
        value.to_string()
    }

    fn mount(
        host: &mut UniversalAdapterHost,
        id: &str,
        fail: Option<&'static str>,
    ) -> Arc<Mutex<Vec<AdapterCommand>>> {
        let calls = Arc::new(Mutex::new(Vec::new()));
        host.mount_trusted(
            &manifest(id),
            Box::new(Fixture {
                id: id.to_string(),
                calls: calls.clone(),
                fail,
            }),
        )
        .unwrap();
        calls
    }

    #[test]
    fn single_protocol_mounts_all_four_reference_types_without_special_cases() {
        let mut host = UniversalAdapterHost::new(Viewport::new(480, 320).unwrap());
        let ids = [
            "org.rustboy.gis",
            "org.rustboy.tv",
            "org.rustboy.radio",
            "org.rustboy.comms",
        ];
        let mut logs = Vec::new();
        for id in ids {
            logs.push(mount(&mut host, id, None));
        }
        assert_eq!(host.len(), 4);
        assert!(logs.iter().all(|l| l.lock().unwrap().is_empty()));
        for id in ids {
            host.select(id).unwrap();
            assert_eq!(host.surface(), HostSurface::Package(id));
            assert_eq!(host.state(id), Some(AdapterState::Foreground));
            assert!(host.foreground_view().is_some());
        }
        assert_eq!(host.surface(), HostSurface::Package("org.rustboy.comms"));
        let foreground = host
            .mounted
            .values()
            .filter(|entry| entry.state == AdapterState::Foreground)
            .count();
        assert_eq!(foreground, 1);
    }

    #[test]
    fn design_fixtures_and_untrusted_process_launchers_are_never_mounted() {
        let mut host = UniversalAdapterHost::new(Viewport::new(480, 320).unwrap());
        let calls = Arc::new(Mutex::new(Vec::new()));
        let runner = || Box::new(Fixture {
            id: "org.rustboy.gis".into(),
            calls: calls.clone(),
            fail: None,
        });
        let design = serde_json::to_string(
            crate::package_socket::reference_catalog()
                .unwrap()
                .get("org.rustboy.gis")
                .unwrap(),
        )
        .unwrap();
        assert!(host.mount_trusted(&design, runner()).is_err());
        let mut external: Value = serde_json::from_str(&manifest("org.rustboy.gis")).unwrap();
        external["entrypoint"]["type"] = json!("external_process");
        assert!(host.mount_trusted(&external.to_string(), runner()).is_err());
        assert_eq!(host.len(), 0);
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn settings_never_yields_foreground_canvas_or_loses_selection() {
        let mut host = UniversalAdapterHost::new(Viewport::new(480, 320).unwrap());
        let calls = mount(&mut host, "org.rustboy.radio", None);
        host.select("org.rustboy.radio").unwrap();
        host.open_settings();
        assert_eq!(host.surface(), HostSurface::Settings);
        assert!(host.foreground_view().is_none());
        host.close_settings().unwrap();
        assert_eq!(host.surface(), HostSurface::Package("org.rustboy.radio"));
        assert_eq!(calls.lock().unwrap().len(), 3);
        host.resize(800, 480).unwrap();
        assert_eq!(calls.lock().unwrap().len(), 4);
    }

    #[test]
    fn failed_activation_restores_previous_foreground() {
        let mut host = UniversalAdapterHost::new(Viewport::new(480, 320).unwrap());
        let previous = mount(&mut host, "org.rustboy.gis", None);
        let failing = mount(&mut host, "org.rustboy.tv", Some("activate"));
        host.select("org.rustboy.gis").unwrap();
        assert!(host.select("org.rustboy.tv").is_err());
        assert_eq!(host.surface(), HostSurface::Package("org.rustboy.gis"));
        assert_eq!(host.state("org.rustboy.gis"), Some(AdapterState::Foreground));
        assert_eq!(host.state("org.rustboy.tv"), Some(AdapterState::Faulted));
        assert_eq!(previous.lock().unwrap().len(), 3);
        assert_eq!(failing.lock().unwrap().len(), 1);
        host.open_settings();
        assert_eq!(host.surface(), HostSurface::Settings);
    }

    #[test]
    fn intent_scopes_are_host_owned_and_denied_by_default() {
        struct Approved;
        impl ScopeAuthority for Approved {
            fn is_granted(&self, id: &str, scope: &str) -> bool {
                id == "org.rustboy.radio" && scope == "media.audio"
            }
        }
        let mut host = UniversalAdapterHost::new(Viewport::new(480, 320).unwrap());
        let log = mount(&mut host, "org.rustboy.radio", None);
        host.select("org.rustboy.radio").unwrap();
        assert!(host
            .intent("play", Some("media.audio"), json!({}), &DenyAllScopes)
            .is_err());
        assert!(host
            .intent("play", Some("secrets.read"), json!({}), &Approved)
            .is_err());
        assert!(host.intent("play", None, json!({}), &Approved).is_err());
        assert_eq!(log.lock().unwrap().len(), 1);
        host.intent("ui.select", None, json!({"index": 2}), &DenyAllScopes)
            .unwrap();
        host.intent("play", Some("media.audio"), json!({"station": "demo"}), &Approved)
            .unwrap();
        assert_eq!(log.lock().unwrap().len(), 3);
        host.open_settings();
        assert!(host
            .intent("play", Some("media.audio"), json!({}), &Approved)
            .is_err());
    }

    #[test]
    fn wire_roundtrip_rejects_oversized_or_incompatible_messages() {
        let envelope = AdapterEnvelope {
            protocol: ADAPTER_PROTOCOL,
            package_id: "org.rustboy.gis".into(),
            request_id: 1,
            body: AdapterCommand::Resize {
                viewport: Viewport::new(800, 480).unwrap(),
            },
        };
        let encoded = envelope.encode().unwrap();
        assert_eq!(AdapterEnvelope::decode(&encoded).unwrap(), envelope);
        let mut incompatible = envelope.clone();
        incompatible.protocol = 77;
        assert!(incompatible.encode().is_err());
        assert!(AdapterEnvelope::decode(&vec![b'x'; MAX_WIRE_BYTES + 1]).is_err());
        let oversized = AdapterEnvelope {
            body: AdapterCommand::Intent {
                action: "ui.search".into(),
                required_scope: None,
                payload: json!({"query": "a".repeat(MAX_INTENT_BYTES)}),
            },
            ..envelope
        };
        assert!(oversized.encode().is_err());
    }

    #[test]
    fn duplicate_or_mismatched_package_rejected_without_calls() {
        let mut host = UniversalAdapterHost::new(Viewport::new(480, 320).unwrap());
        mount(&mut host, "org.rustboy.gis", None);
        let calls = Arc::new(Mutex::new(Vec::new()));
        let duplicate = Fixture {
            id: "org.rustboy.gis".into(),
            calls: calls.clone(),
            fail: None,
        };
        assert!(host
            .mount_trusted(&manifest("org.rustboy.gis"), Box::new(duplicate))
            .is_err());
        let mismatch = Fixture {
            id: "org.rustboy.tv".into(),
            calls: calls.clone(),
            fail: None,
        };
        assert!(host
            .mount_trusted(&manifest("org.rustboy.radio"), Box::new(mismatch))
            .is_err());
        assert_eq!(host.len(), 1);
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn shutdown_clears_view_without_affecting_host_settings() {
        let mut host = UniversalAdapterHost::new(Viewport::new(480, 320).unwrap());
        mount(&mut host, "org.rustboy.comms", None);
        host.select("org.rustboy.comms").unwrap();
        host.shutdown("org.rustboy.comms").unwrap();
        assert_eq!(host.surface(), HostSurface::Empty);
        assert!(host.foreground_view().is_none());
        assert_eq!(host.state("org.rustboy.comms"), Some(AdapterState::Stopped));
        host.open_settings();
        assert_eq!(host.surface(), HostSurface::Settings);
    }
}
