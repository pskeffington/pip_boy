//! Inert, versioned descriptors for independent Rust-Boy packages.
//!
//! This catalog is NOT a binary loader, an authority to run packages, or an
//! operating-system sandbox. Parsing reference manifests never opens devices,
//! executes processes, discovers peers, or reads the network.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::path::Path;

pub const SOCKET_API_VERSION: u32 = 1;
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const MAX_MANIFEST_BYTES: usize = 16 * 1024;
pub const MAX_CATALOG_PACKAGES: usize = 64;

const REFERENCE_MANIFESTS: [&str; 4] = [
    include_str!("../../../catalog/host_packages/rb_gis.example.json"),
    include_str!("../../../catalog/host_packages/rust_tv.example.json"),
    include_str!("../../../catalog/host_packages/rust_radio.example.json"),
    include_str!("../../../catalog/host_packages/rusti_rc.example.json"),
];

// These are requested scopes, NOT permissions granted by the host.
const KNOWN_SCOPES: &[&str] = &[
    "location.read",
    "map.cache.read",
    "map.cache.write",
    "network.fetch.explicit",
    "media.video",
    "media.audio",
    "radio.device",
    "network.connect.explicit",
    "messages.local.write",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackageManifest {
    pub schema_version: u32,
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub license: String,
    pub source_repository: String,
    pub host_api: u32,
    pub entrypoint: PackageEntrypoint,
    pub ui: PackageUi,
    pub capabilities: Vec<String>,
    pub background: BackgroundDeclaration,
    // Documentation-only: never makes a package trusted or installed.
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackageEntrypoint {
    #[serde(rename = "type")]
    pub kind: EntrypointKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntrypointKind {
    BuiltinAdapter,
    ExternalProcess,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackageUi {
    pub mode: PackageUiMode,
    pub min_width: u16,
    pub min_height: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageUiMode {
    HostView,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BackgroundDeclaration {
    pub allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    MalformedJson(String),
    InvalidManifest(&'static str),
    DuplicatePackage(String),
    TooManyPackages,
    ManifestTooLarge,
    InvalidDirectory(String),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedJson(message) => write!(f, "Invalid package JSON: {message}"),
            Self::InvalidManifest(message) => write!(f, "Invalid package manifest: {message}"),
            Self::DuplicatePackage(id) => write!(f, "Duplicate package ID: {id}"),
            Self::TooManyPackages => write!(f, "Package catalog limit exceeded"),
            Self::ManifestTooLarge => write!(f, "Package manifest exceeds 16 KiB"),
            Self::InvalidDirectory(detail) => write!(f, "Unsafe package directory: {detail}"),
        }
    }
}

impl Error for RegistryError {}

fn valid_id(value: &str) -> bool {
    if value.len() > 96 {
        return false;
    }
    let segments: Vec<_> = value.split('.').collect();
    segments.len() >= 3
        && segments.iter().all(|segment| {
            segment.len() <= 32
                && segment
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_lowercase)
                && !segment.ends_with('-')
                && segment
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        })
}

fn valid_version(value: &str) -> bool {
    if value.is_empty() || value.len() > 64 {
        return false;
    }
    let core = value.split(['-', '+']).next().unwrap_or_default();
    let parts: Vec<_> = core.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part.len() == 1 || !part.starts_with('0'))
        })
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+')
        })
        && !value.ends_with('-')
        && !value.ends_with('+')
        && !value.ends_with('.')
}

