//! ORG-3.1: offline verification of organizational approvals and revocation.
//!
//! The caller supplies the trust anchor, clock and locally persisted rollback
//! floor. None are established by this module. This is EVIDENCE ONLY: never
//! use a positive result as an API, file, network or task authorization.
//! No private keys, transport calls, storage, or automatic pairing are used.
use crate::organization::{
    Appointment, Dataset, Grant, GrantAction, GrantSubject, OrgGraph,
};
use crate::organization_identity::{
    verify_pair, EvidenceError, IdentityEvidence, SessionProof, SignedCredential,
    VerificationContext,
};
use crate::organization_policy::ProposedExchange;
use ring::signature::{UnparsedPublicKey, ED25519};
use serde::{Deserialize, Serialize};

const ROOT_BYTES: usize = 32;
const SIGNATURE_BYTES: usize = 64;
const NONCE_BYTES: usize = 32;
const MAX_REVOKED: usize = 1024;
const MAX_SNAPSHOT_SECONDS: u64 = 86_400;
const APPOINTMENT_DOMAIN: &[u8] = b"rust-boy.org.appointment.v1\0";
const GRANT_DOMAIN: &[u8] = b"rust-boy.org.grant.v1\0";
const REVOCATION_DOMAIN: &[u8] = b"rust-boy.org.revocation.v1\0";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedAppointment {
    pub organization_id: String,
    pub vault_id: String,
    pub issuer_key_epoch: u64,
    pub approval_serial: u64,
    pub appointment: Appointment,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedGrant {
    pub issuer_key_epoch: u64,
    pub approval_serial: u64,
    pub grant: Grant,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedRevocations {
    pub schema_version: u32,
    pub organization_id: String,
    pub issuer_key_epoch: u64,
    pub generation: u64,
    pub not_before: u64,
    pub expires_at: u64,
    pub credential_serials: Vec<u64>,
    pub approval_serials: Vec<u64>,
    pub signature: Vec<u8>,
}

/// All fields are inputs supplied by a FUTURE trusted enrollment/session
/// service, not by a peer. Durable generation floors and nonce storage are
/// required in production, but not implemented by this pure Rust module.
pub struct AuthorityContext<'a> {
    pub pinned_root_key: &'a [u8],
    pub issuer_key_epoch: u64,
    pub verified_now: u64,
    pub minimum_revocation_generation: u64,
    pub verifier_nonce: &'a [u8; NONCE_BYTES],
    pub authenticated_channel_binding: &'a [u8; NONCE_BYTES],
}

pub struct ApprovalBundle<'a> {
    pub appointment: &'a SignedAppointment,
    pub operator_grant: &'a SignedGrant,
    pub device_grant: &'a SignedGrant,
    pub revocations: &'a SignedRevocations,
    pub operator_credential: &'a SignedCredential,
    pub operator_proof: &'a SessionProof,
    pub device_credential: &'a SignedCredential,
    pub device_proof: &'a SessionProof,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnrollmentEvidence {
    pub identity: IdentityEvidence,
    pub revocation_generation: u64,
    pub approval_serials: [u64; 3],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthorityError {
    ContextUnavailable,
    InvalidRevocations,
    ExpiredRevocations,
    RevocationRollback,
    InvalidApproval,
    ApprovalRevoked,
    ScopeMismatch,
    StaleApproval,
    Identity(EvidenceError),
    ReplayDetected,
    ReplayCapacityReached,
}

fn add_str(output: &mut Vec<u8>, value: &str) {
    output.extend_from_slice(&(value.len() as u32).to_be_bytes());
    output.extend_from_slice(value.as_bytes());
}

fn add_bool(output: &mut Vec<u8>, value: bool) {
    output.push(u8::from(value));
}

fn encode_appointment(a: &SignedAppointment) -> Vec<u8> {
    let mut out = APPOINTMENT_DOMAIN.to_vec();
    add_str(&mut out, &a.organization_id);
    add_str(&mut out, &a.vault_id);
    out.extend_from_slice(&a.issuer_key_epoch.to_be_bytes());
    out.extend_from_slice(&a.approval_serial.to_be_bytes());
    add_str(&mut out, &a.appointment.id);
    add_str(&mut out, &a.appointment.operator_id);
    add_str(&mut out, &a.appointment.unit_id);
    add_str(&mut out, &a.appointment.position_id);
    add_str(&mut out, &a.appointment.approved_by_operator_id);
    out.extend_from_slice(&a.appointment.not_before.to_be_bytes());
    out.extend_from_slice(&a.appointment.expires_at.to_be_bytes());
    add_bool(&mut out, a.appointment.revoked);
    out
}

fn encode_grant(g: &SignedGrant) -> Vec<u8> {
    let mut out = GRANT_DOMAIN.to_vec();
    out.extend_from_slice(&g.issuer_key_epoch.to_be_bytes());
    out.extend_from_slice(&g.approval_serial.to_be_bytes());
    add_str(&mut out, &g.grant.id);
    add_str(&mut out, &g.grant.organization_id);
    add_str(&mut out, &g.grant.issuer_operator_id);
    match &g.grant.subject {
        GrantSubject::Operator(id) => {
            out.push(1);
            add_str(&mut out, id);
        }
        GrantSubject::Device(id) => {
            out.push(2);
            add_str(&mut out, id);
        }
    }
    add_str(&mut out, &g.grant.vault_id);
    out.push(match g.grant.action {
        GrantAction::View => 1,
        GrantAction::Assign => 2,
        GrantAction::Update => 3,
        GrantAction::Sync => 4,
    });
    out.push(match g.grant.dataset {
        Dataset::Mission => 1,
        Dataset::Progress => 2,
        Dataset::Message => 3,
        Dataset::Site => 4,
    });
    out.extend_from_slice(&g.grant.not_before.to_be_bytes());
    out.extend_from_slice(&g.grant.expires_at.to_be_bytes());
    add_bool(&mut out, g.grant.revoked);
    out
}

fn encode_revocations(r: &SignedRevocations) -> Vec<u8> {
    let mut out = REVOCATION_DOMAIN.to_vec();
    out.extend_from_slice(&r.schema_version.to_be_bytes());
    add_str(&mut out, &r.organization_id);
    out.extend_from_slice(&r.issuer_key_epoch.to_be_bytes());
    out.extend_from_slice(&r.generation.to_be_bytes());
    out.extend_from_slice(&r.not_before.to_be_bytes());
    out.extend_from_slice(&r.expires_at.to_be_bytes());
    out.extend_from_slice(&(r.credential_serials.len() as u32).to_be_bytes());
    for serial in &r.credential_serials {
        out.extend_from_slice(&serial.to_be_bytes());
    }
    out.extend_from_slice(&(r.approval_serials.len() as u32).to_be_bytes());
    for serial in &r.approval_serials {
        out.extend_from_slice(&serial.to_be_bytes());
    }
    out
}

fn sorted_unique_nonzero(values: &[u64]) -> bool {
    values.iter().all(|n| *n != 0)
        && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn check_signature(key: &[u8], bytes: &[u8], signature: &[u8]) -> bool {
    signature.len() == SIGNATURE_BYTES
        && UnparsedPublicKey::new(&ED25519, key)
            .verify(bytes, signature)
            .is_ok()
}

fn verify_revocations(
    query: &ProposedExchange<'_>,
    ctx: &AuthorityContext<'_>,
    snapshot: &SignedRevocations,
) -> Result<(), AuthorityError> {
    if snapshot.schema_version != 1
        || snapshot.organization_id != query.organization_id
        || snapshot.issuer_key_epoch != ctx.issuer_key_epoch
        || snapshot.generation == 0
        || snapshot.not_before >= snapshot.expires_at
        || snapshot.expires_at - snapshot.not_before > MAX_SNAPSHOT_SECONDS
        || snapshot.credential_serials.len() > MAX_REVOKED
        || snapshot.approval_serials.len() > MAX_REVOKED
        || !sorted_unique_nonzero(&snapshot.credential_serials)
        || !sorted_unique_nonzero(&snapshot.approval_serials)
        || !check_signature(
            ctx.pinned_root_key,
            &encode_revocations(snapshot),
            &snapshot.signature,
        )
    {
        return Err(AuthorityError::InvalidRevocations);
    }
    if snapshot.generation < ctx.minimum_revocation_generation {
        return Err(AuthorityError::RevocationRollback);
    }
    if ctx.verified_now < snapshot.not_before || ctx.verified_now >= snapshot.expires_at {
        return Err(AuthorityError::ExpiredRevocations);
    }
    Ok(())
}

fn verify_appointment(
    graph: &OrgGraph,
    query: &ProposedExchange<'_>,
    ctx: &AuthorityContext<'_>,
    snapshot: &SignedRevocations,
    signed: &SignedAppointment,
) -> Result<(), AuthorityError> {
    let a = &signed.appointment;
    if signed.approval_serial == 0
        || signed.issuer_key_epoch != ctx.issuer_key_epoch
        || signed.organization_id != query.organization_id
        || signed.vault_id != query.vault_id
        || a.operator_id != query.operator_id
        || !graph
            .units
            .iter()
            .any(|unit| unit.id == a.unit_id && unit.vault_id == query.vault_id)
    {
        return Err(AuthorityError::ScopeMismatch);
    }
    if snapshot.approval_serials.contains(&signed.approval_serial) {
        return Err(AuthorityError::ApprovalRevoked);
    }
    if !graph.appointments.contains(a) || a.revoked {
        return Err(AuthorityError::StaleApproval);
    }
    if !check_signature(
        ctx.pinned_root_key,
        &encode_appointment(signed),
        &signed.signature,
    ) {
        return Err(AuthorityError::InvalidApproval);
    }
    Ok(())
}

fn verify_grant(
    graph: &OrgGraph,
    query: &ProposedExchange<'_>,
    ctx: &AuthorityContext<'_>,
    snapshot: &SignedRevocations,
    signed: &SignedGrant,
    expected: &GrantSubject,
) -> Result<(), AuthorityError> {
    let grant = &signed.grant;
    if signed.approval_serial == 0
        || signed.issuer_key_epoch != ctx.issuer_key_epoch
        || grant.organization_id != query.organization_id
        || grant.vault_id != query.vault_id
        || grant.action != query.action
        || grant.dataset != query.dataset
        || &grant.subject != expected
    {
        return Err(AuthorityError::ScopeMismatch);
    }
    if snapshot.approval_serials.contains(&signed.approval_serial) {
        return Err(AuthorityError::ApprovalRevoked);
    }
    if !graph.grants.contains(grant) || grant.revoked {
        return Err(AuthorityError::StaleApproval);
    }
    if !check_signature(ctx.pinned_root_key, &encode_grant(signed), &signed.signature) {
        return Err(AuthorityError::InvalidApproval);
    }
    Ok(())
}

/// Memory-only demonstration of one-time challenges; NOT durable replay
/// protection. Production MUST atomically persist spent nonces and the
/// revocation generation floor before acknowledging an authorized session.
#[derive(Default)]
pub struct VolatileNonceJournal {
    used: std::collections::HashSet<[u8; NONCE_BYTES]>,
}

impl VolatileNonceJournal {
    pub fn check_and_record(&mut self, nonce: [u8; NONCE_BYTES]) -> Result<(), AuthorityError> {
        if self.used.contains(&nonce) {
            return Err(AuthorityError::ReplayDetected);
        }
        if self.used.len() >= 1024 {
            return Err(AuthorityError::ReplayCapacityReached);
        }
        self.used.insert(nonce);
        Ok(())
    }
}

/// Verifies independent operator/device possession + signed approvals + a
/// bounded signed revocation snapshot. Returns evidence, NEVER permission.
/// This function is intentionally not reachable from any live Rust-Boy route.
pub fn verify_enrollment_evidence(
    graph: &OrgGraph,
    query: &ProposedExchange<'_>,
    ctx: &AuthorityContext<'_>,
    bundle: &ApprovalBundle<'_>,
    nonce_journal: &mut VolatileNonceJournal,
) -> Result<EnrollmentEvidence, AuthorityError> {
    if ctx.pinned_root_key.len() != ROOT_BYTES
        || ctx.issuer_key_epoch == 0
        || ctx.minimum_revocation_generation == 0
        || ctx.verifier_nonce.iter().all(|b| *b == 0)
        || ctx.authenticated_channel_binding.iter().all(|b| *b == 0)
    {
        return Err(AuthorityError::ContextUnavailable);
    }
    verify_revocations(query, ctx, bundle.revocations)?;
    let verification = VerificationContext {
        organization_public_key: ctx.pinned_root_key,
        verified_now: ctx.verified_now,
        nonce: ctx.verifier_nonce,
        channel_binding: ctx.authenticated_channel_binding,
        revoked_serials: &bundle.revocations.credential_serials,
    };
    let identity = verify_pair(
        graph,
        query,
        &verification,
        (bundle.operator_credential, bundle.operator_proof),
        (bundle.device_credential, bundle.device_proof),
    )
    .map_err(AuthorityError::Identity)?;
    let serials = [
        bundle.appointment.approval_serial,
        bundle.operator_grant.approval_serial,
        bundle.device_grant.approval_serial,
    ];
    if serials.iter().any(|n| *n == 0)
        || serials[0] == serials[1]
        || serials[0] == serials[2]
        || serials[1] == serials[2]
    {
        return Err(AuthorityError::InvalidApproval);
    }
    verify_appointment(graph, query, ctx, bundle.revocations, bundle.appointment)?;
    verify_grant(
        graph,
        query,
        ctx,
        bundle.revocations,
        bundle.operator_grant,
        &GrantSubject::Operator(query.operator_id.into()),
    )?;
    verify_grant(
        graph,
        query,
        ctx,
        bundle.revocations,
        bundle.device_grant,
        &GrantSubject::Device(query.device_id.into()),
    )?;
    nonce_journal.check_and_record(*ctx.verifier_nonce)?;
    Ok(EnrollmentEvidence {
        identity,
        revocation_generation: bundle.revocations.generation,
        approval_serials: serials,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::{
        DeviceLifecycle, DeviceRecord, Operator, Organization, Unit, UnitKind, VaultSite,
    };
    use crate::organization_identity::{
        credential_transcript, proof_transcript, PrincipalKind,
    };
    use ring::signature::{Ed25519KeyPair, KeyPair};

    const NONCE: [u8; 32] = [21; 32];
    const BINDING: [u8; 32] = [24; 32];

    fn key(seed: u8) -> Ed25519KeyPair {
        // Fixed seeds are deliberately limited to synthetic test-only fixtures.
        Ed25519KeyPair::from_seed_unchecked(&[seed; 32]).unwrap()
    }

    fn query() -> ProposedExchange<'static> {
        ProposedExchange {
            organization_id: "sample-org",
            vault_id: "sample-vault",
            operator_id: "field-operator",
            device_id: "field-device",
            action: GrantAction::Sync,
            dataset: Dataset::Progress,
            time_unverified: 11,
        }
    }

    fn graph() -> OrgGraph {
        OrgGraph {
            schema_version: 1,
            organizations: vec![Organization {
                id: "sample-org".into(),
                name: "Synthetic Organization".into(),
            }],
            vaults: vec![VaultSite {
                id: "sample-vault".into(),
                organization_id: "sample-org".into(),
                name: "Sample Vault".into(),
            }],
            units: vec![Unit {
                id: "field-division".into(),
                vault_id: "sample-vault".into(),
                parent_unit_id: None,
                kind: UnitKind::Division,
                name: "Field Division".into(),
            }],
            operators: vec![
                Operator {
                    id: "field-operator".into(),
                    organization_id: "sample-org".into(),
                    display_name: "Operator".into(),
                    active: true,
                },
                Operator {
                    id: "command-issuer".into(),
                    organization_id: "sample-org".into(),
                    display_name: "Issuer".into(),
                    active: true,
                },
            ],
            appointments: vec![Appointment {
                id: "shift-one".into(),
                operator_id: "field-operator".into(),
                unit_id: "field-division".into(),
                position_id: "recon".into(),
                approved_by_operator_id: "command-issuer".into(),
                not_before: 10,
                expires_at: 100,
                revoked: false,
            }],
            devices: vec![DeviceRecord {
                id: "field-device".into(),
                organization_id: "sample-org".into(),
                vault_id: "sample-vault".into(),
                assigned_operator_id: Some("field-operator".into()),
                lifecycle: DeviceLifecycle::RecordedActive,
            }],
            grants: vec![
                Grant {
                    id: "operator-grant".into(),
                    organization_id: "sample-org".into(),
                    issuer_operator_id: "command-issuer".into(),
                    subject: GrantSubject::Operator("field-operator".into()),
                    vault_id: "sample-vault".into(),
                    action: GrantAction::Sync,
                    dataset: Dataset::Progress,
                    not_before: 10,
                    expires_at: 100,
                    revoked: false,
                },
                Grant {
                    id: "device-grant".into(),
                    organization_id: "sample-org".into(),
                    issuer_operator_id: "command-issuer".into(),
                    subject: GrantSubject::Device("field-device".into()),
                    vault_id: "sample-vault".into(),
                    action: GrantAction::Sync,
                    dataset: Dataset::Progress,
                    not_before: 10,
                    expires_at: 100,
                    revoked: false,
                },
            ],
        }
    }

    fn credential(
        root: &Ed25519KeyPair,
        holder: &Ed25519KeyPair,
        kind: PrincipalKind,
        principal_id: &str,
        serial: u64,
    ) -> SignedCredential {
        let mut signed = SignedCredential {
            schema_version: 1,
            organization_id: "sample-org".into(),
            vault_id: "sample-vault".into(),
            principal_kind: kind,
            principal_id: principal_id.into(),
            serial,
            key_epoch: 1,
            not_before: 10,
            expires_at: 100,
            public_key: holder.public_key().as_ref().to_vec(),
            issuer_signature: Vec::new(),
        };
        signed.issuer_signature = root.sign(&credential_transcript(&signed)).as_ref().to_vec();
        signed
    }

    fn proof(
        holder: &Ed25519KeyPair,
        claim: &SignedCredential,
        q: &ProposedExchange<'_>,
    ) -> SessionProof {
        SessionProof {
            signature: holder
                .sign(&proof_transcript(q, claim, &NONCE, &BINDING))
                .as_ref()
                .to_vec(),
        }
    }

    struct Fixture {
        root: Ed25519KeyPair,
        graph: OrgGraph,
        appointment: SignedAppointment,
        operator_grant: SignedGrant,
        device_grant: SignedGrant,
        revocations: SignedRevocations,
        operator_credential: SignedCredential,
        device_credential: SignedCredential,
        operator_proof: SessionProof,
        device_proof: SessionProof,
    }

    impl Fixture {
        fn sign_revocations(&mut self) {
            self.revocations.signature = self
                .root
                .sign(&encode_revocations(&self.revocations))
                .as_ref()
                .to_vec();
        }

        fn check(
            &self,
            now: u64,
            min_generation: u64,
            nonce: &[u8; 32],
            journal: &mut VolatileNonceJournal,
        ) -> Result<EnrollmentEvidence, AuthorityError> {
            let context = AuthorityContext {
                pinned_root_key: self.root.public_key().as_ref(),
                issuer_key_epoch: 1,
                verified_now: now,
                minimum_revocation_generation: min_generation,
                verifier_nonce: nonce,
                authenticated_channel_binding: &BINDING,
            };
            let approvals = ApprovalBundle {
                appointment: &self.appointment,
                operator_grant: &self.operator_grant,
                device_grant: &self.device_grant,
                revocations: &self.revocations,
                operator_credential: &self.operator_credential,
                operator_proof: &self.operator_proof,
                device_credential: &self.device_credential,
                device_proof: &self.device_proof,
            };
            verify_enrollment_evidence(&self.graph, &query(), &context, &approvals, journal)
        }
    }

    fn fixture() -> Fixture {
        let root = key(1);
        let operator_key = key(2);
        let device_key = key(3);
        let graph = graph();
        let q = query();
        let operator_credential = credential(
            &root,
            &operator_key,
            PrincipalKind::Operator,
            q.operator_id,
            1,
        );
        let device_credential = credential(
            &root,
            &device_key,
            PrincipalKind::Device,
            q.device_id,
            2,
        );
        let operator_proof = proof(&operator_key, &operator_credential, &q);
        let device_proof = proof(&device_key, &device_credential, &q);
        let mut appointment = SignedAppointment {
            organization_id: q.organization_id.into(),
            vault_id: q.vault_id.into(),
            issuer_key_epoch: 1,
            approval_serial: 10,
            appointment: graph.appointments[0].clone(),
            signature: Vec::new(),
        };
        appointment.signature = root.sign(&encode_appointment(&appointment)).as_ref().to_vec();
        let mut operator_grant = SignedGrant {
            issuer_key_epoch: 1,
            approval_serial: 11,
            grant: graph.grants[0].clone(),
            signature: Vec::new(),
        };
        operator_grant.signature = root.sign(&encode_grant(&operator_grant)).as_ref().to_vec();
        let mut device_grant = SignedGrant {
            issuer_key_epoch: 1,
            approval_serial: 12,
            grant: graph.grants[1].clone(),
            signature: Vec::new(),
        };
        device_grant.signature = root.sign(&encode_grant(&device_grant)).as_ref().to_vec();
        let mut f = Fixture {
            root,
            graph,
            appointment,
            operator_grant,
            device_grant,
            revocations: SignedRevocations {
                schema_version: 1,
                organization_id: "sample-org".into(),
                issuer_key_epoch: 1,
                generation: 2,
                not_before: 10,
                expires_at: 200,
                credential_serials: Vec::new(),
                approval_serials: Vec::new(),
                signature: Vec::new(),
            },
            operator_credential,
            device_credential,
            operator_proof,
            device_proof,
        };
        f.sign_revocations();
        f
    }

    #[test]
    fn verified_signatures_produce_evidence_not_authorization() {
        let f = fixture();
        let mut journal = VolatileNonceJournal::default();
        let evidence = f.check(20, 2, &NONCE, &mut journal).unwrap();
        assert_eq!(evidence.identity.device_id, "field-device");
        assert_eq!(evidence.approval_serials, [10, 11, 12]);
        assert_eq!(evidence.revocation_generation, 2);
        assert_eq!(
            f.check(20, 2, &NONCE, &mut journal),
            Err(AuthorityError::ReplayDetected)
        );
    }

    #[test]
    fn tampered_appointment_grant_and_unsigned_edits_fail_closed() {
        let mut f = fixture();
        f.appointment.appointment.position_id = "overseer".into();
        assert_eq!(
            f.check(20, 2, &NONCE, &mut VolatileNonceJournal::default()),
            Err(AuthorityError::StaleApproval)
        );
        let mut f = fixture();
        f.operator_grant.signature[1] ^= 1;
        assert_eq!(
            f.check(20, 2, &NONCE, &mut VolatileNonceJournal::default()),
            Err(AuthorityError::InvalidApproval)
        );
        let mut f = fixture();
        f.graph.grants[1].action = GrantAction::Assign;
        assert_eq!(
            f.check(20, 2, &NONCE, &mut VolatileNonceJournal::default()),
            Err(AuthorityError::Identity(EvidenceError::StructuralPreflight))
        );
    }

    #[test]
    fn revoked_credentials_and_approvals_fail_closed() {
        let mut f = fixture();
        f.revocations.credential_serials.push(2);
        f.sign_revocations();
        assert_eq!(
            f.check(20, 2, &NONCE, &mut VolatileNonceJournal::default()),
            Err(AuthorityError::Identity(EvidenceError::Revoked))
        );
        let mut f = fixture();
        f.revocations.approval_serials.push(12);
        f.sign_revocations();
        assert_eq!(
            f.check(20, 2, &NONCE, &mut VolatileNonceJournal::default()),
            Err(AuthorityError::ApprovalRevoked)
        );
        let mut f = fixture();
        f.revocations.approval_serials.extend([12, 10]);
        f.sign_revocations();
        assert_eq!(
            f.check(20, 2, &NONCE, &mut VolatileNonceJournal::default()),
            Err(AuthorityError::InvalidRevocations)
        );
    }

    #[test]
    fn freshness_snapshot_signature_and_generation_floor_enforced() {
        let f = fixture();
        assert_eq!(
            f.check(20, 3, &NONCE, &mut VolatileNonceJournal::default()),
            Err(AuthorityError::RevocationRollback)
        );
        assert_eq!(
            f.check(200, 2, &NONCE, &mut VolatileNonceJournal::default()),
            Err(AuthorityError::ExpiredRevocations)
        );
        let mut f = fixture();
        f.revocations.signature[0] ^= 1;
        assert_eq!(
            f.check(20, 2, &NONCE, &mut VolatileNonceJournal::default()),
            Err(AuthorityError::InvalidRevocations)
        );
        let mut f = fixture();
        f.revocations.expires_at = 10 + MAX_SNAPSHOT_SECONDS + 1;
        f.sign_revocations();
        assert_eq!(
            f.check(20, 2, &NONCE, &mut VolatileNonceJournal::default()),
            Err(AuthorityError::InvalidRevocations)
        );
    }

    #[test]
    fn failed_evidence_does_not_consume_nonce_and_valid_replay_is_rejected() {
        let mut f = fixture();
        let mut journal = VolatileNonceJournal::default();
        f.device_grant.signature[0] ^= 1;
        assert_eq!(
            f.check(20, 2, &NONCE, &mut journal),
            Err(AuthorityError::InvalidApproval)
        );
        f.device_grant.signature[0] ^= 1;
        assert!(f.check(20, 2, &NONCE, &mut journal).is_ok());
        assert_eq!(
            f.check(20, 2, &NONCE, &mut journal),
            Err(AuthorityError::ReplayDetected)
        );
    }

    #[test]
    fn cross_vault_and_delegated_assignment_require_exact_approval() {
        let mut f = fixture();
        f.device_grant.grant.vault_id = "other-vault".into();
        assert_eq!(
            f.check(20, 2, &NONCE, &mut VolatileNonceJournal::default()),
            Err(AuthorityError::ScopeMismatch)
        );
        let mut f = fixture();
        f.graph.appointments[0].revoked = true;
        assert_eq!(
            f.check(20, 2, &NONCE, &mut VolatileNonceJournal::default()),
            Err(AuthorityError::Identity(EvidenceError::StructuralPreflight))
        );
    }

    #[test]
    fn nonce_journal_is_bounded_and_not_a_persistent_production_store() {
        let mut journal = VolatileNonceJournal::default();
        for counter in 0..1024_u32 {
            let mut nonce = [0_u8; 32];
            nonce[..4].copy_from_slice(&counter.to_be_bytes());
            assert!(journal.check_and_record(nonce).is_ok());
        }
        assert_eq!(
            journal.check_and_record([255; 32]),
            Err(AuthorityError::ReplayCapacityReached)
        );
        let mut fresh_process = VolatileNonceJournal::default();
        assert!(fresh_process.check_and_record([0; 32]).is_ok());
    }
}
