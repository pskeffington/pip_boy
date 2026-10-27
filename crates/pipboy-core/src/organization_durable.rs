//! ORG-3.2 offline state: pinned administrator-approved trust roots, replay
//! records, and signed revocation snapshots with durable atomic replacement.
//! File permissions and local exclusive lock are required. No live route uses
//! this store. The OS credential-store anchor detects replacement of the local
//! state file, but cannot resist a simultaneous rollback/compromise of both
//! the filesystem and the OS credential store; hardware counters are untested.
use crate::organization_authority::{
    verify_enrollment_evidence, ApprovalBundle, AuthorityContext, AuthorityError,
    EnrollmentEvidence, SignedRevocations, VolatileNonceJournal,
};
use crate::organization::OrgGraph;
use crate::organization_policy::ProposedExchange;
use crate::organization_custody::{fingerprint, valid_org_id, AdministratorApproval, ProtectedSecrets};
use fs2::FileExt;
use ring::digest::{digest, SHA256};
use ring::signature::{UnparsedPublicKey, ED25519};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

const MAX_RECORDS: usize = 16_384;
const MAX_FILE_BYTES: u64 = 2_000_000;
const SNAPSHOT_MAX: u64 = 86_400;
const REVOCATION_DOMAIN: &[u8] = b"rust-boy.org.revocation.v1\0";

#[derive(Debug)]
pub enum StateError {
    Io(io::Error),
    Invalid,
    Unapproved,
    Replay,
    Capacity,
    Stale,
    Rollback,
    UntrustedAnchor,
    AnchorMismatch,
    RevocationRegression,
    Poisoned,
}

impl From<io::Error> for StateError {
    fn from(value: io::Error) -> Self { Self::Io(value) }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrganizationState {
    schema_version: u32,
    organization_id: String,
    root_public_key: Vec<u8>,
    issuer_key_epoch: u64,
    revocation_generation: u64,
    revocations: Option<SignedRevocations>,
    consumed_nonces: Vec<[u8; 32]>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtectedAnchor {
    schema_version: u32,
    organization_id: String,
    root_fingerprint: String,
    issuer_key_epoch: u64,
    revocation_generation: u64,
    directory_sha256: String,
    state_sha256: String,
}

pub struct DurableOrganization<'s, S: ProtectedSecrets> {
    root: std::path::PathBuf,
    _lock: File,
    state: OrganizationState,
    store: &'s S,
    anchor_name: String,
    poisoned: bool,
}

fn anchored_name(org: &str) -> String {
    format!("organization-anchor:{org}")
}

fn sha256_hex(bytes: &[u8]) -> String {
    crate::organization_custody::hex(digest(&SHA256, bytes).as_ref())
}

fn expected_anchor(directory: &Path, state: &OrganizationState) -> Result<ProtectedAnchor, StateError> {
    let encoded = serde_json::to_vec(state).map_err(|_| StateError::Invalid)?;
    let canonical = fs::canonicalize(directory)?;
    Ok(ProtectedAnchor {
        schema_version: 1,
        organization_id: state.organization_id.clone(),
        root_fingerprint: fingerprint(&state.root_public_key).map_err(|_| StateError::Invalid)?,
        issuer_key_epoch: state.issuer_key_epoch,
        revocation_generation: state.revocation_generation,
        directory_sha256: sha256_hex(canonical.to_string_lossy().as_bytes()),
        state_sha256: sha256_hex(&encoded),
    })
}

fn read_anchor(store: &impl ProtectedSecrets, name: &str) -> Result<Option<ProtectedAnchor>, StateError> {
    store.read(name).map_err(|_| StateError::UntrustedAnchor)?
        .map(|s| serde_json::from_str(&s).map_err(|_| StateError::UntrustedAnchor))
        .transpose()
}

fn write_anchor(store: &impl ProtectedSecrets, name: &str, value: &ProtectedAnchor) -> Result<(), StateError> {
    let encoded = serde_json::to_string(value).map_err(|_| StateError::Invalid)?;
    store.write(name, &encoded).map_err(|_| StateError::UntrustedAnchor)?;
    if read_anchor(store, name)?.as_ref() != Some(value) {
        return Err(StateError::UntrustedAnchor);
    }
    Ok(())
}

impl<'s, S: ProtectedSecrets> DurableOrganization<'s, S> {
    pub fn provision(
        directory: &Path,
        organization_id: &str,
        root_public_key: &[u8],
        admin: &impl AdministratorApproval,
        store: &'s S,
    ) -> Result<Self, StateError> {
        if !valid_org_id(organization_id) { return Err(StateError::Invalid); }
        let fingerprint = fingerprint(root_public_key).map_err(|_| StateError::Invalid)?;
        admin.approve_root_pin(organization_id, &fingerprint)
            .map_err(|_| StateError::Unapproved)?;
        let (lock, _) = Self::open_lock(directory)?;
        let name = anchored_name(organization_id);
        if directory.join("org-state.json").exists() || read_anchor(store, &name)?.is_some() {
            return Err(StateError::Unapproved);
        }
        let state = OrganizationState {
            schema_version: 1,
            organization_id: organization_id.to_owned(),
            root_public_key: root_public_key.to_vec(),
            issuer_key_epoch: 1,
            revocation_generation: 0,
            revocations: None,
            consumed_nonces: Vec::new(),
        };
        // Protected keyring anchor first: interrupted storage commit fails closed.
        write_anchor(store, &name, &expected_anchor(directory, &state)?)?;
        persist(directory, &state)?;
        Ok(Self {
            root: directory.to_owned(),
            _lock: lock,
            state,
            store,
            anchor_name: name,
            poisoned: false,
        })
    }

