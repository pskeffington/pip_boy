//! Persisted adapter preferences. Requests are NOT active capability grants.
//! No hardware, socket, radio or device handle is opened by this module.

use crate::capability::Capability;
use crate::notes::NoteStore;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const PREFERENCES_SCHEMA_VERSION: u32 = 1;
pub const MAX_PREFERENCES_BYTES: u64 = 16 * 1024;

/// External adapter surfaces only. The local host/notes/library permissions
/// remain command-scoped; writing a preference never authorizes an adapter.
pub const ADAPTER_CAPABILITIES: &[Capability] = &[
    Capability::DeviceEnumerate,
    Capability::GpioRead,
    Capability::DisplayWrite,
    Capability::SerialRead,
    Capability::GnssRead,
    Capability::SdrReceive,
    Capability::BleDiscover,
    Capability::MeshTransmit,
    Capability::SensorRead,
    Capability::AudioPlay,
    Capability::NetworkFetch,
];

/// Session-only adapter approvals. These are supplied by trusted host code,
/// scoped to the currently active role and package, and are never persisted.
/// They do not authorize operating-system or physical-device access.
pub struct AdapterAuthorization {
    active_role: String,
    approvals: HashSet<(String, String, String)>,
    media_owner: Mutex<Option<String>>,
}

impl AdapterAuthorization {
    pub fn new(active_role: impl Into<String>) -> io::Result<Self> {
        let active_role = active_role.into();
        if !valid_role(&active_role) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid active role",
            ));
        }
        Ok(Self {
            active_role,
            approvals: HashSet::new(),
            media_owner: Mutex::new(None),
        })
    }

    /// Record an ephemeral approval from the trusted host consent path.
    pub fn approve(&mut self, role: &str, package_id: &str, scope: &str) -> io::Result<()> {
        if !valid_role(role)
            || !valid_authority_label(package_id, 128)
            || !valid_authority_label(scope, 80)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid adapter approval",
            ));
        }
        self.approvals
            .insert((role.to_owned(), package_id.to_owned(), scope.to_owned()));
        Ok(())
    }

    pub fn revoke(&mut self, role: &str, package_id: &str, scope: &str) {
        self.approvals
            .remove(&(role.to_owned(), package_id.to_owned(), scope.to_owned()));
    }

    /// Change the host-owned active role. Existing approvals remain bound to
    /// their original role and therefore cannot cross this boundary.
    pub fn set_active_role(&mut self, role: impl Into<String>) -> io::Result<()> {
        let role = role.into();
        if !valid_role(&role) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid active role",
            ));
        }
        let owner = self
            .media_owner
            .lock()
            .map_err(|_| io::Error::other("media ownership state unavailable"))?;
        if owner.is_some() && role != self.active_role {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "stop the current media owner before changing role",
            ));
        }
        drop(owner);
        self.active_role = role;
        Ok(())
    }

    /// Release audio ownership after a successful host-mediated stop.
    pub(crate) fn release_media(&self, package_id: &str) -> bool {
        let Ok(mut owner) = self.media_owner.lock() else {
            return false;
        };
        if owner.as_deref() == Some(package_id) {
            *owner = None;
            true
        } else {
            false
        }
    }
}

fn valid_authority_label(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn valid_role(role: &str) -> bool {
    valid_authority_label(role, 80) && crate::workbench::preset(role).is_some()
}

impl crate::universal_adapter::AdapterIntentAuthority for AdapterAuthorization {
    fn is_granted(&self, package_id: &str, scope: &str) -> bool {
        scope != "media.audio"
            && self.approvals.contains(&(
                self.active_role.clone(),
                package_id.to_owned(),
                scope.to_owned(),
            ))
    }

    fn authorize_media_action(&self, package_id: &str, action: &str) -> bool {
        if !self.approvals.contains(&(
            self.active_role.clone(),
            package_id.to_owned(),
            "media.audio".to_owned(),
        )) {
            return false;
        }
        let Ok(mut owner) = self.media_owner.lock() else {
            return false;
        };
        match action {
            "play" | "resume" => match owner.as_deref() {
                Some(current) => current == package_id,
                None => {
                    *owner = Some(package_id.to_owned());
                    true
                }
            },
            "pause" | "stop" => owner.as_deref() == Some(package_id),
            _ => false,
        }
    }

    fn complete_media_action(&self, package_id: &str, action: &str, succeeded: bool) {
        if action == "stop" && succeeded {
            self.release_media(package_id);
        }
    }
}

pub fn adapter_capability(key: &str) -> io::Result<Capability> {
    Capability::from_key(key)
        .filter(|capability| ADAPTER_CAPABILITIES.contains(capability))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown adapter capability; use permissions list",
            )
        })
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Preference {
    Requested,
    Denied,
}

