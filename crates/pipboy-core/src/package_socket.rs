//! Inert, versioned descriptors for independent Rust-Boy packages.
//!
//! This catalog is NOT a binary loader, an authority to run packages, or an
//! operating-system sandbox. Parsing reference manifests never opens devices,
//! executes processes, discovers peers, or reads the network.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::io::{self, Read, Write};
use std::path::Path;

pub const SOCKET_API_VERSION: u32 = 1;
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const MAX_MANIFEST_BYTES: usize = 16 * 1024;
pub const MAX_CATALOG_PACKAGES: usize = 64;
/// Maximum encoded size of one adapter protocol message.
pub const MAX_PROTOCOL_MESSAGE_BYTES: usize = 64 * 1024;
pub const MAX_PROTOCOL_VERSIONS: usize = 8;
pub const MAX_IN_FLIGHT_REQUESTS: usize = 64;
pub const PROTOCOL_FRAME_HEADER_BYTES: usize = 4;

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

/// One bounded request on the future host/package transport. This type is a
/// codec contract only; decoding a message never dispatches it to an adapter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdapterMessage {
    pub protocol_version: u32,
    pub request_id: u64,
    pub package_id: String,
    pub command: AdapterCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdapterCommand {
    Activate,
    Suspend,
    Resume,
    Resize { width: u16, height: u16 },
    Intent { name: String },
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    MessageTooLarge,
    MalformedJson(String),
    UnsupportedVersion(u32),
    InvalidRequestId,
    InvalidPackageId,
    InvalidCommand,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MessageTooLarge => write!(f, "Adapter message exceeds 64 KiB"),
            Self::MalformedJson(message) => write!(f, "Invalid adapter message: {message}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "Unsupported adapter protocol version: {version}")
            }
            Self::InvalidRequestId => write!(f, "Adapter request ID must be nonzero"),
            Self::InvalidPackageId => write!(f, "Invalid adapter package ID"),
            Self::InvalidCommand => write!(f, "Invalid adapter command"),
        }
    }
}

impl Error for ProtocolError {}

/// Version negotiation claim made by an adapter at connection startup.
/// `package_id` is an asserted identity, not cryptographic authentication.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdapterHello {
    pub package_id: String,
    pub supported_versions: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandshakeAccepted {
    pub protocol_version: u32,
}

/// Correlated adapter reply. A completed reply reports protocol handling only;
/// it is not proof of user receipt or successful domain work.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AdapterResponse {
    pub protocol_version: u32,
    pub request_id: u64,
    pub package_id: String,
    pub result: AdapterResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdapterResult {
    Completed,
    Rejected { code: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeError {
    InvalidPackageId,
    PackageIdentityMismatch,
    EmptyOrOversizedVersionList,
    InvalidVersionList,
    NoCommonVersion,
}

impl AdapterHello {
    pub fn decode(frame: &[u8]) -> Result<Self, ProtocolError> {
        if frame.len() > MAX_PROTOCOL_MESSAGE_BYTES {
            return Err(ProtocolError::MessageTooLarge);
        }
        let hello: Self = serde_json::from_slice(frame)
            .map_err(|error| ProtocolError::MalformedJson(error.to_string()))?;
        if !valid_id(&hello.package_id) {
            return Err(ProtocolError::InvalidPackageId);
        }
        hello
            .validate_versions()
            .map_err(|_| ProtocolError::InvalidCommand)?;
        Ok(hello)
    }

    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        if !valid_id(&self.package_id) {
            return Err(ProtocolError::InvalidPackageId);
        }
        self.validate_versions()
            .map_err(|_| ProtocolError::InvalidCommand)?;
        let frame = serde_json::to_vec(self)
            .map_err(|error| ProtocolError::MalformedJson(error.to_string()))?;
        if frame.len() > MAX_PROTOCOL_MESSAGE_BYTES {
            return Err(ProtocolError::MessageTooLarge);
        }
        Ok(frame)
    }

    /// Accept only a matching installed-manifest identity and a unique,
    /// bounded list containing the host's supported protocol version.
    pub fn negotiate(
        &self,
        expected_package_id: &str,
        supported_host_versions: &[u32],
    ) -> Result<HandshakeAccepted, HandshakeError> {
        if !valid_id(&self.package_id) {
            return Err(HandshakeError::InvalidPackageId);
        }
        if self.package_id != expected_package_id {
            return Err(HandshakeError::PackageIdentityMismatch);
        }
        let seen = self.validate_versions()?;
        supported_host_versions
            .iter()
            .filter(|version| seen.contains(version))
            .copied()
            .max()
            .map(|protocol_version| HandshakeAccepted { protocol_version })
            .ok_or(HandshakeError::NoCommonVersion)
    }

    fn validate_versions(&self) -> Result<BTreeSet<u32>, HandshakeError> {
        if self.supported_versions.is_empty()
            || self.supported_versions.len() > MAX_PROTOCOL_VERSIONS
        {
            return Err(HandshakeError::EmptyOrOversizedVersionList);
        }
        let mut seen = BTreeSet::new();
        if self
            .supported_versions
            .iter()
            .any(|version| *version == 0 || !seen.insert(*version))
        {
            return Err(HandshakeError::InvalidVersionList);
        }
        Ok(seen)
    }
}

impl AdapterResponse {
    pub fn decode(frame: &[u8]) -> Result<Self, ProtocolError> {
        if frame.len() > MAX_PROTOCOL_MESSAGE_BYTES {
            return Err(ProtocolError::MessageTooLarge);
        }
        let response: Self = serde_json::from_slice(frame)
            .map_err(|error| ProtocolError::MalformedJson(error.to_string()))?;
        response.validate()?;
        Ok(response)
    }

    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;
        let frame = serde_json::to_vec(self)
            .map_err(|error| ProtocolError::MalformedJson(error.to_string()))?;
        if frame.len() > MAX_PROTOCOL_MESSAGE_BYTES {
            return Err(ProtocolError::MessageTooLarge);
        }
        Ok(frame)
    }

    fn validate(&self) -> Result<(), ProtocolError> {
        if self.protocol_version != SOCKET_API_VERSION {
            return Err(ProtocolError::UnsupportedVersion(self.protocol_version));
        }
        if self.request_id == 0 {
            return Err(ProtocolError::InvalidRequestId);
        }
        if !valid_id(&self.package_id) {
            return Err(ProtocolError::InvalidPackageId);
        }
        if let AdapterResult::Rejected { code } = &self.result {
            if !valid_label(code, 64) {
                return Err(ProtocolError::InvalidCommand);
            }
        }
        Ok(())
    }
}