    pub fn open(directory: &Path, organization_id: &str, store: &'s S) -> Result<Self, StateError> {
        if !valid_org_id(organization_id) { return Err(StateError::Invalid); }
        let (lock, _) = Self::open_lock(directory)?;
        let path = directory.join("org-state.json");
        reject_symlink(&path)?;
        let mut bytes = Vec::new();
        File::open(path)?.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_FILE_BYTES { return Err(StateError::Capacity); }
        let state: OrganizationState = serde_json::from_slice(&bytes)
            .map_err(|_| StateError::Invalid)?;
        if state.schema_version != 1 || !valid_org_id(&state.organization_id)
            || fingerprint(&state.root_public_key).is_err()
            || state.issuer_key_epoch == 0 || state.consumed_nonces.len() > MAX_RECORDS
            || state.consumed_nonces.iter().any(|n| *n == [0; 32])
        { return Err(StateError::Invalid); }
        let mut distinct = state.consumed_nonces.clone();
        distinct.sort_unstable();
        distinct.dedup();
        if distinct.len() != state.consumed_nonces.len() { return Err(StateError::Invalid); }
        if let Some(snapshot) = &state.revocations {
            if snapshot.generation != state.revocation_generation
                || !check_snapshot(&state, snapshot) { return Err(StateError::Invalid); }
        } else if state.revocation_generation != 0 { return Err(StateError::Invalid); }
        if state.organization_id != organization_id { return Err(StateError::AnchorMismatch); }
        let name = anchored_name(organization_id);
        if read_anchor(store, &name)?.as_ref() != Some(&expected_anchor(directory, &state)?) {
            return Err(StateError::AnchorMismatch);
        }
        Ok(Self {
            root: directory.to_owned(), _lock: lock, state,
            store, anchor_name: name, poisoned: false,
        })
    }

    fn open_lock(directory: &Path) -> Result<(File, Option<OrganizationState>), StateError> {
        fs::create_dir_all(directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::{PermissionsExt, OpenOptionsExt};
            let meta = fs::symlink_metadata(directory)?;
            if meta.file_type().is_symlink() || !meta.is_dir() { return Err(StateError::Invalid); }
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
            reject_symlink(&directory.join("org-state.lock"))?;
            let lock = OpenOptions::new().create(true).read(true).write(true).mode(0o600)
                .open(directory.join("org-state.lock"))?;
            lock.try_lock_exclusive()?;
            return Ok((lock, None));
        }
        #[cfg(not(unix))]
        {
            reject_symlink(&directory.join("org-state.lock"))?;
            let lock = OpenOptions::new().create(true).read(true).write(true)
                .open(directory.join("org-state.lock"))?;
            lock.try_lock_exclusive()?;
            Ok((lock, None))
        }
    }