impl Preference {
    pub fn label(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::Denied => "denied",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreferenceEntry {
    pub capability: String,
    pub preference: Preference,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema_version: u32,
    entries: Vec<PreferenceEntry>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            schema_version: PREFERENCES_SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }
}

impl Document {
    fn validate(&self) -> io::Result<()> {
        if self.schema_version != PREFERENCES_SCHEMA_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported preferences schema; original file preserved",
            ));
        }
        if self.entries.len() > ADAPTER_CAPABILITIES.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "too many adapter preferences; original file preserved",
            ));
        }
        let mut seen = HashSet::new();
        for entry in &self.entries {
            if adapter_capability(&entry.capability).is_err()
                || !seen.insert(entry.capability.as_str())
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid or repeated adapter preference; original file preserved",
                ));
            }
        }
        Ok(())
    }

    fn decode(reader: impl Read) -> io::Result<Self> {
        let mut bytes = Vec::new();
        reader
            .take(MAX_PREFERENCES_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_PREFERENCES_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "preferences document exceeds 16 KiB",
            ));
        }
        let document: Self = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        document.validate()?;
        Ok(document)
    }

    fn encode(&self) -> io::Result<Vec<u8>> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        if bytes.len() as u64 > MAX_PREFERENCES_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "preferences document exceeds 16 KiB",
            ));
        }
        Ok(bytes)
    }
}

pub struct PreferenceStore {
    path: PathBuf,
}