/// Enforces strictly increasing request IDs for one adapter session.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RequestSequence {
    last_request_id: u64,
}

impl RequestSequence {
    pub fn accept(&mut self, message: &AdapterMessage) -> Result<(), ProtocolError> {
        if message.request_id <= self.last_request_id {
            return Err(ProtocolError::InvalidRequestId);
        }
        self.last_request_id = message.request_id;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorrelationError {
    InvalidMessage(ProtocolError),
    TooManyPending,
    DuplicateRequestId,
    UnknownRequestId,
    PackageMismatch,
}

/// Tracks bounded outstanding requests and accepts only matching responses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequestCorrelator {
    pending: BTreeMap<u64, String>,
}

impl RequestCorrelator {
    pub fn register(&mut self, request: &AdapterMessage) -> Result<(), CorrelationError> {
        request
            .validate()
            .map_err(CorrelationError::InvalidMessage)?;
        if self.pending.contains_key(&request.request_id) {
            return Err(CorrelationError::DuplicateRequestId);
        }
        if self.pending.len() >= MAX_IN_FLIGHT_REQUESTS {
            return Err(CorrelationError::TooManyPending);
        }
        self.pending
            .insert(request.request_id, request.package_id.clone());
        Ok(())
    }

    /// Removes a pending request only after both its ID and package ID match.
    pub fn complete(&mut self, response: &AdapterResponse) -> Result<(), CorrelationError> {
        response
            .validate()
            .map_err(CorrelationError::InvalidMessage)?;
        let Some(package_id) = self.pending.get(&response.request_id) else {
            return Err(CorrelationError::UnknownRequestId);
        };
        if package_id != &response.package_id {
            return Err(CorrelationError::PackageMismatch);
        }
        self.pending.remove(&response.request_id);
        Ok(())
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

#[derive(Debug)]
pub enum FrameError {
    Io(io::Error),
    TruncatedHeader,
    TruncatedPayload,
    EmptyFrame,
    FrameTooLarge,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "Adapter stream I/O failed: {error}"),
            Self::TruncatedHeader => write!(f, "Adapter frame header was truncated"),
            Self::TruncatedPayload => write!(f, "Adapter frame payload was truncated"),
            Self::EmptyFrame => write!(f, "Adapter frame must not be empty"),
            Self::FrameTooLarge => write!(f, "Adapter frame exceeds 64 KiB"),
        }
    }
}

impl Error for FrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

