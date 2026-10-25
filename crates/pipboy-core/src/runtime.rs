//! Testable in-process module polymorphism and capability prechecks.
//!
//! Important: PermissionSet is an application policy check, **not** an
//! operating system sandbox. No untrusted native code may be loaded here.

use crate::registry::{Readiness, MODULES};
pub use crate::capability::{Capability, PermissionSet};
use serde::Serialize;
use std::collections::HashSet;

/// A domain sample; future versioned Observation<T> models add type/units/quality.
#[derive(Clone, Debug, Serialize)]
pub struct ModuleSample {
    pub source: String,
    pub detail: String,
}

/// dyn-compatible interface for heterogeneous, **trusted built-in** modules.
/// This interface does not give untrusted code OS-level isolation.
pub trait RuntimeModule: Send {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> &'static [Capability];
    fn sample(&mut self) -> Result<ModuleSample, String>;
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PollOutcome {
    Sample { data: ModuleSample },
    Denied { missing: Vec<Capability> },
    Failed { reason: String },
}

#[derive(Clone, Debug, Serialize)]
pub struct ModulePoll {
    pub module_id: &'static str,
    pub outcome: PollOutcome,
}

/// A small OO host that makes prechecks before calling any module behavior.
///
/// Register only trusted built-in modules. The host is **not** used as a
/// sandbox and currently does not own hardware handles.
pub struct ModuleHost {
    grants: PermissionSet,
    modules: Vec<Box<dyn RuntimeModule>>,
}

impl ModuleHost {
    pub fn new(grants: PermissionSet) -> Self {
        Self {
            grants,
            modules: Vec::new(),
        }
    }

    pub fn permissions(&self) -> &PermissionSet {
        &self.grants
    }

    /// Caller is responsible for authenticating user consent to changes.
    pub fn permissions_mut(&mut self) -> &mut PermissionSet {
        &mut self.grants
    }

    pub fn register(&mut self, module: Box<dyn RuntimeModule>) -> Result<(), String> {
        let id = module.id();
        let declared = MODULES
            .iter()
            .find(|item| item.id == id)
            .ok_or_else(|| format!("unknown module {id}"))?;

        if declared.readiness != Readiness::Ready {
            return Err(format!("module {id} is not implemented"));
        }
        if module.capabilities() != declared.required_capabilities {
            return Err(format!("module {id} capability declaration differs from registry"));
        }
        if self.modules.iter().any(|item| item.id() == id) {
            return Err(format!("duplicate module {id}"));
        }
        let mut seen = HashSet::new();
        if !module.capabilities().iter().all(|capability| seen.insert(*capability)) {
            return Err(format!("module {id} declares duplicate capabilities"));
        }
        self.modules.push(module);
        Ok(())
    }

    pub fn poll_once(&mut self) -> Vec<ModulePoll> {
        self.modules
            .iter_mut()
            .map(|module| {
                let missing: Vec<Capability> = module
                    .capabilities()
                    .iter()
                    .copied()
                    .filter(|capability| !self.grants.allows(*capability))
                    .collect();

                let outcome = if !missing.is_empty() {
                    PollOutcome::Denied { missing }
                } else {
                    match module.sample() {
                        Ok(data) => PollOutcome::Sample { data },
                        Err(reason) => PollOutcome::Failed { reason },
                    }
                };
                ModulePoll {
                    module_id: module.id(),
                    outcome,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    const READ: &[Capability] = &[Capability::HostRead];
    const WRITE: &[Capability] = &[Capability::FilesWrite];
    const FILE_READ: &[Capability] = &[Capability::FilesRead];
    const RADIO: &[Capability] = &[Capability::SdrReceive];

    struct MockModule {
        id: &'static str,
        required: &'static [Capability],
        calls: Arc<AtomicUsize>,
    }

    impl RuntimeModule for MockModule {
        fn id(&self) -> &'static str {
            self.id
        }

        fn capabilities(&self) -> &'static [Capability] {
            self.required
        }

        fn sample(&mut self) -> Result<ModuleSample, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ModuleSample {
                source: self.id.to_string(),
                detail: "fixture".to_string(),
            })
        }
    }

    /// A second concrete type proves actual heterogeneous trait dispatch.
    struct LibraryFixture {
        calls: Arc<AtomicUsize>,
    }

    impl RuntimeModule for LibraryFixture {
        fn id(&self) -> &'static str {
            "library"
        }

        fn capabilities(&self) -> &'static [Capability] {
            FILE_READ
        }

        fn sample(&mut self) -> Result<ModuleSample, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ModuleSample {
                source: "library-fixture".to_owned(),
                detail: "offline-only".to_owned(),
            })
        }
    }

    fn mock(id: &'static str, required: &'static [Capability], calls: &Arc<AtomicUsize>) -> Box<dyn RuntimeModule> {
        Box::new(MockModule {
            id,
            required,
            calls: Arc::clone(calls),
        })
    }

    #[test]
    fn denies_without_invoking_and_allows_after_grant() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut host = ModuleHost::new(PermissionSet::offline_read_only());
        host.register(mock("notes", WRITE, &calls)).unwrap();

        assert!(matches!(&host.poll_once()[0].outcome, PollOutcome::Denied { .. }));
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        host.permissions_mut().grant(Capability::FilesWrite);
        assert!(matches!(host.poll_once()[0].outcome, PollOutcome::Sample { .. }));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        host.permissions_mut().revoke(Capability::FilesWrite);
        assert!(matches!(host.poll_once()[0].outcome, PollOutcome::Denied { .. }));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn two_concrete_modules_share_one_host() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut host = ModuleHost::new(PermissionSet::offline_read_only());
        host.register(mock("stat", READ, &calls)).unwrap();
        host.register(Box::new(LibraryFixture { calls: Arc::clone(&calls) })).unwrap();

        let output = host.poll_once();
        assert_eq!(output.len(), 2);
        assert!(output.iter().all(|entry| matches!(&entry.outcome, PollOutcome::Sample { .. })));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn unimplemented_and_duplicate_modules_are_rejected() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut host = ModuleHost::new(PermissionSet::empty());
        assert!(host.register(mock("radio", RADIO, &calls)).is_err());
        assert!(host.register(mock("unknown", &[], &calls)).is_err());
        host.register(mock("stat", READ, &calls)).unwrap();
        assert!(host.register(mock("stat", READ, &calls)).is_err());
        assert!(host.register(mock("library", &[], &calls)).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn permission_vocabulary_does_not_accept_undeclared_operations() {
        assert_eq!(Capability::from_key("mesh.transmit"), Some(Capability::MeshTransmit));
        assert_eq!(Capability::from_key("network.scan"), None);
        assert!(!PermissionSet::offline_read_only().allows(Capability::NetworkFetch));
    }
}