    fn check_anchor(&self) -> Result<(), StateError> {
        if self.poisoned { return Err(StateError::Poisoned); }
        let actual = read_anchor(self.store, &self.anchor_name)?
            .ok_or(StateError::UntrustedAnchor)?;
        if actual != expected_anchor(&self.root, &self.state)? {
            return Err(StateError::AnchorMismatch);
        }
        Ok(())
    }

    fn commit_state(&mut self, next: OrganizationState) -> Result<(), StateError> {
        self.check_anchor()?;
        let next_anchor = expected_anchor(&self.root, &next)?;
        // A failed keyring/filesystem transition poisons this handle.
        // No transparent rollback or plaintext recovery is permitted.
        self.poisoned = true;
        write_anchor(self.store, &self.anchor_name, &next_anchor)?;
        persist(&self.root, &next)?;
        self.state = next;
        self.poisoned = false;
        Ok(())
    }

    pub fn verified_root_public_key(&self) -> Result<&[u8], StateError> {
        self.check_anchor()?;
        Ok(&self.state.root_public_key)
    }
    pub fn organization_id(&self) -> &str { &self.state.organization_id }
    pub fn issuer_key_epoch(&self) -> u64 { self.state.issuer_key_epoch }
    pub fn revocation_generation(&self) -> u64 { self.state.revocation_generation }
    pub fn revocations(&self) -> Result<Option<&SignedRevocations>, StateError> {
        self.check_anchor()?;
        Ok(self.state.revocations.as_ref())
    }

    /// Verified expiry is enforced by the caller's separately authenticated clock.
    /// Missing or expired snapshots MUST block authentication evidence consumption.
    pub fn fresh_snapshot(&self, trusted_now: u64) -> Result<&SignedRevocations, StateError> {
        self.check_anchor()?;
        let r = self.state.revocations.as_ref().ok_or(StateError::Stale)?;
        if !(r.not_before <= trusted_now && trusted_now < r.expires_at) {
            return Err(StateError::Stale);
        }
        Ok(r)
    }

    /// Install a full, signed snapshot using monotonic generation. Even with
    /// matching generation, different contents are not accepted.
    pub fn install_revocations(&mut self, snapshot: SignedRevocations)
        -> Result<(), StateError>
    {
        self.check_anchor()?;
        if !check_snapshot(&self.state, &snapshot) { return Err(StateError::Invalid); }
        if snapshot.generation <= self.state.revocation_generation {
            return Err(StateError::Rollback);
        }
        if let Some(previous) = &self.state.revocations {
            let contains_prior = |old: &[u64], new: &[u64]| {
                old.iter().all(|serial| new.binary_search(serial).is_ok())
            };
            if !contains_prior(&previous.credential_serials, &snapshot.credential_serials)
                || !contains_prior(&previous.approval_serials, &snapshot.approval_serials)
            {
                return Err(StateError::RevocationRegression);
            }
        }
        let mut next = self.state.clone();
        next.revocation_generation = snapshot.generation;
        next.revocations = Some(snapshot);
        self.commit_state(next)
    }

    /// Atomically commit a challenge before reporting an evidence result.
    /// One entire 32-byte nonce is unique within this organization's ledger.
    /// Capacity exhaustion fails closed. Keep ledger through expiry/rotation
    /// until a separately reviewed compaction protocol exists.
    /// Import a bounded signed full snapshot from offline media. This does not
    /// contact peers, establish freshness of time, or automatically grant access.
    /// The signed organization's ID, signature, generation and cumulative
    /// revocations are checked before a durable state transition.
    pub fn import_revocations_json(&mut self, bytes: &[u8]) -> Result<(), StateError> {
        if bytes.len() > 32_768 {
            return Err(StateError::Capacity);
        }
        let snapshot: SignedRevocations =
            serde_json::from_slice(bytes).map_err(|_| StateError::Invalid)?;
        self.install_revocations(snapshot)
    }

