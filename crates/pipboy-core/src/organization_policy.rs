//! Structural *preflight* for proposed organization-scoped RB operations.
//!
//! This module cannot authenticate a person, device, organization, signature,
//! grant issuer or clock. In particular, `CandidateForVerification` is NEVER a permission to
//! send, read, assign or synchronize. Do not wire this result into the live
//! task routes or SSH bridge until independent verification is implemented.
use crate::organization::{
    Appointment, Dataset, DeviceLifecycle, Grant, GrantAction, GrantSubject, OrgGraph,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preflight {
    /// Scope metadata matches. Cryptographic identity, issuer authority and
    /// trusted time still have to be verified by a separate policy boundary.
    CandidateForVerification,
    InvalidGraph,
    UnknownScope,
    OperatorUnavailable,
    DeviceUnavailable,
    AppointmentUnavailable,
    OperatorGrantUnavailable,
    DeviceGrantUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProposedExchange<'a> {
    pub organization_id: &'a str,
    pub vault_id: &'a str,
    pub operator_id: &'a str,
    pub device_id: &'a str,
    pub action: GrantAction,
    pub dataset: Dataset,
    /// Supplied by caller, NOT guaranteed to be a trusted time source.
    pub time_unverified: u64,
}

fn active_appointment(
    appointment: &Appointment,
    query: &ProposedExchange<'_>,
    units: &OrgGraph,
) -> bool {
    !appointment.revoked
        && appointment.operator_id == query.operator_id
        && appointment.not_before <= query.time_unverified
        && query.time_unverified < appointment.expires_at
        && units
            .units
            .iter()
            .any(|unit| unit.id == appointment.unit_id && unit.vault_id == query.vault_id)
}

fn active_grant(grant: &Grant, query: &ProposedExchange<'_>) -> bool {
    !grant.revoked
        && grant.organization_id == query.organization_id
        && grant.vault_id == query.vault_id
        && grant.action == query.action
        && grant.dataset == query.dataset
        && grant.not_before <= query.time_unverified
        && query.time_unverified < grant.expires_at
}

/// Checks whether a *claimed* operator/device pair could proceed to verified
/// identity/authorization review for one dataset and one Vault. No transitive
/// permission is derived from a reporting manager, division, role or device.
/// Both the person and the device need separately scoped draft grants.
pub fn preflight(graph: &OrgGraph, query: &ProposedExchange<'_>) -> Preflight {
    if graph.validate().is_err() {
        return Preflight::InvalidGraph;
    }
    if !graph
        .vaults
        .iter()
        .any(|site| site.id == query.vault_id && site.organization_id == query.organization_id)
    {
        return Preflight::UnknownScope;
    }
    if !graph.operators.iter().any(|person| {
        person.id == query.operator_id
            && person.organization_id == query.organization_id
            && person.active
    }) {
        return Preflight::OperatorUnavailable;
    }
    if !graph.devices.iter().any(|device| {
        device.id == query.device_id
            && device.organization_id == query.organization_id
            && device.vault_id == query.vault_id
            && device.assigned_operator_id.as_deref() == Some(query.operator_id)
            && device.lifecycle == DeviceLifecycle::RecordedActive
    }) {
        return Preflight::DeviceUnavailable;
    }
    if !graph
        .appointments
        .iter()
        .any(|appointment| active_appointment(appointment, query, graph))
    {
        return Preflight::AppointmentUnavailable;
    }
    if !graph.grants.iter().any(|grant| {
        active_grant(grant, query)
            && matches!(
                &grant.subject,
                GrantSubject::Operator(id) if id.as_str() == query.operator_id
            )
    }) {
        return Preflight::OperatorGrantUnavailable;
    }
    if !graph.grants.iter().any(|grant| {
        active_grant(grant, query)
            && matches!(
                &grant.subject,
                GrantSubject::Device(id) if id.as_str() == query.device_id
            )
    }) {
        return Preflight::DeviceGrantUnavailable;
    }
    Preflight::CandidateForVerification
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::{
        Appointment, DeviceRecord, Organization, Operator, Unit, UnitKind, VaultSite,
    };

    fn test_graph() -> OrgGraph {
        OrgGraph {
            schema_version: 1,
            organizations: vec![
                Organization {
                    id: "first-org".into(),
                    name: "First".into(),
                },
                Organization {
                    id: "other-org".into(),
                    name: "Other".into(),
                },
            ],
            vaults: vec![
                VaultSite {
                    id: "north-vault".into(),
                    organization_id: "first-org".into(),
                    name: "North".into(),
                },
                VaultSite {
                    id: "south-vault".into(),
                    organization_id: "first-org".into(),
                    name: "South".into(),
                },
                VaultSite {
                    id: "other-vault".into(),
                    organization_id: "other-org".into(),
                    name: "Other".into(),
                },
            ],
            units: vec![
                Unit {
                    id: "north-security".into(),
                    vault_id: "north-vault".into(),
                    parent_unit_id: None,
                    kind: UnitKind::Division,
                    name: "Security".into(),
                },
                Unit {
                    id: "north-recon".into(),
                    vault_id: "north-vault".into(),
                    parent_unit_id: Some("north-security".into()),
                    kind: UnitKind::Team,
                    name: "Recon".into(),
                },
                Unit {
                    id: "south-ops".into(),
                    vault_id: "south-vault".into(),
                    parent_unit_id: None,
                    kind: UnitKind::Division,
                    name: "Operations".into(),
                },
            ],
            operators: vec![
                Operator {
                    id: "scout-one".into(),
                    organization_id: "first-org".into(),
                    display_name: "Scout".into(),
                    active: true,
                },
                Operator {
                    id: "commander".into(),
                    organization_id: "first-org".into(),
                    display_name: "Commander".into(),
                    active: true,
                },
                Operator {
                    id: "other-person".into(),
                    organization_id: "other-org".into(),
                    display_name: "Other".into(),
                    active: true,
                },
            ],
            appointments: vec![Appointment {
                id: "scout-shift".into(),
                operator_id: "scout-one".into(),
                unit_id: "north-recon".into(),
                position_id: "recon".into(),
                approved_by_operator_id: "commander".into(),
                not_before: 10,
                expires_at: 50,
                revoked: false,
            }],
            devices: vec![DeviceRecord {
                id: "scout-device".into(),
                organization_id: "first-org".into(),
                vault_id: "north-vault".into(),
                assigned_operator_id: Some("scout-one".into()),
                lifecycle: DeviceLifecycle::RecordedActive,
            }],
            grants: vec![
                Grant {
                    id: "scout-grant".into(),
                    organization_id: "first-org".into(),
                    issuer_operator_id: "commander".into(),
                    subject: GrantSubject::Operator("scout-one".into()),
                    vault_id: "north-vault".into(),
                    action: GrantAction::Update,
                    dataset: Dataset::Progress,
                    not_before: 10,
                    expires_at: 100,
                    revoked: false,
                },
                Grant {
                    id: "device-grant".into(),
                    organization_id: "first-org".into(),
                    issuer_operator_id: "commander".into(),
                    subject: GrantSubject::Device("scout-device".into()),
                    vault_id: "north-vault".into(),
                    action: GrantAction::Update,
                    dataset: Dataset::Progress,
                    not_before: 10,
                    expires_at: 100,
                    revoked: false,
                },
            ],
        }
    }

    fn query<'a>() -> ProposedExchange<'a> {
        ProposedExchange {
            organization_id: "first-org",
            vault_id: "north-vault",
            operator_id: "scout-one",
            device_id: "scout-device",
            action: GrantAction::Update,
            dataset: Dataset::Progress,
            time_unverified: 20,
        }
    }

    #[test]
    fn candidate_is_not_a_permission_and_requires_both_scoped_grants() {
        let mut graph = test_graph();
        assert_eq!(preflight(&graph, &query()), Preflight::CandidateForVerification);
        graph.grants.pop();
        assert_eq!(preflight(&graph, &query()), Preflight::DeviceGrantUnavailable);
        graph.grants.clear();
        assert_eq!(preflight(&graph, &query()), Preflight::OperatorGrantUnavailable);
    }

    #[test]
    fn revocation_and_inactive_membership_fail_closed() {
        let mut graph = test_graph();
        graph.devices[0].lifecycle = DeviceLifecycle::Revoked;
        assert_eq!(preflight(&graph, &query()), Preflight::DeviceUnavailable);
        let mut graph = test_graph();
        graph.operators[0].active = false;
        assert_eq!(preflight(&graph, &query()), Preflight::OperatorUnavailable);
        let mut graph = test_graph();
        graph.appointments[0].revoked = true;
        assert_eq!(preflight(&graph, &query()), Preflight::AppointmentUnavailable);
        let mut graph = test_graph();
        graph.grants[0].revoked = true;
        assert_eq!(preflight(&graph, &query()), Preflight::OperatorGrantUnavailable);
        let mut graph = test_graph();
        graph.grants[1].revoked = true;
        assert_eq!(preflight(&graph, &query()), Preflight::DeviceGrantUnavailable);
    }

    #[test]
    fn shift_boundary_and_grant_expiry_are_exclusive() {
        let graph = test_graph();
        let mut q = query();
        q.time_unverified = 9;
        assert_eq!(preflight(&graph, &q), Preflight::AppointmentUnavailable);
        q.time_unverified = 10;
        assert_eq!(preflight(&graph, &q), Preflight::CandidateForVerification);
        q.time_unverified = 50;
        assert_eq!(preflight(&graph, &q), Preflight::AppointmentUnavailable);
        let mut graph = test_graph();
        graph.appointments[0].expires_at = 150;
        q.time_unverified = 100;
        assert_eq!(preflight(&graph, &q), Preflight::OperatorGrantUnavailable);
    }

    #[test]
    fn no_inherited_manager_or_cross_vault_privileges() {
        let graph = test_graph();
        let mut q = query();
        q.vault_id = "south-vault";
        assert_eq!(preflight(&graph, &q), Preflight::DeviceUnavailable);
        q = query();
        q.action = GrantAction::Assign;
        assert_eq!(preflight(&graph, &q), Preflight::OperatorGrantUnavailable);
        q = query();
        q.dataset = Dataset::Site;
        assert_eq!(preflight(&graph, &q), Preflight::OperatorGrantUnavailable);
        q = query();
        q.operator_id = "commander";
        assert_eq!(preflight(&graph, &q), Preflight::DeviceUnavailable);
        q = query();
        q.organization_id = "other-org";
        assert_eq!(preflight(&graph, &q), Preflight::UnknownScope);
    }

    #[test]
    fn invalid_graph_and_cross_tenant_attempts_rejected() {
        let mut graph = test_graph();
        graph.units[1].vault_id = "south-vault".into();
        assert_eq!(preflight(&graph, &query()), Preflight::InvalidGraph);
        let mut graph = test_graph();
        graph.devices[0].assigned_operator_id = Some("other-person".into());
        assert_eq!(preflight(&graph, &query()), Preflight::InvalidGraph);
        let mut graph = test_graph();
        graph.grants[0].organization_id = "other-org".into();
        assert_eq!(preflight(&graph, &query()), Preflight::InvalidGraph);
    }

    #[test]
    fn separated_untrusted_time_never_opens_an_ssh_session() {
        let graph = test_graph();
        let mut q = query();
        q.time_unverified = u64::MAX;
        assert_eq!(preflight(&graph, &q), Preflight::AppointmentUnavailable);
        assert_eq!(graph.validate(), Ok(()));
        // The result is an explanatory metadata preflight only; it has no
        // authority to open a socket, sign a grant or mutate TaskStore.
    }
}