/// Reads one big-endian length-prefixed frame. A clean EOF before any header
/// byte returns `Ok(None)`; partial headers and payloads are rejected.
pub fn read_protocol_frame<R: Read>(reader: &mut R) -> Result<Option<Vec<u8>>, FrameError> {
    let mut header = [0u8; PROTOCOL_FRAME_HEADER_BYTES];
    loop {
        match reader.read(&mut header[..1]) {
            Ok(0) => return Ok(None),
            Ok(1) => break,
            Ok(_) => unreachable!("one-byte read returned more than one byte"),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(FrameError::Io(error)),
        }
    }
    reader
        .read_exact(&mut header[1..])
        .map_err(|error| match error.kind() {
            io::ErrorKind::UnexpectedEof => FrameError::TruncatedHeader,
            _ => FrameError::Io(error),
        })?;
    let length = u32::from_be_bytes(header) as usize;
    if length == 0 {
        return Err(FrameError::EmptyFrame);
    }
    if length > MAX_PROTOCOL_MESSAGE_BYTES {
        return Err(FrameError::FrameTooLarge);
    }
    let mut frame = vec![0; length];
    reader
        .read_exact(&mut frame)
        .map_err(|error| match error.kind() {
            io::ErrorKind::UnexpectedEof => FrameError::TruncatedPayload,
            _ => FrameError::Io(error),
        })?;
    Ok(Some(frame))
}

/// Writes one complete, nonempty, bounded length-prefixed protocol frame.
pub fn write_protocol_frame<W: Write>(writer: &mut W, frame: &[u8]) -> Result<(), FrameError> {
    if frame.is_empty() {
        return Err(FrameError::EmptyFrame);
    }
    if frame.len() > MAX_PROTOCOL_MESSAGE_BYTES {
        return Err(FrameError::FrameTooLarge);
    }
    let header = (frame.len() as u32).to_be_bytes();
    writer.write_all(&header).map_err(FrameError::Io)?;
    writer.write_all(frame).map_err(FrameError::Io)
}

impl AdapterMessage {
    /// Parses one complete JSON frame and validates its bounded header/body.
    pub fn decode(frame: &[u8]) -> Result<Self, ProtocolError> {
        if frame.len() > MAX_PROTOCOL_MESSAGE_BYTES {
            return Err(ProtocolError::MessageTooLarge);
        }
        let message: Self = serde_json::from_slice(frame)
            .map_err(|error| ProtocolError::MalformedJson(error.to_string()))?;
        message.validate()?;
        Ok(message)
    }

    pub fn encode(&self) -> Result<Vec<u8>, ProtocolError> {
        self.validate()?;
        let frame = serde_json::to_vec(self)
            .map_err(|error| ProtocolError::MalformedJson(error.to_string()))?;
        if frame.len() > MAX_PROTOCOL_MESSAGE_BYTES {
            return Err(ProtocolError::MessageTooLarge);
        }
        Ok(frame)
    }

    fn validate(&self) -> Result<(), ProtocolError> {
        if self.protocol_version != SOCKET_API_VERSION {
            return Err(ProtocolError::UnsupportedVersion(self.protocol_version));
        }
        if self.request_id == 0 {
            return Err(ProtocolError::InvalidRequestId);
        }
        if !valid_id(&self.package_id) {
            return Err(ProtocolError::InvalidPackageId);
        }
        match &self.command {
            AdapterCommand::Resize { width, height }
                if !(320..=4096).contains(width) || !(240..=4096).contains(height) =>
            {
                Err(ProtocolError::InvalidCommand)
            }
            AdapterCommand::Intent { name } if !valid_label(name, 96) => {
                Err(ProtocolError::InvalidCommand)
            }
            _ => Ok(()),
        }
    }
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
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
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
            return Err(Invalid(
                "package_id must be a bounded reverse-DNS identifier",
            ));
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