    pub fn export_revocations_json(&self) -> Result<Vec<u8>, StateError> {
        self.check_anchor()?;
        let snapshot = self.state.revocations.as_ref().ok_or(StateError::Stale)?;
        serde_json::to_vec(snapshot).map_err(|_| StateError::Invalid)
    }

    pub fn record_nonce(&mut self, nonce: [u8; 32]) -> Result<(), StateError> {
        self.check_anchor()?;
        if nonce == [0; 32] { return Err(StateError::Invalid); }
        if self.state.consumed_nonces.contains(&nonce) { return Err(StateError::Replay); }
        if self.state.consumed_nonces.len() >= MAX_RECORDS { return Err(StateError::Capacity); }
        let mut next = self.state.clone();
        next.consumed_nonces.push(nonce);
        self.commit_state(next)
    }
}

/// The result is identity/approval EVIDENCE ONLY, never route authorization.
#[derive(Debug)]
pub enum DurableEvidenceError {
    State(StateError),
    Verification(AuthorityError),
    SnapshotMismatch,
}

impl From<StateError> for DurableEvidenceError {
    fn from(value: StateError) -> Self { Self::State(value) }
}

/// Tie ORG-3.1's existing signature checks to the pinned local root, stored
/// signed revocation snapshot and durable one-time nonce journal. The caller
/// is still responsible for independently authenticated time, a CSPRNG nonce,
/// authenticated operator session, and a channel binding from the real link.
/// This function does not activate a communication or task route.
pub fn verify_with_durable_evidence<S: ProtectedSecrets>(
    state: &mut DurableOrganization<'_, S>,
    graph: &OrgGraph,
    query: &ProposedExchange<'_>,
    verified_now: u64,
    nonce: &[u8; 32],
    authenticated_channel_binding: &[u8; 32],
    bundle: &ApprovalBundle<'_>,
) -> Result<EnrollmentEvidence, DurableEvidenceError> {
    if state.organization_id() != query.organization_id {
        return Err(DurableEvidenceError::SnapshotMismatch);
    }
    let verified = {
        let snapshot = state.fresh_snapshot(verified_now)?;
        if snapshot != bundle.revocations {
            return Err(DurableEvidenceError::SnapshotMismatch);
        }
        let ctx = AuthorityContext {
            pinned_root_key: state.verified_root_public_key()?,
            issuer_key_epoch: state.issuer_key_epoch(),
            verified_now,
            minimum_revocation_generation: state.revocation_generation(),
            verifier_nonce: nonce,
            authenticated_channel_binding,
        };
        // ORG-3.1 verifies all approvals. The permanent write below, not this
        // ephemeral journal, enforces replay rejection across process restarts.
        let mut ephemeral = VolatileNonceJournal::default();
        verify_enrollment_evidence(graph, query, &ctx, bundle, &mut ephemeral)
            .map_err(DurableEvidenceError::Verification)?
    };
    state.record_nonce(*nonce)?;
    Ok(verified)
}

fn reject_symlink(path: &Path) -> Result<(), StateError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(StateError::Invalid),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(StateError::Io(error)),
    }
}

fn sorted_nonzero(items: &[u64]) -> bool {
    items.iter().all(|v| *v > 0) && items.windows(2).all(|w| w[0] < w[1])
}

