//! A build-time snapshot of third-party references, not vendored code.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CatalogSource {
    pub id: String,
    pub repository: String,
    pub project: String,
    pub language: String,
    pub category: String,
    pub license: String,
    pub license_status: String,
    pub disposition: String,
    pub summary: String,
}

impl CatalogSource {
    pub fn url(&self) -> String {
        format!("https://github.com/{}", self.repository)
    }

    pub fn matches(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        [
            &self.id,
            &self.repository,
            &self.project,
            &self.language,
            &self.category,
            &self.license,
            &self.summary,
        ]
        .iter()
        .any(|value| value.to_lowercase().contains(&query))
    }
}

/// Returns the checked-in GitHub discovery snapshot without contacting GitHub.
pub fn sources() -> Result<Vec<CatalogSource>, serde_json::Error> {
    serde_json::from_str(include_str!("../../../catalog/sources.json"))
}

pub fn validate_sources(entries: &[CatalogSource]) -> Result<(), String> {
    if entries.is_empty() {
        return Err("catalog must contain at least one source".into());
    }

    let mut ids = HashSet::new();
    let mut repos = HashSet::new();

    for entry in entries {
        if !ids.insert(entry.id.as_str()) {
            return Err(format!("duplicate source id: {}", entry.id));
        }
        if !repos.insert(entry.repository.as_str()) {
            return Err(format!("duplicate repository: {}", entry.repository));
        }

        let mut parts = entry.repository.split('/');
        if !matches!(
            (parts.next(), parts.next(), parts.next()),
            (Some(owner), Some(name), None) if !owner.is_empty() && !name.is_empty()
        ) {
            return Err(format!("invalid repository: {}", entry.repository));
        }

        if entry.project.trim().is_empty() || entry.summary.trim().is_empty() {
            return Err(format!("source {} lacks basic metadata", entry.id));
        }

        if !matches!(
            entry.license_status.as_str(),
            "declared" | "unverified" | "review-required" | "copyleft-review"
        ) {
            return Err(format!("source {} has invalid license status", entry.id));
        }

        if entry.disposition != "reference-only" {
            return Err(format!(
                "source {} needs an independent import review",
                entry.id
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_valid_and_reference_only() {
        let entries = sources().expect("source JSON should parse");
        assert!(entries.len() >= 10);
        validate_sources(&entries).expect("source data should be unique and reviewed");
    }

    #[test]
    fn search_matches_project_and_capabilities() {
        let entries = sources().unwrap();
        assert!(entries.iter().any(|entry| entry.matches("ratatui")));
        assert!(entries.iter().any(|entry| entry.matches("Raspberry")));
        assert!(!entries
            .iter()
            .any(|entry| entry.matches("unlikely-query-8173")));
    }

    #[test]
    fn rejects_duplicate_entries() {
        let mut entries = sources().unwrap();
        entries.push(entries[0].clone());
        assert!(validate_sources(&entries).is_err());
    }
}