fn valid_label(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

impl PackageManifest {
    pub fn parse(document: &str) -> Result<Self, RegistryError> {
        if document.len() > MAX_MANIFEST_BYTES {
            return Err(RegistryError::ManifestTooLarge);
        }
        let manifest: Self = serde_json::from_str(document)
            .map_err(|error| RegistryError::MalformedJson(error.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), RegistryError> {
        use RegistryError::InvalidManifest as Invalid;
        if self.schema_version != MANIFEST_SCHEMA_VERSION {
            return Err(Invalid("unsupported schema_version"));
        }
        if self.host_api != SOCKET_API_VERSION {
            return Err(Invalid("incompatible host_api"));
        }
        if !valid_id(&self.package_id) {
            return Err(Invalid("package_id must be a bounded reverse-DNS identifier"));
        }
        if !valid_label(&self.name, 48)
            || !valid_label(&self.publisher, 96)
            || !valid_label(&self.license, 64)
        {
            return Err(Invalid("invalid name, publisher, or license"));
        }
        if !valid_version(&self.version) {
            return Err(Invalid("invalid semantic version"));
        }
        let repository = self
            .source_repository
            .strip_prefix("https://")
            .unwrap_or_default();
        if repository.len() < 4
            || repository.len() > 250
            || !repository.contains('/')
            || repository.contains('@')
            || repository.contains('#')
            || repository.contains('?')
            || repository.chars().any(char::is_whitespace)
        {
            return Err(Invalid("source_repository requires a bounded HTTPS URL"));
        }
        if self.ui.min_width < 320
            || self.ui.min_width > 4096
            || self.ui.min_height < 240
            || self.ui.min_height > 4096
        {
            return Err(Invalid("unsupported host viewport minimum"));
        }
        if self.capabilities.len() > 16 {
            return Err(Invalid("too many requested capabilities"));
        }
        let mut seen = BTreeSet::new();
        for scope in &self.capabilities {
            if !KNOWN_SCOPES.contains(&scope.as_str()) || !seen.insert(scope.as_str()) {
                return Err(Invalid("unknown or duplicate requested capability"));
            }
        }
        if self
            .status
            .as_deref()
            .is_some_and(|status| status != "design_fixture_not_installed")
        {
            return Err(Invalid("runtime install state must not appear in manifest"));
        }
        Ok(())
    }
}

#[derive(Default, Debug, Clone)]
pub struct PackageRegistry {
    entries: BTreeMap<String, PackageManifest>,
}

impl PackageRegistry {
    /// Registers metadata only; never starts or grants a package.
    pub fn insert_json(&mut self, document: &str) -> Result<(), RegistryError> {
        let manifest = PackageManifest::parse(document)?;
        if self.entries.contains_key(&manifest.package_id) {
            return Err(RegistryError::DuplicatePackage(manifest.package_id));
        }
        if self.entries.len() >= MAX_CATALOG_PACKAGES {
            return Err(RegistryError::TooManyPackages);
        }
        self.entries.insert(manifest.package_id.clone(), manifest);
        Ok(())
    }

    /// Explicit local-only discovery. A scanned manifest is still untrusted,
    /// not installed, not activatable and never executed by this registry.
    pub fn scan_directory(directory: &Path) -> Result<Self, RegistryError> {
        use std::fs;
        let metadata = match fs::symlink_metadata(directory) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(RegistryError::InvalidDirectory(error.to_string())),
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(RegistryError::InvalidDirectory(
                "directory must be a real local directory".into(),
            ));
        }
        let mut paths = Vec::new();
        for entry in fs::read_dir(directory)
            .map_err(|error| RegistryError::InvalidDirectory(error.to_string()))?
        {
            let entry =
                entry.map_err(|error| RegistryError::InvalidDirectory(error.to_string()))?;
            let path = entry.path();
            if path.extension().and_then(|suffix| suffix.to_str()) != Some("json") {
                continue;
            }
            if paths.len() >= MAX_CATALOG_PACKAGES {
                return Err(RegistryError::TooManyPackages);
            }
            paths.push(path);
        }
        paths.sort();
        let mut registry = Self::default();
        for path in paths {
            let metadata = fs::symlink_metadata(&path)
                .map_err(|error| RegistryError::InvalidDirectory(error.to_string()))?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(RegistryError::InvalidDirectory(
                    "linked or nonregular JSON manifest".into(),
                ));
            }
            if metadata.len() > MAX_MANIFEST_BYTES as u64 {
                return Err(RegistryError::ManifestTooLarge);
            }
            let document = fs::read_to_string(&path)
                .map_err(|error| RegistryError::InvalidDirectory(error.to_string()))?;
            registry.insert_json(&document)?;
        }
        Ok(registry)
    }

    pub fn get(&self, package_id: &str) -> Option<&PackageManifest> {
        self.entries.get(package_id)
    }

    pub fn entries(&self) -> impl Iterator<Item = &PackageManifest> {
        self.entries.values()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Bundled reference designs only; none of these are installed socket packages.
pub fn reference_catalog() -> Result<PackageRegistry, RegistryError> {
    let mut registry = PackageRegistry::default();
    for document in REFERENCE_MANIFESTS {
        registry.insert_json(document)?;
    }
    Ok(registry)
}

/// Host-only foreground selection. Does not start/suspend any package and
/// cannot itself authorize an adapter or mutate the installed package roster.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ForegroundSelection {
    active_package_id: Option<String>,
    settings_open: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostSurface<'a> {
    Empty,
    Settings,
    Package(&'a str),
}

impl ForegroundSelection {
    /// The approved IDs must come from a future host-owned trusted installation
    /// registry. A reference manifest never counts as an installation.
    pub fn select(
        &mut self,
        registry: &PackageRegistry,
        approved_installed_ids: &BTreeSet<String>,
        package_id: &str,
    ) -> Result<(), RegistryError> {
        if registry.get(package_id).is_none() || !approved_installed_ids.contains(package_id) {
            return Err(RegistryError::InvalidManifest(
                "package not approved and installed by the host",
            ));
        }
        self.active_package_id = Some(package_id.to_owned());
        self.settings_open = false;
        Ok(())
    }

    pub fn open_settings(&mut self) {
        self.settings_open = true;
    }

    pub fn close_settings(&mut self) {
        self.settings_open = false;
    }

    pub fn clear_package(&mut self) {
        self.active_package_id = None;
    }

    /// Selected package remains known while host Settings is open.
    pub fn active_package_id(&self) -> Option<&str> {
        self.active_package_id.as_deref()
    }

    pub fn settings_open(&self) -> bool {
        self.settings_open
    }

    pub fn surface(&self) -> HostSurface<'_> {
        if self.settings_open {
            HostSurface::Settings
        } else if let Some(id) = self.active_package_id.as_deref() {
            HostSurface::Package(id)
        } else {
            HostSurface::Empty
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn map() -> Value {
        serde_json::from_str(REFERENCE_MANIFESTS[0]).unwrap()
    }

    fn altered(field: &str, value: Value) -> String {
        let mut document = map();
        document[field] = value;
        document.to_string()
    }

    #[test]
    fn four_reference_packages_are_sorted_and_never_installed() {
        let catalog = reference_catalog().unwrap();
        let ids: Vec<_> = catalog
            .entries()
            .map(|entry| entry.package_id.as_str())
            .collect();
        assert_eq!(
            ids,
            ["org.rustboy.comms", "org.rustboy.gis", "org.rustboy.radio", "org.rustboy.tv"]
        );
        assert_eq!(catalog.len(), 4);
        for entry in catalog.entries() {
            assert_eq!(entry.status.as_deref(), Some("design_fixture_not_installed"));
            assert_eq!(entry.host_api, SOCKET_API_VERSION);
        }
    }

    #[test]
    fn unsupported_versions_and_unsafe_ids_are_rejected() {
        for (field, value) in [
            ("schema_version", json!(2)),
            ("host_api", json!(2)),
            ("package_id", json!("../../map")),
            ("package_id", json!("org.RustBoy.map")),
            ("package_id", json!("org.rustboy.")),
            ("version", json!("latest")),
            ("version", json!("1.02.3")),
            ("source_repository", json!("http://example.invalid/map")),
            ("status", json!("installed")),
        ] {
            assert!(PackageManifest::parse(&altered(field, value)).is_err(), "{field}");
        }
    }

    #[test]
    fn no_commands_unknown_fields_or_unreviewed_capabilities() {
        for document in [
            altered("entrypoint", json!({"type":"external_process","command":"/bin/sh"})),
            altered("entrypoint", json!({"type":"external_process","args":["--run"]})),
            altered("capabilities", json!(["network.fetch.explicit", "secrets.read"])),
            altered("capabilities", json!(["location.read", "location.read"])),
            altered("ui", json!({"mode":"host_view","min_width":1,"min_height":240})),
            altered("ui", json!({"mode":"browser_window","min_width":320,"min_height":240})),
        ] {
            assert!(PackageManifest::parse(&document).is_err());
        }
    }

    #[test]
    fn rejected_duplicates_never_overwrite_valid_manifest() {
        let mut registry = PackageRegistry::default();
        registry.insert_json(REFERENCE_MANIFESTS[0]).unwrap();
        assert!(matches!(
            registry.insert_json(REFERENCE_MANIFESTS[0]),
            Err(RegistryError::DuplicatePackage(_))
        ));
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get("org.rustboy.gis").unwrap().name, "MAP");
    }

    #[test]
    fn corrupt_and_oversized_documents_are_rejected_without_execution() {
        let mut registry = PackageRegistry::default();
        assert!(registry.insert_json("{").is_err());
        assert!(registry.insert_json(&" ".repeat(MAX_MANIFEST_BYTES + 1)).is_err());
        assert!(registry.is_empty());
    }

    #[test]
    fn scan_discovers_local_manifests_without_installing_them() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("map.json"), REFERENCE_MANIFESTS[0]).unwrap();
        std::fs::write(temp.path().join("ignored.txt"), "not a manifest").unwrap();
        let registry = PackageRegistry::scan_directory(temp.path()).unwrap();
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.get("org.rustboy.gis").unwrap().name, "MAP");
        assert!(PackageRegistry::scan_directory(&temp.path().join("absent"))
            .unwrap()
            .is_empty());
        // No handles, runners, package install state or grants are created.
    }

    #[test]
    fn scan_rejects_ambiguous_duplicate_package_ids() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("a.json"), REFERENCE_MANIFESTS[0]).unwrap();
        std::fs::write(temp.path().join("b.json"), REFERENCE_MANIFESTS[0]).unwrap();
        assert!(matches!(
            PackageRegistry::scan_directory(temp.path()),
            Err(RegistryError::DuplicatePackage(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn scan_rejects_symlinked_manifest_files() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("original.json"), REFERENCE_MANIFESTS[0]).unwrap();
        std::os::unix::fs::symlink(
            temp.path().join("original.json"),
            temp.path().join("linked.json"),
        )
        .unwrap();
        assert!(matches!(
            PackageRegistry::scan_directory(temp.path()),
            Err(RegistryError::InvalidDirectory(_))
        ));
    }

    #[test]
    fn one_foreground_package_and_permanent_settings_are_host_owned() {
        let registry = reference_catalog().unwrap();
        let mut foreground = ForegroundSelection::default();
        let mut approved = BTreeSet::new();
        assert_eq!(foreground.surface(), HostSurface::Empty);
        assert!(foreground
            .select(&registry, &approved, "org.rustboy.radio")
            .is_err());
        assert_eq!(foreground.surface(), HostSurface::Empty);
        foreground.open_settings();
        assert_eq!(foreground.surface(), HostSurface::Settings);
        foreground.close_settings();
        assert_eq!(foreground.surface(), HostSurface::Empty);

        // Synthetic host approval for pure navigation tests only.
        approved.insert("org.rustboy.radio".to_owned());
        approved.insert("org.rustboy.gis".to_owned());
        foreground
            .select(&registry, &approved, "org.rustboy.radio")
            .unwrap();
        assert_eq!(
            foreground.surface(),
            HostSurface::Package("org.rustboy.radio")
        );
        foreground.open_settings();
        assert_eq!(foreground.surface(), HostSurface::Settings);
        foreground.close_settings();
        assert_eq!(
            foreground.surface(),
            HostSurface::Package("org.rustboy.radio")
        );
        foreground
            .select(&registry, &approved, "org.rustboy.gis")
            .unwrap();
        assert_eq!(
            foreground.surface(),
            HostSurface::Package("org.rustboy.gis")
        );
        assert!(foreground
            .select(&registry, &approved, "org.rustboy.comms")
            .is_err());
        assert_eq!(
            foreground.surface(),
            HostSurface::Package("org.rustboy.gis")
        );
        foreground.clear_package();
        assert_eq!(foreground.surface(), HostSurface::Empty);
        foreground.open_settings();
        assert_eq!(foreground.surface(), HostSurface::Settings);
    }

    #[test]
    fn requesting_background_work_does_not_grant_it() {
        let manifest = PackageManifest::parse(REFERENCE_MANIFESTS[2]).unwrap();
        assert!(manifest.background.allowed);
        // PackageRegistry stores no grants, capabilities handles or runnable entrypoints.
        assert_eq!(manifest.entrypoint.kind, EntrypointKind::BuiltinAdapter);
    }
}