fn check_snapshot(state: &OrganizationState, r: &SignedRevocations) -> bool {
    if r.schema_version != 1 || r.organization_id != state.organization_id
        || r.issuer_key_epoch != state.issuer_key_epoch || r.generation == 0
        || r.not_before >= r.expires_at
        || r.expires_at - r.not_before > SNAPSHOT_MAX
        || r.credential_serials.len() > 1024 || r.approval_serials.len() > 1024
        || !sorted_nonzero(&r.credential_serials)
        || !sorted_nonzero(&r.approval_serials)
        || r.signature.len() != 64 { return false; }
    let mut data = REVOCATION_DOMAIN.to_vec();
    data.extend_from_slice(&r.schema_version.to_be_bytes());
    data.extend_from_slice(&(r.organization_id.len() as u32).to_be_bytes());
    data.extend_from_slice(r.organization_id.as_bytes());
    data.extend_from_slice(&r.issuer_key_epoch.to_be_bytes());
    data.extend_from_slice(&r.generation.to_be_bytes());
    data.extend_from_slice(&r.not_before.to_be_bytes());
    data.extend_from_slice(&r.expires_at.to_be_bytes());
    data.extend_from_slice(&(r.credential_serials.len() as u32).to_be_bytes());
    for s in &r.credential_serials { data.extend_from_slice(&s.to_be_bytes()); }
    data.extend_from_slice(&(r.approval_serials.len() as u32).to_be_bytes());
    for s in &r.approval_serials { data.extend_from_slice(&s.to_be_bytes()); }
    UnparsedPublicKey::new(&ED25519, &state.root_public_key).verify(&data, &r.signature).is_ok()
}

