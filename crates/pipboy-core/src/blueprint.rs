//! Research-backed module plan and literature registry.
//!
//! Both catalogs are compiled into the binary; reading them makes no
//! network requests and does not grant any OS or hardware permissions.

use crate::registry::{Readiness, MODULES};
use crate::capability::Capability;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Literature {
    pub id: String,
    pub title: String,
    pub contributors: String,
    pub publisher: String,
    pub year: u16,
    pub r#type: String,
    pub url: String,
    pub doi: Option<String>,
    pub modules: Vec<String>,
    pub design: String,
    pub limitation: String,
    pub acceptance: String,
    pub reviewed_on: String,
}

impl Literature {
    pub fn matches(&self, term: &str) -> bool {
        let term = term.to_lowercase();
        self.id.to_lowercase().contains(&term)
            || self.title.to_lowercase().contains(&term)
            || self.design.to_lowercase().contains(&term)
            || self.modules.iter().any(|item| item.to_lowercase().contains(&term))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModulePlan {
    pub id: String,
    pub name: String,
    pub gate: String,
    pub priority: String,
    pub domain: String,
    pub status: String,
    pub depends_on: Vec<String>,
    pub capabilities: Vec<String>,
    pub evidence: Vec<String>,
    pub features: Vec<String>,
    pub acceptance: Vec<String>,
}

impl ModulePlan {
    pub fn matches(&self, term: &str) -> bool {
        let term = term.to_lowercase();
        self.id.to_lowercase().contains(&term)
            || self.name.to_lowercase().contains(&term)
            || self.gate.to_lowercase().contains(&term)
            || self.domain.to_lowercase().contains(&term)
            || self.features.iter().any(|feature| feature.to_lowercase().contains(&term))
    }
}

/// Returns a bundled, *non-executable* engineering literature index.
pub fn literature() -> Result<Vec<Literature>, serde_json::Error> {
    serde_json::from_str(include_str!("../../../catalog/literature.json"))
}

/// Returns planned feature/module metadata; hardware modules remain disabled.
pub fn plans() -> Result<Vec<ModulePlan>, serde_json::Error> {
    serde_json::from_str(include_str!("../../../catalog/module_plan.json"))
}

fn gate_number(gate: &str) -> Option<u8> {
    match gate {
        "G0" => Some(0),
        "G1" => Some(1),
        "G2" => Some(2),
        "G3" => Some(3),
        "G4" => Some(4),
        _ => None,
    }
}

pub fn validate_blueprint(modules: &[ModulePlan], literature: &[Literature]) -> Result<(), String> {
    if modules.is_empty() || literature.is_empty() {
        return Err("planning and literature catalogs must not be empty".to_owned());
    }

    let mut ids = HashSet::new();
    let mut by_id = HashMap::new();
    for (index, module) in modules.iter().enumerate() {
        if module.id.trim().is_empty() || !ids.insert(module.id.as_str()) {
            return Err(format!("empty/duplicate module id: {}", module.id));
        }
        if gate_number(&module.gate).is_none() || !matches!(module.priority.as_str(), "P0" | "P1" | "P2") {
            return Err(format!("invalid gate or priority for module {}", module.id));
        }
        if !matches!(module.status.as_str(), "planned" | "implemented_unverified") {
            return Err(format!("invalid implementation status for {}", module.id));
        }
        if module.status == "implemented_unverified" && module.gate != "G0" {
            return Err(format!("only G0 implemented modules are currently supported: {}", module.id));
        }
        if module.name.trim().is_empty() || module.features.is_empty() || module.acceptance.len() < 2 {
            return Err(format!("module {} lacks features or acceptance criteria", module.id));
        }
        if module.evidence.is_empty() {
            return Err(format!("module {} lacks evidence references", module.id));
        }
        let mut capabilities = HashSet::new();
        for key in &module.capabilities {
            if Capability::from_key(key).is_none() || !capabilities.insert(key) {
                return Err(format!("module {} has invalid/repeated capability: {key}", module.id));
            }
        }
        by_id.insert(module.id.as_str(), index);
    }

    // Every runtime module must remain in the plan with consistent implementation readiness.
    for runtime in MODULES {
        let item = modules
            .iter()
            .find(|module| module.id == runtime.id)
            .ok_or_else(|| format!("runtime module {} missing in plan", runtime.id))?;
        let implemented = item.status == "implemented_unverified";
        if implemented != (runtime.readiness == Readiness::Ready) {
            return Err(format!("readiness drift for runtime module {}", runtime.id));
        }
        let planned_capabilities = item
            .capabilities
            .iter()
            .map(|key| Capability::from_key(key).expect("capabilities validated above"))
            .collect::<Vec<_>>();
        if planned_capabilities.as_slice() != runtime.required_capabilities {
            return Err(format!("capability drift for runtime module {}", runtime.id));
        }
    }

    let mut evidence_ids = HashSet::new();
    for item in literature {
        if item.id.trim().is_empty() || !evidence_ids.insert(item.id.as_str()) {
            return Err(format!("empty/duplicate evidence id: {}", item.id));
        }
        if !(2000..=2026).contains(&item.year)
            || !item.url.starts_with("https://")
            || item.title.trim().is_empty()
            || item.contributors.trim().is_empty()
            || item.publisher.trim().is_empty()
            || item.design.trim().is_empty()
            || item.limitation.trim().is_empty()
            || item.acceptance.trim().is_empty()
            || item.reviewed_on.trim().is_empty()
            || item.modules.is_empty()
        {
            return Err(format!("incomplete evidence metadata: {}", item.id));
        }
        if !matches!(
            item.r#type.as_str(),
            "peer_reviewed" | "official_docs" | "language_spec" | "platform_spec" | "standard" | "license_policy"
        ) {
            return Err(format!("unrecognized evidence type {}", item.id));
        }
        for mapped in &item.modules {
            if !ids.contains(mapped.as_str()) {
                return Err(format!("{} references missing module {mapped}", item.id));
            }
        }
    }

    for module in modules {
        let current = gate_number(&module.gate).expect("validated above");
        let mut dependencies = HashSet::new();
        for prerequisite in &module.depends_on {
            if !dependencies.insert(prerequisite) {
                return Err(format!("duplicate prerequisite for {}", module.id));
            }
            let earlier = by_id
                .get(prerequisite.as_str())
                .map(|index| &modules[*index])
                .ok_or_else(|| format!("{} references unknown prerequisite {}", module.id, prerequisite))?;
            if gate_number(&earlier.gate).expect("validated above") > current {
                return Err(format!("{} depends on later gate {}", module.id, prerequisite));
            }
        }
        for evidence in &module.evidence {
            if !evidence_ids.contains(evidence.as_str()) {
                return Err(format!("module {} references unknown evidence {}", module.id, evidence));
            }
        }
    }

    // Iterative topological elimination handles long chains without recursion or panics.
    let mut completed: HashSet<&str> = HashSet::new();
    loop {
        let before = completed.len();
        for module in modules {
            if module.depends_on.iter().all(|dependency| completed.contains(dependency.as_str())) {
                completed.insert(&module.id);
            }
        }
        if completed.len() == modules.len() {
            break;
        }
        if before == completed.len() {
            return Err("module dependencies contain a cycle".to_owned());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_catalogs_are_consistent() {
        let modules = plans().unwrap();
        let evidence = literature().unwrap();
        assert!(modules.len() >= 25);
        assert!(evidence.len() >= 20);
        validate_blueprint(&modules, &evidence).unwrap();
    }

    #[test]
    fn names_and_topic_search_work() {
        let plans = plans().unwrap();
        let evidence = literature().unwrap();
        assert!(plans.iter().any(|p| p.matches("GNSS")));
        assert!(evidence.iter().any(|p| p.matches("Rust")));
    }

    #[test]
    fn missing_evidence_fails_closed() {
        let mut modules = plans().unwrap();
        modules[0].evidence = vec!["L_DOES_NOT_EXIST".into()];
        assert!(validate_blueprint(&modules, &literature().unwrap()).is_err());
    }

    #[test]
    fn duplicate_and_cyclic_modules_fail_closed() {
        let mut modules = plans().unwrap();
        let evidence = literature().unwrap();
        modules.push(modules[0].clone());
        assert!(validate_blueprint(&modules, &evidence).is_err());
        modules.pop();

        let a = modules.iter().position(|m| m.id == "stat").unwrap();
        let b = modules.iter().position(|m| m.id == "library").unwrap();
        modules[a].depends_on = vec!["library".into()];
        modules[b].depends_on = vec!["stat".into()];
        assert!(validate_blueprint(&modules, &evidence).is_err());
    }

    #[test]
    fn later_gates_and_fake_capabilities_rejected() {
        let mut modules = plans().unwrap();
        let evidence = literature().unwrap();
        let stat = modules.iter().position(|m| m.id == "stat").unwrap();
        modules[stat].depends_on = vec!["radio".to_string()];
        assert!(validate_blueprint(&modules, &evidence).is_err());
        modules[stat].depends_on.clear();
        modules[stat].capabilities = vec!["network.scan_everywhere".to_string()];
        assert!(validate_blueprint(&modules, &evidence).is_err());
    }
}
