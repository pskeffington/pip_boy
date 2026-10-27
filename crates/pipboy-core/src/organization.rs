//! Pure, design-stage Vault organization records for future RB-to-RB routing.
//!
//! IMPORTANT: This graph is NOT an authenticator, a grant evaluator, a directory
//! service, or a trusted source of device enrollment. All records may be
//! self-asserted by their creator; no network, filesystem, clock, key, or
//! transport operations occur in this module. Existing workbench presets and
//! SSH task/message protocol are intentionally unchanged.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const ORG_SCHEMA: u32 = 1;
const MAX_ORGS: usize = 16;
const MAX_VAULTS: usize = 64;
const MAX_UNITS: usize = 256;
const MAX_OPERATORS: usize = 1024;
const MAX_APPOINTMENTS: usize = 4096;
const MAX_DEVICES: usize = 1024;
const MAX_GRANTS: usize = 4096;

fn id(value: &str) -> bool {
    (2..=64).contains(&value.len())
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == b'-')
}

fn name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value.trim() == value
        && value.chars().all(|ch| !ch.is_control())
}

fn unique<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = HashSet::new();
    values.into_iter().all(|value| seen.insert(value))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Organization {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultSite {
    pub id: String,
    pub organization_id: String,
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitKind {
    Division,
    Team,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub id: String,
    pub vault_id: String,
    pub parent_unit_id: Option<String>,
    pub kind: UnitKind,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operator {
    pub id: String,
    pub organization_id: String,
    pub display_name: String,
    pub active: bool,
}

/// Appointment state is informational until a separately authenticated
/// organizational authority verifies the signer and applicable policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Appointment {
    pub id: String,
    pub operator_id: String,
    pub unit_id: String,
    pub position_id: String,
    pub approved_by_operator_id: String,
    pub not_before: u64,
    pub expires_at: u64,
    pub revoked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceLifecycle {
    Unenrolled,
    Pending,
    RecordedActive,
    Suspended,
    Revoked,
}

/// RecordedActive is a roster label, not proof of cryptographic enrollment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceRecord {
    pub id: String,
    pub organization_id: String,
    pub vault_id: String,
    pub assigned_operator_id: Option<String>,
    pub lifecycle: DeviceLifecycle,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum GrantSubject {
    Operator(String),
    Device(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrantAction {
    View,
    Assign,
    Update,
    Sync,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dataset {
    Mission,
    Progress,
    Message,
    Site,
}

/// Draft policy only. Never authorize a task, data read or SSH session on this
/// serialized grant without an independently authenticated authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grant {
    pub id: String,
    pub organization_id: String,
    pub issuer_operator_id: String,
    pub subject: GrantSubject,
    pub vault_id: String,
    pub action: GrantAction,
    pub dataset: Dataset,
    pub not_before: u64,
    pub expires_at: u64,
    pub revoked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrgGraph {
    pub schema_version: u32,
    pub organizations: Vec<Organization>,
    pub vaults: Vec<VaultSite>,
    pub units: Vec<Unit>,
    pub operators: Vec<Operator>,
    pub appointments: Vec<Appointment>,
    pub devices: Vec<DeviceRecord>,
    pub grants: Vec<Grant>,
}

impl Default for OrgGraph {
    fn default() -> Self {
        Self {
            schema_version: ORG_SCHEMA,
            organizations: Vec::new(),
            vaults: Vec::new(),
            units: Vec::new(),
            operators: Vec::new(),
            appointments: Vec::new(),
            devices: Vec::new(),
            grants: Vec::new(),
        }
    }
}

impl OrgGraph {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Validates internal structure ONLY. Caller-provided JSON remains
    /// untrusted; success does not authenticate anyone or permit data transfer.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != ORG_SCHEMA {
            return Err("Unsupported organization schema");
        }
        if self.organizations.len() > MAX_ORGS
            || self.vaults.len() > MAX_VAULTS
            || self.units.len() > MAX_UNITS
            || self.operators.len() > MAX_OPERATORS
            || self.appointments.len() > MAX_APPOINTMENTS
            || self.devices.len() > MAX_DEVICES
            || self.grants.len() > MAX_GRANTS
        {
            return Err("Organization graph capacity exceeded");
        }
        if !unique(self.organizations.iter().map(|x| x.id.as_str()))
            || !unique(self.vaults.iter().map(|x| x.id.as_str()))
            || !unique(self.units.iter().map(|x| x.id.as_str()))
            || !unique(self.operators.iter().map(|x| x.id.as_str()))
            || !unique(self.appointments.iter().map(|x| x.id.as_str()))
            || !unique(self.devices.iter().map(|x| x.id.as_str()))
            || !unique(self.grants.iter().map(|x| x.id.as_str()))
        {
            return Err("Duplicate organization record identity");
        }

        for org in &self.organizations {
            if !id(&org.id) || !name(&org.name) {
                return Err("Invalid organization identity");
            }
        }
        for site in &self.vaults {
            if !id(&site.id) || !name(&site.name) || self.org(&site.organization_id).is_none() {
                return Err("Vault references unknown organization");
            }
        }
        for unit in &self.units {
            if !id(&unit.id) || !name(&unit.name) || self.site(&unit.vault_id).is_none() {
                return Err("Unit references unknown vault");
            }
            match (unit.kind, unit.parent_unit_id.as_deref()) {
                (UnitKind::Division, None) => (),
                (UnitKind::Team, Some(parent)) => {
                    let Some(parent_unit) = self.unit(parent) else {
                        return Err("Team references unknown parent");
                    };
                    if parent_unit.vault_id != unit.vault_id {
                        return Err("Reporting relationship crosses vault boundary");
                    }
                }
                _ => return Err("Division/team parent rule violated"),
            }
            let mut seen = HashSet::new();
            let mut current = Some(unit.id.as_str());
            while let Some(node_id) = current {
                if !seen.insert(node_id) {
                    return Err("Reporting relationship contains a cycle");
                }
                current = self.unit(node_id).and_then(|n| n.parent_unit_id.as_deref());
            }
        }
        for person in &self.operators {
            if !id(&person.id)
                || !name(&person.display_name)
                || self.org(&person.organization_id).is_none()
            {
                return Err("Operator references unknown organization");
            }
        }
        for appointment in &self.appointments {
            let Some(person) = self.person(&appointment.operator_id) else {
                return Err("Appointment operator unknown");
            };
            let Some(approver) = self.person(&appointment.approved_by_operator_id) else {
                return Err("Appointment approver unknown");
            };
            let Some(unit) = self.unit(&appointment.unit_id) else {
                return Err("Appointment unit unknown");
            };
            let Some(site) = self.site(&unit.vault_id) else {
                return Err("Appointment vault unknown");
            };
            if !id(&appointment.id)
                || !id(&appointment.position_id)
                || appointment.not_before >= appointment.expires_at
                || appointment.operator_id == appointment.approved_by_operator_id
            {
                return Err("Invalid appointment metadata");
            }
            if person.organization_id != site.organization_id
                || approver.organization_id != site.organization_id
            {
                return Err("Cross-organization appointment");
            }
        }
        for device in &self.devices {
            let Some(site) = self.site(&device.vault_id) else {
                return Err("Device vault unknown");
            };
            if !id(&device.id)
                || device.organization_id != site.organization_id
                || self.org(&device.organization_id).is_none()
            {
                return Err("Device crosses organization boundary");
            }
            if let Some(person_id) = &device.assigned_operator_id {
                if self
                    .person(person_id)
                    .is_none_or(|p| p.organization_id != device.organization_id)
                {
                    return Err("Device assigned to operator in another organization");
                }
            }
        }
        for grant in &self.grants {
            let Some(issuer) = self.person(&grant.issuer_operator_id) else {
                return Err("Grant issuer unknown");
            };
            let Some(site) = self.site(&grant.vault_id) else {
                return Err("Grant vault unknown");
            };
            if !id(&grant.id)
                || grant.not_before >= grant.expires_at
                || issuer.organization_id != grant.organization_id
                || site.organization_id != grant.organization_id
            {
                return Err("Invalid or cross-organization grant");
            }
            let same_org = match &grant.subject {
                GrantSubject::Operator(subject) => self
                    .person(subject)
                    .is_some_and(|p| p.organization_id == grant.organization_id),
                GrantSubject::Device(subject) => self
                    .device(subject)
                    .is_some_and(|d| d.organization_id == grant.organization_id),
            };
            if !same_org {
                return Err("Grant subject belongs to another organization");
            }
        }
        Ok(())
    }

    fn org(&self, id: &str) -> Option<&Organization> {
        self.organizations.iter().find(|x| x.id == id)
    }
    fn site(&self, id: &str) -> Option<&VaultSite> {
        self.vaults.iter().find(|x| x.id == id)
    }
    fn unit(&self, id: &str) -> Option<&Unit> {
        self.units.iter().find(|x| x.id == id)
    }
    fn person(&self, id: &str) -> Option<&Operator> {
        self.operators.iter().find(|x| x.id == id)
    }
    fn device(&self, id: &str) -> Option<&DeviceRecord> {
        self.devices.iter().find(|x| x.id == id)
    }

    /// Reporting lookup only; this is not a permissions inheritance API.
    pub fn reporting_chain(&self, unit_id: &str) -> Option<Vec<String>> {
        self.validate().ok()?;
        let mut current = Some(self.unit(unit_id)?);
        let mut path = Vec::new();
        while let Some(unit) = current {
            path.push(unit.id.clone());
            current = unit.parent_unit_id.as_deref().and_then(|id| self.unit(id));
        }
        Some(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> OrgGraph {
        OrgGraph {
            schema_version: ORG_SCHEMA,
            organizations: vec![
                Organization {
                    id: "vault-corp".into(),
                    name: "Vault Corp".into(),
                },
                Organization {
                    id: "care-network".into(),
                    name: "Care Network".into(),
                },
            ],
            vaults: vec![
                VaultSite {
                    id: "vault-one".into(),
                    organization_id: "vault-corp".into(),
                    name: "North".into(),
                },
                VaultSite {
                    id: "vault-two".into(),
                    organization_id: "vault-corp".into(),
                    name: "South".into(),
                },
                VaultSite {
                    id: "hospital-one".into(),
                    organization_id: "care-network".into(),
                    name: "Hospital".into(),
                },
            ],
            units: vec![
                Unit {
                    id: "security".into(),
                    vault_id: "vault-one".into(),
                    parent_unit_id: None,
                    kind: UnitKind::Division,
                    name: "Security".into(),
                },
                Unit {
                    id: "recon".into(),
                    vault_id: "vault-one".into(),
                    parent_unit_id: Some("security".into()),
                    kind: UnitKind::Team,
                    name: "Recon".into(),
                },
                Unit {
                    id: "maintenance".into(),
                    vault_id: "vault-two".into(),
                    parent_unit_id: None,
                    kind: UnitKind::Division,
                    name: "Maintenance".into(),
                },
                Unit {
                    id: "medical".into(),
                    vault_id: "hospital-one".into(),
                    parent_unit_id: None,
                    kind: UnitKind::Division,
                    name: "Medical".into(),
                },
            ],
            operators: vec![
                Operator {
                    id: "commander".into(),
                    organization_id: "vault-corp".into(),
                    display_name: "Commander".into(),
                    active: true,
                },
                Operator {
                    id: "scout".into(),
                    organization_id: "vault-corp".into(),
                    display_name: "Scout".into(),
                    active: true,
                },
                Operator {
                    id: "medic".into(),
                    organization_id: "care-network".into(),
                    display_name: "Medic".into(),
                    active: true,
                },
            ],
            appointments: vec![Appointment {
                id: "shift-one".into(),
                operator_id: "scout".into(),
                unit_id: "recon".into(),
                position_id: "recon".into(),
                approved_by_operator_id: "commander".into(),
                not_before: 10,
                expires_at: 100,
                revoked: false,
            }],
            devices: vec![
                DeviceRecord {
                    id: "field-one".into(),
                    organization_id: "vault-corp".into(),
                    vault_id: "vault-one".into(),
                    assigned_operator_id: Some("scout".into()),
                    lifecycle: DeviceLifecycle::RecordedActive,
                },
                DeviceRecord {
                    id: "care-one".into(),
                    organization_id: "care-network".into(),
                    vault_id: "hospital-one".into(),
                    assigned_operator_id: Some("medic".into()),
                    lifecycle: DeviceLifecycle::Pending,
                },
            ],
            grants: vec![Grant {
                id: "demo-grant".into(),
                organization_id: "vault-corp".into(),
                issuer_operator_id: "commander".into(),
                subject: GrantSubject::Operator("scout".into()),
                vault_id: "vault-one".into(),
                action: GrantAction::View,
                dataset: Dataset::Mission,
                not_before: 10,
                expires_at: 50,
                revoked: false,
            }],
        }
    }

    #[test]
    fn valid_multi_vault_graph_and_chain() {
        let graph = fixture();
        assert_eq!(graph.validate(), Ok(()));
        assert_eq!(
            graph.reporting_chain("recon"),
            Some(vec!["recon".into(), "security".into()])
        );
        assert_eq!(graph.reporting_chain("unknown"), None);
        assert!(OrgGraph::empty().validate().is_ok());
        assert!(OrgGraph::default().validate().is_ok());
    }

    #[test]
    fn cross_tenant_device_and_grant_fail_closed() {
        let mut graph = fixture();
        graph.devices[0].assigned_operator_id = Some("medic".into());
        assert_eq!(
            graph.validate(),
            Err("Device assigned to operator in another organization")
        );
        let mut graph = fixture();
        graph.grants[0].subject = GrantSubject::Device("care-one".into());
        assert_eq!(
            graph.validate(),
            Err("Grant subject belongs to another organization")
        );
        let mut graph = fixture();
        graph.appointments[0].operator_id = "medic".into();
        assert_eq!(graph.validate(), Err("Cross-organization appointment"));
    }

    #[test]
    fn same_org_different_vault_requires_local_reporting_tree() {
        let mut graph = fixture();
        graph.units[1].parent_unit_id = Some("maintenance".into());
        assert_eq!(
            graph.validate(),
            Err("Reporting relationship crosses vault boundary")
        );
        let mut graph = fixture();
        graph.devices[0].vault_id = "vault-two".into();
        assert!(graph.validate().is_ok()); // explicit reassignment within same org
    }

    #[test]
    fn missing_parent_cycles_and_duplicate_ids_rejected() {
        let mut graph = fixture();
        graph.units[1].parent_unit_id = Some("ghost".into());
        assert_eq!(graph.validate(), Err("Team references unknown parent"));
        let mut graph = fixture();
        graph.units[0].kind = UnitKind::Team;
        graph.units[0].parent_unit_id = Some("recon".into());
        assert_eq!(
            graph.validate(),
            Err("Reporting relationship contains a cycle")
        );
        let mut graph = fixture();
        graph.units[1].id = "security".into();
        assert_eq!(
            graph.validate(),
            Err("Duplicate organization record identity")
        );
    }

    #[test]
    fn appointment_time_and_revocation_metadata_not_privileges() {
        let mut graph = fixture();
        graph.appointments[0].expires_at = 10;
        assert_eq!(graph.validate(), Err("Invalid appointment metadata"));
        let mut graph = fixture();
        graph.appointments[0].revoked = true;
        graph.grants[0].revoked = true;
        graph.devices[0].lifecycle = DeviceLifecycle::Revoked;
        // Historical revoked records remain structurally valid, not authorized.
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn strict_serialization_and_bounds() {
        let mut graph = fixture();
        graph.schema_version = 99;
        assert_eq!(graph.validate(), Err("Unsupported organization schema"));
        let mut graph = fixture();
        graph.organizations[0].id = "../vault".into();
        assert_eq!(graph.validate(), Err("Invalid organization identity"));
        let mut graph = fixture();
        graph.operators[0].display_name = "Commander\nInjected".into();
        assert_eq!(
            graph.validate(),
            Err("Operator references unknown organization")
        );
        assert!(
            serde_json::from_str::<OrgGraph>(r#"{"schema_version":1,"unknown":true}"#).is_err()
        );
        let graph = fixture();
        let serialized = serde_json::to_vec(&graph).unwrap();
        assert_eq!(
            serde_json::from_slice::<OrgGraph>(&serialized).unwrap(),
            graph
        );
    }

    #[test]
    fn no_implicit_current_permissions_or_transport() {
        let graph = fixture();
        // The model intentionally exposes no authorization or sync invocation:
        // a signed credential, time source and policy engine are required later.
        assert_eq!(graph.grants[0].dataset, Dataset::Mission);
        assert_eq!(graph.devices[0].lifecycle, DeviceLifecycle::RecordedActive);
        assert!(graph.validate().is_ok());
    }
}