fn persist(directory: &Path, state: &OrganizationState) -> Result<(), StateError> {
    let bytes = serde_json::to_vec(state).map_err(|_| StateError::Invalid)?;
    if bytes.len() as u64 > MAX_FILE_BYTES { return Err(StateError::Capacity); }
    let mut temp = tempfile::NamedTempFile::new_in(directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temp.as_file().set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    temp.write_all(&bytes)?;
    temp.as_file().sync_all()?;
    reject_symlink(&directory.join("org-state.json"))?;
    temp.persist(directory.join("org-state.json")).map_err(|e| StateError::Io(e.error))?;
    #[cfg(unix)]
    File::open(directory)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization_custody::{CustodyError, tests::MemorySecrets};
    use ring::signature::{Ed25519KeyPair, KeyPair};
    struct Approve;
    impl AdministratorApproval for Approve {
        fn approve_root_pin(&self, _: &str, _: &str) -> Result<(), CustodyError> { Ok(()) }
    }
    struct Deny;
    impl AdministratorApproval for Deny {
        fn approve_root_pin(&self, _: &str, _: &str) -> Result<(), CustodyError> {
            Err(CustodyError::NotApproved)
        }
    }
    fn key() -> Ed25519KeyPair {
        Ed25519KeyPair::from_seed_unchecked(&[12; 32]).unwrap()
    }
    fn snapshot(root: &Ed25519KeyPair, generation: u64, serials: &[u64]) -> SignedRevocations {
        let mut state = OrganizationState {
            schema_version: 1, organization_id: "test-org".into(),
            root_public_key: root.public_key().as_ref().to_vec(),
            issuer_key_epoch: 1, revocation_generation: 0,
            revocations: None, consumed_nonces: vec![]
        };
        let mut r = SignedRevocations {
            schema_version: 1, organization_id: "test-org".into(),
            issuer_key_epoch: 1, generation, not_before: 10, expires_at: 100,
            credential_serials: serials.to_vec(), approval_serials: vec![9], signature: vec![]
        };
        let mut data = REVOCATION_DOMAIN.to_vec();
        data.extend_from_slice(&r.schema_version.to_be_bytes());
        data.extend_from_slice(&(r.organization_id.len() as u32).to_be_bytes());
        data.extend_from_slice(r.organization_id.as_bytes());
        data.extend_from_slice(&r.issuer_key_epoch.to_be_bytes());
        data.extend_from_slice(&r.generation.to_be_bytes());
        data.extend_from_slice(&r.not_before.to_be_bytes());
        data.extend_from_slice(&r.expires_at.to_be_bytes());
        data.extend_from_slice(&(r.credential_serials.len() as u32).to_be_bytes());
        for v in &r.credential_serials { data.extend_from_slice(&v.to_be_bytes()); }
        data.extend_from_slice(&(r.approval_serials.len() as u32).to_be_bytes());
        for v in &r.approval_serials { data.extend_from_slice(&v.to_be_bytes()); }
        r.signature = root.sign(&data).as_ref().to_vec();
        assert!(check_snapshot(&state, &r));
        state.revocation_generation = generation;
        r
    }

    #[test]
    fn restored_old_nonce_ledger_and_root_substitution_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemorySecrets::default();
        let root = key();
        let mut org = DurableOrganization::provision(
            dir.path(), "test-org", root.public_key().as_ref(), &Approve, &store
        ).unwrap();
        let old_state = fs::read(dir.path().join("org-state.json")).unwrap();
        org.record_nonce([42; 32]).unwrap();
        drop(org);
        fs::write(dir.path().join("org-state.json"), &old_state).unwrap();
        assert!(matches!(
            DurableOrganization::open(dir.path(), "test-org", &store),
            Err(StateError::AnchorMismatch)
        ));
        let mut changed: OrganizationState = serde_json::from_slice(&old_state).unwrap();
        let substitute = Ed25519KeyPair::from_seed_unchecked(&[99; 32]).unwrap();
        changed.root_public_key = substitute.public_key().as_ref().to_vec();
        fs::write(
            dir.path().join("org-state.json"),
            serde_json::to_vec(&changed).unwrap()
        ).unwrap();
        assert!(matches!(
            DurableOrganization::open(dir.path(), "test-org", &store),
            Err(StateError::AnchorMismatch)
        ));
    }

    #[test]
    fn missing_protected_anchor_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let store = MemorySecrets::default();
        let root = key();
        let org = DurableOrganization::provision(
            dir.path(), "test-org", root.public_key().as_ref(), &Approve, &store
        ).unwrap();
        drop(org);
        store.0.lock().unwrap().remove("organization-anchor:test-org");
        assert!(matches!(
            DurableOrganization::open(dir.path(), "test-org", &store),
            Err(StateError::AnchorMismatch)
        ));
    }

    #[test]
    fn explicit_pin_replay_restart_and_generation_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let root = key();
        let store = MemorySecrets::default();
        assert!(matches!(DurableOrganization::provision(dir.path(), "test-org", root.public_key().as_ref(), &Deny, &store), Err(StateError::Unapproved)));
        let mut org = DurableOrganization::provision(dir.path(), "test-org", root.public_key().as_ref(), &Approve, &store).unwrap();
        assert!(matches!(org.fresh_snapshot(20), Err(StateError::Stale)));
        assert!(org.install_revocations(snapshot(&root, 2, &[5])).is_ok());
        assert!(org.fresh_snapshot(20).is_ok());
        let signed_export = org.export_revocations_json().unwrap();
        assert!(matches!(
            org.import_revocations_json(&signed_export),
            Err(StateError::Rollback)
        ));
        assert!(matches!(
            org.import_revocations_json(b"{broken json"),
            Err(StateError::Invalid)
        ));
        org.record_nonce([7; 32]).unwrap();
        assert!(matches!(org.record_nonce([7; 32]), Err(StateError::Replay)));
        drop(org);
        let mut org = DurableOrganization::open(dir.path(), "test-org", &store).unwrap();
        assert!(matches!(org.record_nonce([7; 32]), Err(StateError::Replay)));
        assert!(matches!(org.install_revocations(snapshot(&root, 1, &[5])), Err(StateError::Rollback)));
        assert!(matches!(org.fresh_snapshot(100), Err(StateError::Stale)));
        let mut tampered = snapshot(&root, 3, &[5]);
        tampered.signature[0] ^= 1;
        assert!(matches!(org.install_revocations(tampered), Err(StateError::Invalid)));
        assert!(matches!(
            org.install_revocations(snapshot(&root, 3, &[])),
            Err(StateError::RevocationRegression)
        ));
    }
}
