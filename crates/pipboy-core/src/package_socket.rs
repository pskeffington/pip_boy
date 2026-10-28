//! Inert, versioned descriptors for independent Rust-Boy packages.
//!
//! This catalog is NOT a binary loader, an authority to run packages, or an
//! operating-system sandbox. Parsing reference manifests never opens devices,
//! executes processes, discovers peers, or reads the network.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

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
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedJson(message) => write!(f, "Invalid package JSON: {message}"),
            Self::InvalidManifest(message) => write!(f, "Invalid package manifest: {message}"),
            Self::DuplicatePackage(id) => write!(f, "Duplicate package ID: {id}"),
            Self::TooManyPackages => write!(f, "Package catalog limit exceeded"),
            Self::ManifestTooLarge => write!(f, "Package manifest exceeds 16 KiB"),
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
    fn requesting_background_work_does_not_grant_it() {
        let manifest = PackageManifest::parse(REFERENCE_MANIFESTS[2]).unwrap();
        assert!(manifest.background.allowed);
        // PackageRegistry stores no grants, capabilities handles or runnable entrypoints.
        assert_eq!(manifest.entrypoint.kind, EntrypointKind::BuiltinAdapter);
    }
}