impl PreferenceStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            path: root.join("permissions.json"),
        }
    }

    pub fn from_environment() -> io::Result<Self> {
        let notes = NoteStore::from_environment()?;
        let root = notes
            .path()
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid data directory"))?;
        Ok(Self::new(root.to_path_buf()))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn parent(&self) -> &Path {
        self.path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
    }

    fn load(&self) -> io::Result<Document> {
        match fs::File::open(&self.path) {
            Ok(file) => Document::decode(file),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Document::default()),
            Err(error) => Err(error),
        }
    }

    /// Reading preferences never creates a directory, file or lock.
    pub fn list(&self) -> io::Result<Vec<PreferenceEntry>> {
        Ok(self.load()?.entries)
    }

    fn ensure_directory(&self) -> io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(self.parent())?;
        }
        #[cfg(not(unix))]
        fs::create_dir_all(self.parent())?;
        Ok(())
    }

    /// Stable advisory lock for cooperating preference writers.
    fn writer_lock(&self) -> io::Result<fs::File> {
        self.ensure_directory()?;
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(self.parent().join("permissions.lock"))?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|error| {
            if error.kind() == io::ErrorKind::WouldBlock
                || error.raw_os_error() == fs2::lock_contended_error().raw_os_error()
            {
                io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "preferences are being updated by another writer; retry",
                )
            } else {
                error
            }
        })?;
        Ok(lock)
    }

    fn save(&self, document: &Document) -> io::Result<()> {
        let bytes = document.encode()?;
        let mut staging = tempfile::Builder::new()
            .prefix(".permissions-")
            .tempfile_in(self.parent())?;
        staging.as_file_mut().write_all(&bytes)?;
        staging.as_file().sync_all()?;
        staging.persist(&self.path).map_err(|error| error.error)?;
        #[cfg(unix)]
        fs::File::open(self.parent())
            .and_then(|directory| directory.sync_all())
            .map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!(
                        "preferences committed, but directory durability is uncertain; inspect before retrying: {error}"
                    ),
                )
            })?;
        Ok(())
    }

    /// Returns true only when the persisted preference changed.
    pub fn set(&self, capability: Capability, preference: Preference) -> io::Result<bool> {
        adapter_capability(capability.key())?;
        let _lock = self.writer_lock()?;
        let mut document = self.load()?;
        if let Some(entry) = document
            .entries
            .iter_mut()
            .find(|entry| entry.capability == capability.key())
        {
            if entry.preference == preference {
                return Ok(false);
            }
            entry.preference = preference;
        } else {
            document.entries.push(PreferenceEntry {
                capability: capability.key().to_owned(),
                preference,
            });
        }
        document
            .entries
            .sort_by(|left, right| left.capability.cmp(&right.capability));
        self.save(&document)?;
        Ok(true)
    }

    /// Remove a preference; the adapter remains disabled and ungranted.
    pub fn clear(&self, capability: Capability) -> io::Result<bool> {
        adapter_capability(capability.key())?;
        let _lock = self.writer_lock()?;
        let mut document = self.load()?;
        let before = document.entries.len();
        document
            .entries
            .retain(|entry| entry.capability != capability.key());
        if document.entries.len() == before {
            return Ok(false);
        }
        self.save(&document)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal_adapter::{AdapterIntentAuthority, UniversalAdapterHost};
    use rust_boy_adapter_sdk::Viewport;
    use serde_json::json;

    #[test]
    fn absent_preferences_are_read_only_and_fail_closed() {
        let temporary = tempfile::tempdir().unwrap();
        let home = temporary.path().join("absent");
        let store = PreferenceStore::new(home.clone());
        assert!(store.list().unwrap().is_empty());
        assert!(!home.exists());
        let host =
            crate::runtime::ModuleHost::new(crate::capability::PermissionSet::offline_read_only());
        for capability in ADAPTER_CAPABILITIES {
            assert!(!host.permissions().allows(*capability));
        }
    }

    #[test]
    fn request_deny_clear_round_trip_never_authorizes_a_device() {
        let temporary = tempfile::tempdir().unwrap();
        let store = PreferenceStore::new(temporary.path().join("data"));
        assert!(store
            .set(Capability::SdrReceive, Preference::Requested)
            .unwrap());
        let initial = fs::read(store.path()).unwrap();
        assert!(!store
            .set(Capability::SdrReceive, Preference::Requested)
            .unwrap());
        assert_eq!(fs::read(store.path()).unwrap(), initial);
        assert_eq!(store.list().unwrap()[0].preference, Preference::Requested);
        let host =
            crate::runtime::ModuleHost::new(crate::capability::PermissionSet::offline_read_only());
        assert!(!host.permissions().allows(Capability::SdrReceive));
        assert!(store
            .set(Capability::SdrReceive, Preference::Denied)
            .unwrap());
        assert_eq!(store.list().unwrap()[0].preference, Preference::Denied);
        assert!(store.clear(Capability::SdrReceive).unwrap());
        assert!(!store.clear(Capability::SdrReceive).unwrap());
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn unknown_keys_and_future_or_duplicate_documents_are_preserved() {
        let temporary = tempfile::tempdir().unwrap();
        let store = PreferenceStore::new(temporary.path().to_owned());
        assert!(adapter_capability("host.read").is_err());
        assert!(adapter_capability("unexpected.adapter").is_err());
        assert!(store
            .set(Capability::FilesWrite, Preference::Requested)
            .is_err());
        assert!(!store.path().exists());
        let invalid = [
            "{broken",
            r#"{"schema_version":2,"entries":[]}"#,
            r#"{"schema_version":1,"entries":[{"capability":"network.fetch","preference":"requested"},{"capability":"network.fetch","preference":"denied"}]}"#,
            r#"{"schema_version":1,"entries":[{"capability":"bogus","preference":"requested"}]}"#,
            r#"{"schema_version":1,"entries":[],"future":true}"#,
        ];
        for item in invalid {
            fs::write(store.path(), item).unwrap();
            assert_eq!(store.list().unwrap_err().kind(), io::ErrorKind::InvalidData);
            assert!(store
                .set(Capability::SdrReceive, Preference::Denied)
                .is_err());
            assert_eq!(fs::read_to_string(store.path()).unwrap(), item);
        }
    }

    #[test]
    fn lock_contention_does_not_mutate_preferences() {
        let temporary = tempfile::tempdir().unwrap();
        let store = PreferenceStore::new(temporary.path().to_owned());
        store
            .set(Capability::BleDiscover, Preference::Denied)
            .unwrap();
        let prior = fs::read(store.path()).unwrap();
        let lock = store.writer_lock().unwrap();
        assert_eq!(
            store
                .set(Capability::BleDiscover, Preference::Requested)
                .unwrap_err()
                .kind(),
            io::ErrorKind::WouldBlock
        );
        assert_eq!(fs::read(store.path()).unwrap(), prior);
        drop(lock);
        assert!(store
            .set(Capability::BleDiscover, Preference::Requested)
            .unwrap());
    }

    #[test]
    fn oversized_and_invalid_documents_are_rejected() {
        let temporary = tempfile::tempdir().unwrap();
        let store = PreferenceStore::new(temporary.path().to_owned());
        fs::write(
            store.path(),
            vec![b' '; (MAX_PREFERENCES_BYTES + 1) as usize],
        )
        .unwrap();
        assert_eq!(store.list().unwrap_err().kind(), io::ErrorKind::InvalidData);
        let before = fs::read(store.path()).unwrap();
        assert!(store.clear(Capability::GnssRead).is_err());
        assert_eq!(fs::read(store.path()).unwrap(), before);
    }

    #[test]
    fn adapter_approvals_are_revocable_and_bound_to_the_active_role() {
        use crate::universal_adapter::AdapterCommand;
        use std::sync::{Arc, Mutex};

        let mut host = UniversalAdapterHost::new(Viewport::new(480, 320).unwrap());
        let calls = Arc::new(Mutex::new(Vec::<AdapterCommand>::new()));
        let mut manifest: serde_json::Value = serde_json::from_str(&{
            let source = crate::package_socket::reference_catalog().unwrap();
            let mut value = serde_json::to_value(source.get("org.rustboy.radio").unwrap()).unwrap();
            value.as_object_mut().unwrap().remove("status");
            value.to_string()
        })
        .unwrap();
        manifest["capabilities"] = json!(["media.audio", "network.fetch.explicit", "radio.device"]);
        struct Recorder {
            calls: Arc<Mutex<Vec<AdapterCommand>>>,
        }
        impl rust_boy_adapter_sdk::UniversalPackageAdapter for Recorder {
            fn package_id(&self) -> &str {
                "org.rustboy.radio"
            }
            fn handle(
                &mut self,
                request: rust_boy_adapter_sdk::AdapterEnvelope,
            ) -> Result<rust_boy_adapter_sdk::AdapterReply, rust_boy_adapter_sdk::AdapterError>
            {
                self.calls.lock().unwrap().push(request.body);
                Ok(rust_boy_adapter_sdk::AdapterReply::Ack)
            }
        }
        host.mount_trusted(
            &manifest.to_string(),
            Box::new(Recorder {
                calls: calls.clone(),
            }),
        )
        .unwrap();
        host.select("org.rustboy.radio").unwrap();

        let mut authority = AdapterAuthorization::new("security-chief").unwrap();
        authority
            .approve("recon", "org.rustboy.radio", "media.audio")
            .unwrap();
        assert!(host
            .intent("play", Some("media.audio"), json!({}), &authority)
            .is_err());
        assert_eq!(
            calls.lock().unwrap().len(),
            1,
            "role mismatch reached adapter"
        );

        authority.set_active_role("recon").unwrap();
        assert!(host
            .intent("play", Some("media.audio"), json!({}), &authority)
            .is_ok());
        let after_play = calls.lock().unwrap().len();

        assert_eq!(
            authority
                .set_active_role("security-chief")
                .unwrap_err()
                .kind(),
            io::ErrorKind::WouldBlock,
            "active media owner must be stopped before role change"
        );
        assert!(host
            .intent("stop", Some("media.audio"), json!({}), &authority)
            .is_ok());
        authority.set_active_role("security-chief").unwrap();
        assert_eq!(
            calls.lock().unwrap().len(),
            after_play + 1,
            "only the authorized stop should reach adapter"
        );

        authority.set_active_role("recon").unwrap();
        authority.revoke("recon", "org.rustboy.radio", "media.audio");
        assert!(host
            .intent("play", Some("media.audio"), json!({}), &authority)
            .is_err());
        assert_eq!(
            calls.lock().unwrap().len(),
            after_play + 1,
            "revoked intent reached adapter"
        );
    }

    #[test]
    fn media_authority_serializes_package_owners_and_releases_after_stop() {
        let mut authority = AdapterAuthorization::new("recon").unwrap();
        authority
            .approve("recon", "org.rustboy.radio", "media.audio")
            .unwrap();
        authority
            .approve("recon", "org.rustboy.tv", "media.audio")
            .unwrap();
        assert!(authority.authorize_media_action("org.rustboy.radio", "play"));
        assert!(!authority.authorize_media_action("org.rustboy.tv", "play"));
        assert!(!authority.authorize_media_action("org.rustboy.radio", "seek"));
        assert!(authority.authorize_media_action("org.rustboy.radio", "stop"));
        authority.complete_media_action("org.rustboy.radio", "stop", false);
        assert!(!authority.authorize_media_action("org.rustboy.tv", "play"));
        authority.complete_media_action("org.rustboy.radio", "stop", true);
        assert!(authority.authorize_media_action("org.rustboy.tv", "play"));
    }
}