    /// Host-owned registry removal for an explicitly unmounted adapter.
    /// Does not remove packages from disk or revoke OS permissions.
    pub fn remove(&mut self, package_id: &str) -> Option<PackageManifest> {
        self.entries.remove(package_id)
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
            [
                "org.rustboy.comms",
                "org.rustboy.gis",
                "org.rustboy.radio",
                "org.rustboy.tv"
            ]
        );
        assert_eq!(catalog.len(), 4);
        for entry in catalog.entries() {
            assert_eq!(
                entry.status.as_deref(),
                Some("design_fixture_not_installed")
            );
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
            assert!(
                PackageManifest::parse(&altered(field, value)).is_err(),
                "{field}"
            );
        }
    }

    #[test]
    fn no_commands_unknown_fields_or_unreviewed_capabilities() {
        for document in [
            altered(
                "entrypoint",
                json!({"type":"external_process","command":"/bin/sh"}),
            ),
            altered(
                "entrypoint",
                json!({"type":"external_process","args":["--run"]}),
            ),
            altered(
                "capabilities",
                json!(["network.fetch.explicit", "secrets.read"]),
            ),
            altered("capabilities", json!(["location.read", "location.read"])),
            altered(
                "ui",
                json!({"mode":"host_view","min_width":1,"min_height":240}),
            ),
            altered(
                "ui",
                json!({"mode":"browser_window","min_width":320,"min_height":240}),
            ),
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
        assert!(registry
            .insert_json(&" ".repeat(MAX_MANIFEST_BYTES + 1))
            .is_err());
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

    #[test]
    fn adapter_protocol_round_trips_typed_lifecycle_messages() {
        let messages = [
            AdapterCommand::Activate,
            AdapterCommand::Suspend,
            AdapterCommand::Resume,
            AdapterCommand::Resize {
                width: 800,
                height: 480,
            },
            AdapterCommand::Intent {
                name: "open-map".into(),
            },
            AdapterCommand::Shutdown,
        ];
        for (request_id, command) in messages.into_iter().enumerate() {
            let message = AdapterMessage {
                protocol_version: SOCKET_API_VERSION,
                request_id: request_id as u64 + 1,
                package_id: "org.rustboy.gis".into(),
                command,
            };
            let frame = message.encode().unwrap();
            assert_eq!(AdapterMessage::decode(&frame).unwrap(), message);
        }
    }

    #[test]
    fn adapter_protocol_rejects_bad_headers_commands_and_oversized_frames() {
        let base = json!({
            "protocol_version": SOCKET_API_VERSION,
            "request_id": 1,
            "package_id": "org.rustboy.gis",
            "command": { "type": "activate" }
        });
        for (field, value, expected) in [
            ("protocol_version", json!(2), "version"),
            ("request_id", json!(0), "request"),
            ("package_id", json!("../gis"), "package"),
        ] {
            let mut document = base.clone();
            document[field] = value;
            let error = AdapterMessage::decode(document.to_string().as_bytes()).unwrap_err();
            assert!(error.to_string().contains(expected));
        }
        let resize = json!({
            "protocol_version": SOCKET_API_VERSION,
            "request_id": 1,
            "package_id": "org.rustboy.gis",
            "command": { "type": "resize", "width": 1, "height": 480 }
        });
        assert_eq!(
            AdapterMessage::decode(resize.to_string().as_bytes()),
            Err(ProtocolError::InvalidCommand)
        );
        assert_eq!(
            AdapterMessage::decode(&vec![b' '; MAX_PROTOCOL_MESSAGE_BYTES + 1]),
            Err(ProtocolError::MessageTooLarge)
        );
    }

    #[test]
    fn adapter_handshake_negotiates_highest_common_version_and_rejects_bad_claims() {
        let hello = AdapterHello {
            package_id: "org.rustboy.gis".into(),
            supported_versions: vec![1, 3, 2],
        };
        let frame = hello.encode().unwrap();
        assert_eq!(AdapterHello::decode(&frame).unwrap(), hello);
        assert_eq!(
            hello.negotiate("org.rustboy.gis", &[1, 2]),
            Ok(HandshakeAccepted {
                protocol_version: 2
            })
        );
        assert_eq!(
            hello.negotiate("org.rustboy.radio", &[1, 2]),
            Err(HandshakeError::PackageIdentityMismatch)
        );
        assert_eq!(
            hello.negotiate("org.rustboy.gis", &[4]),
            Err(HandshakeError::NoCommonVersion)
        );
        for versions in [
            vec![],
            vec![0],
            vec![1, 1],
            vec![1; MAX_PROTOCOL_VERSIONS + 1],
        ] {
            assert!(hello_for_versions(versions).encode().is_err());
        }
    }

    fn hello_for_versions(supported_versions: Vec<u32>) -> AdapterHello {
        AdapterHello {
            package_id: "org.rustboy.gis".into(),
            supported_versions,
        }
    }

    #[test]
    fn adapter_responses_are_bounded_and_request_ids_are_strictly_increasing() {
        let response = AdapterResponse {
            protocol_version: SOCKET_API_VERSION,
            request_id: 4,
            package_id: "org.rustboy.gis".into(),
            result: AdapterResult::Rejected {
                code: "not_ready".into(),
            },
        };
        let frame = response.encode().unwrap();
        assert_eq!(AdapterResponse::decode(&frame).unwrap(), response);
        let bad_response = AdapterResponse {
            result: AdapterResult::Rejected {
                code: "\nsecret".into(),
            },
            ..response.clone()
        };
        assert_eq!(bad_response.encode(), Err(ProtocolError::InvalidCommand));

        let mut sequence = RequestSequence::default();
        let message = |request_id| AdapterMessage {
            protocol_version: SOCKET_API_VERSION,
            request_id,
            package_id: "org.rustboy.gis".into(),
            command: AdapterCommand::Activate,
        };
        assert_eq!(sequence.accept(&message(1)), Ok(()));
        assert_eq!(sequence.accept(&message(3)), Ok(()));
        assert_eq!(
            sequence.accept(&message(2)),
            Err(ProtocolError::InvalidRequestId)
        );
        assert_eq!(
            sequence.accept(&message(3)),
            Err(ProtocolError::InvalidRequestId)
        );
        assert_eq!(sequence.accept(&message(4)), Ok(()));
    }

    #[test]
    fn request_correlator_is_bounded_and_keeps_mismatched_responses_pending() {
        let request = |request_id, package_id: &str| AdapterMessage {
            protocol_version: SOCKET_API_VERSION,
            request_id,
            package_id: package_id.into(),
            command: AdapterCommand::Activate,
        };
        let response = |request_id, package_id: &str| AdapterResponse {
            protocol_version: SOCKET_API_VERSION,
            request_id,
            package_id: package_id.into(),
            result: AdapterResult::Completed,
        };
        let mut correlator = RequestCorrelator::default();
        for request_id in 1..=MAX_IN_FLIGHT_REQUESTS as u64 {
            correlator
                .register(&request(request_id, "org.rustboy.gis"))
                .unwrap();
        }
        assert_eq!(
            correlator.register(&request(1, "org.rustboy.gis")),
            Err(CorrelationError::DuplicateRequestId)
        );
        assert_eq!(
            correlator.register(&request(100, "org.rustboy.gis")),
            Err(CorrelationError::TooManyPending)
        );
        assert_eq!(
            correlator.complete(&response(1, "org.rustboy.radio")),
            Err(CorrelationError::PackageMismatch)
        );
        assert_eq!(correlator.pending_count(), MAX_IN_FLIGHT_REQUESTS);
        assert_eq!(correlator.complete(&response(1, "org.rustboy.gis")), Ok(()));
        assert_eq!(correlator.pending_count(), MAX_IN_FLIGHT_REQUESTS - 1);
        assert_eq!(
            correlator.complete(&response(1, "org.rustboy.gis")),
            Err(CorrelationError::UnknownRequestId)
        );
    }

    #[test]
    fn stream_frames_round_trip_and_reject_truncation_and_unbounded_lengths() {
        let mut encoded = Vec::new();
        write_protocol_frame(&mut encoded, b"hello").unwrap();
        write_protocol_frame(&mut encoded, b"world").unwrap();
        let mut reader = encoded.as_slice();
        assert_eq!(read_protocol_frame(&mut reader).unwrap().unwrap(), b"hello");
        assert_eq!(read_protocol_frame(&mut reader).unwrap().unwrap(), b"world");
        assert_eq!(read_protocol_frame(&mut reader).unwrap(), None);

        assert!(matches!(
            read_protocol_frame(&mut &[0, 0][..]),
            Err(FrameError::TruncatedHeader)
        ));
        assert!(matches!(
            read_protocol_frame(&mut &[0, 0, 0, 3, 1, 2][..]),
            Err(FrameError::TruncatedPayload)
        ));
        assert!(matches!(
            read_protocol_frame(&mut &[0, 0, 0, 0][..]),
            Err(FrameError::EmptyFrame)
        ));
        let oversized_length = (MAX_PROTOCOL_MESSAGE_BYTES as u32 + 1).to_be_bytes();
        assert!(matches!(
            read_protocol_frame(&mut oversized_length.as_slice()),
            Err(FrameError::FrameTooLarge)
        ));
        let mut output = Vec::new();
        assert!(matches!(
            write_protocol_frame(&mut output, &[]),
            Err(FrameError::EmptyFrame)
        ));
        assert!(matches!(
            write_protocol_frame(&mut output, &vec![0; MAX_PROTOCOL_MESSAGE_BYTES + 1]),
            Err(FrameError::FrameTooLarge)
        ));
        assert!(output.is_empty());
    }
}
