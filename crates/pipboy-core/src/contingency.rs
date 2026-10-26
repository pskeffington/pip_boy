//! Radio and local-link contingency policy core. No I/O or RF side effects.
//! Proposed adapters remain disabled until separate authentication and hardware gates.
//! This is NOT a claim that Meshtastic, MeshCore or Reticulum are integrated.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bearer {
    PinnedSsh,
    AlternateIpSsh,
    MeshtasticUsb,
    MeshcoreUsb,
    ReticulumBridge,
    LocalMeshIp,
    OfflineHandoff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Readiness {
    CodeAvailable,
    Proposed,
    Research,
    ProcedureOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaceTier {
    Primary,
    Alternate,
    Contingency,
    Emergency,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BearerSpec {
    pub bearer: Bearer,
    pub tier: PaceTier,
    pub readiness: Readiness,
    pub estimated_payload_max: Option<usize>,
    pub supports_ip: bool,
    pub needs_explicit_peer_enrollment: bool,
    pub requires_separate_e2ee: bool,
    pub default_enabled: bool,
}

pub fn catalog() -> Vec<BearerSpec> {
    use Bearer::*;
    use PaceTier::*;
    use Readiness::*;
    vec![
        BearerSpec {
            bearer: PinnedSsh,
            tier: Primary,
            readiness: CodeAvailable,
            estimated_payload_max: Some(1024),
            supports_ip: true,
            needs_explicit_peer_enrollment: true,
            requires_separate_e2ee: true,
            default_enabled: false,
        },
        BearerSpec {
            bearer: AlternateIpSsh,
            tier: Alternate,
            readiness: Proposed,
            estimated_payload_max: Some(1024),
            supports_ip: true,
            needs_explicit_peer_enrollment: true,
            requires_separate_e2ee: true,
            default_enabled: false,
        },
        BearerSpec {
            bearer: MeshtasticUsb,
            tier: Contingency,
            readiness: Research,
            // Planning budget, NOT a claimed Meshtastic protobuf data ceiling.
            estimated_payload_max: Some(128),
            supports_ip: false,
            needs_explicit_peer_enrollment: true,
            requires_separate_e2ee: true,
            default_enabled: false,
        },
        BearerSpec {
            bearer: MeshcoreUsb,
            tier: Contingency,
            readiness: Research,
            estimated_payload_max: Some(100),
            supports_ip: false,
            needs_explicit_peer_enrollment: true,
            requires_separate_e2ee: true,
            default_enabled: false,
        },
        BearerSpec {
            bearer: ReticulumBridge,
            tier: Contingency,
            readiness: Research,
            estimated_payload_max: None,
            supports_ip: false,
            needs_explicit_peer_enrollment: true,
            requires_separate_e2ee: true,
            default_enabled: false,
        },
        BearerSpec {
            bearer: LocalMeshIp,
            tier: Alternate,
            readiness: Proposed,
            estimated_payload_max: None,
            supports_ip: true,
            needs_explicit_peer_enrollment: true,
            requires_separate_e2ee: true,
            default_enabled: false,
        },
        BearerSpec {
            bearer: OfflineHandoff,
            tier: Emergency,
            readiness: ProcedureOnly,
            estimated_payload_max: None,
            supports_ip: false,
            needs_explicit_peer_enrollment: true,
            requires_separate_e2ee: true,
            default_enabled: false,
        },
    ]
}

/// Deliberately fail closed. This function only evaluates eligibility; it
/// cannot activate a network or transmit data. Separate route verification
/// and a reviewed secure envelope must be implemented before any new bearer.
pub fn eligible(
    candidate: &BearerSpec,
    operator_approved: bool,
    paired_identity: bool,
    authenticated_envelope: bool,
    destination_verified: bool,
) -> bool {
    operator_approved
        && paired_identity
        && destination_verified
        && (!candidate.requires_separate_e2ee || authenticated_envelope)
        && candidate.readiness == Readiness::CodeAvailable
}

/// A routing ACK is not a durable end-recipient receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryEvidence {
    LocalQueued,
    LinkAccepted,
    RelayObserved,
    RecipientStored,
    HumanRead,
}

impl DeliveryEvidence {
    pub fn may_mark_stored(self) -> bool {
        matches!(self, Self::RecipientStored | Self::HumanRead)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_radio_or_proposed_route_can_be_mistaken_for_implemented_transport() {
        let list = catalog();
        assert_eq!(list.len(), 7);
        assert!(list.iter().all(|item| !item.default_enabled));
        assert!(list
            .iter()
            .filter(|item| item.bearer != Bearer::PinnedSsh)
            .all(|item| item.readiness != Readiness::CodeAvailable));
        assert!(list
            .iter()
            .all(|item| !eligible(item, true, true, false, true)));
        assert!(list
            .iter()
            .filter(|item| item.bearer != Bearer::PinnedSsh)
            .all(|item| !eligible(item, true, true, true, true)));
    }

    #[test]
    fn peer_enrollment_and_operator_consent_are_mandatory() {
        let baseline = catalog().remove(0);
        assert!(!eligible(&baseline, false, true, true, true));
        assert!(!eligible(&baseline, true, false, true, true));
        assert!(!eligible(&baseline, true, true, true, false));
        assert!(eligible(&baseline, true, true, true, true));
    }

    #[test]
    fn relay_or_radio_ack_does_not_imply_delivery() {
        for evidence in [
            DeliveryEvidence::LocalQueued,
            DeliveryEvidence::LinkAccepted,
            DeliveryEvidence::RelayObserved,
        ] {
            assert!(!evidence.may_mark_stored());
        }
        assert!(DeliveryEvidence::RecipientStored.may_mark_stored());
    }
}
