//! Lifecycle and capability orchestration for trusted built-in modules.
//! This application policy is not an operating-system sandbox.

pub use crate::capability::{Capability, PermissionSet};
use crate::lifecycle::{Event, ModuleLifecycle, State};
use crate::metrics::Snapshot;
use crate::observation::Observation;
use crate::registry::{Readiness, MODULES};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize)]
pub struct ModuleSample {
    pub source: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_observation: Option<Observation<Snapshot>>,
}

/// Only trusted built-ins may implement this interface. Construction and
/// registration must be free of I/O; open resources in start after prechecks.
pub trait RuntimeModule: Send {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> &'static [Capability];
    fn start(&mut self) -> Result<(), String> {
        Ok(())
    }
    /// Release resources and cancel pending work. Must be idempotent.
    fn stop(&mut self) {}
    fn sample(&mut self) -> Result<ModuleSample, String>;
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PollOutcome {
    Sample { data: Box<ModuleSample> },
    Denied { missing: Vec<Capability> },
    Inactive { state: State },
    Failed { reason: String },
}

#[derive(Clone, Debug, Serialize)]
pub struct ModulePoll {
    pub module_id: &'static str,
    pub state: State,
    pub outcome: PollOutcome,
}

struct HostedModule {
    module: Box<dyn RuntimeModule>,
    lifecycle: ModuleLifecycle,
}

impl HostedModule {
    fn active(&self) -> bool {
        matches!(self.lifecycle.state(), State::Running | State::Degraded)
    }

    fn disable(&mut self, event: Event) {
        if self.active() {
            self.module.stop();
        }
        // All registered modules are implemented and support Stop/Revoke.
        self.lifecycle
            .apply(event)
            .expect("registered lifecycle can disable");
    }
}

/// A synchronous, trusted built-in operation. This is not an OS sandbox.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalOperation {
    LibraryRead,
    NotesRead,
    NotesWrite,
}

impl LocalOperation {
    fn module_id(self) -> &'static str {
        match self {
            Self::LibraryRead => "library",
            Self::NotesRead | Self::NotesWrite => "notes",
        }
    }

    fn required(self) -> &'static [Capability] {
        match self {
            Self::LibraryRead | Self::NotesRead => &[Capability::FilesRead],
            Self::NotesWrite => &[Capability::FilesRead, Capability::FilesWrite],
        }
    }
}

pub struct ModuleHost {
    grants: PermissionSet,
    modules: Vec<HostedModule>,
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

    /// The caller supplies a trusted, explicit consent decision. Grants last
    /// only for this host instance and never start or restart a module.
    pub fn grant(&mut self, capability: Capability) {
        self.grants.grant(capability);
    }

    /// Stop affected running modules immediately, before returning to caller.
    pub fn revoke(&mut self, capability: Capability) {
        self.grants.revoke(capability);
        for entry in &mut self.modules {
            if entry.module.capabilities().contains(&capability) {
                entry.disable(Event::Revoke);
            }
        }
    }

    /// Checks permission before invoking a synchronous local operation.
    /// Explicit caller intent and authorization must supply write grants.
    pub fn run_local<T, E>(
        &self,
        operation: LocalOperation,
        action: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, String>
    where
        E: std::fmt::Display,
    {
        let descriptor = MODULES
            .iter()
            .find(|entry| entry.id == operation.module_id())
            .ok_or_else(|| format!("unknown local module {}", operation.module_id()))?;
        if descriptor.readiness != Readiness::Ready {
            return Err(format!("module {} is not implemented", descriptor.id));
        }
        let missing: Vec<_> = operation
            .required()
            .iter()
            .filter(|capability| !self.grants.allows(**capability))
            .map(|capability| capability.key())
            .collect();
        if !missing.is_empty() {
            return Err(format!("{} denied: {}", descriptor.id, missing.join(", ")));
        }
        action().map_err(|error| format!("{} operation failed: {error}", descriptor.id))
    }

    pub fn state(&self, id: &str) -> Option<State> {
        self.modules
            .iter()
            .find(|entry| entry.module.id() == id)
            .map(|entry| entry.lifecycle.state())
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
            return Err(format!(
                "module {id} capability declaration differs from registry"
            ));
        }
        if self.modules.iter().any(|entry| entry.module.id() == id) {
            return Err(format!("duplicate module {id}"));
        }
        let mut seen = HashSet::new();
        if !module
            .capabilities()
            .iter()
            .all(|capability| seen.insert(*capability))
        {
            return Err(format!("module {id} declares duplicate capabilities"));
        }
        self.modules.push(HostedModule {
            lifecycle: ModuleLifecycle::new(id, true)?,
            module,
        });
        Ok(())
    }

    pub fn start(&mut self, id: &str) -> Result<(), String> {
        let entry = self
            .modules
            .iter_mut()
            .find(|entry| entry.module.id() == id)
            .ok_or_else(|| format!("unknown registered module {id}"))?;
        if entry.lifecycle.state() != State::Disabled {
            return Err(format!("module {id} must be disabled before starting"));
        }
        let missing: Vec<_> = entry
            .module
            .capabilities()
            .iter()
            .filter(|capability| !self.grants.allows(**capability))
            .map(|capability| capability.key())
            .collect();
        if !missing.is_empty() {
            return Err(format!("module {id} denied: {}", missing.join(", ")));
        }
        entry.lifecycle.apply(Event::GrantConfirmed)?;
        if let Err(error) = entry.module.start() {
            entry.module.stop(); // Clean up even partially opened resources.
            entry.lifecycle.apply(Event::DeviceUnavailable)?;
            return Err(error);
        }
        entry.lifecycle.apply(Event::StartSucceeded)?;
        Ok(())
    }

    pub fn stop(&mut self, id: &str) -> Result<(), String> {
        let entry = self
            .modules
            .iter_mut()
            .find(|entry| entry.module.id() == id)
            .ok_or_else(|| format!("unknown registered module {id}"))?;
        entry.disable(Event::Stop);
        Ok(())
    }

    pub fn poll_once(&mut self) -> Vec<ModulePoll> {
        self.modules
            .iter_mut()
            .map(|entry| {
                let missing: Vec<Capability> = entry
                    .module
                    .capabilities()
                    .iter()
                    .copied()
                    .filter(|capability| !self.grants.allows(*capability))
                    .collect();
                let outcome = if !missing.is_empty() {
                    entry.disable(Event::Revoke);
                    PollOutcome::Denied { missing }
                } else if !entry.active() {
                    PollOutcome::Inactive {
                        state: entry.lifecycle.state(),
                    }
                } else {
                    match entry.module.sample() {
                        Ok(data) => {
                            if entry.lifecycle.state() == State::Degraded {
                                entry
                                    .lifecycle
                                    .apply(Event::DataRecovered)
                                    .expect("degraded recovery");
                            }
                            PollOutcome::Sample {
                                data: Box::new(data),
                            }
                        }
                        Err(reason) => {
                            if entry.lifecycle.state() == State::Running {
                                entry
                                    .lifecycle
                                    .apply(Event::DataDegraded)
                                    .expect("running degradation");
                            }
                            PollOutcome::Failed { reason }
                        }
                    }
                };
                ModulePoll {
                    module_id: entry.module.id(),
                    state: entry.lifecycle.state(),
                    outcome,
                }
            })
            .collect()
    }
}

impl Drop for ModuleHost {
    fn drop(&mut self) {
        for entry in &mut self.modules {
            entry.disable(Event::Stop);
        }
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
                host_observation: None,
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
                host_observation: None,
            })
        }
    }

    fn mock(
        id: &'static str,
        required: &'static [Capability],
        calls: &Arc<AtomicUsize>,
    ) -> Box<dyn RuntimeModule> {
        Box::new(MockModule {
            id,
            required,
            calls: Arc::clone(calls),
        })
    }

    #[test]
    fn local_denial_does_not_execute_callback() {
        let host = ModuleHost::new(PermissionSet::empty());
        let mut called = false;
        let error = host
            .run_local(LocalOperation::LibraryRead, || {
                called = true;
                Ok::<(), std::io::Error>(())
            })
            .unwrap_err();
        assert!(error.contains("files.read"));
        assert!(!called);
    }

    #[test]
    fn local_read_only_cannot_write_but_explicit_grant_can() {
        let mut host = ModuleHost::new(PermissionSet::offline_read_only());
        assert_eq!(
            host.run_local(LocalOperation::NotesRead, || Ok::<_, String>(7))
                .unwrap(),
            7
        );
        let denied = host.run_local(LocalOperation::NotesWrite, || Ok::<_, String>(8));
        assert!(denied.unwrap_err().contains("files.write"));
        host.grant(Capability::FilesWrite);
        assert_eq!(
            host.run_local(LocalOperation::NotesWrite, || Ok::<_, String>(9))
                .unwrap(),
            9
        );
        host.revoke(Capability::FilesWrite);
        assert!(host
            .run_local(LocalOperation::NotesWrite, || Ok::<_, String>(10))
            .is_err());
    }

    #[test]
    fn local_operation_errors_propagate() {
        let host = ModuleHost::new(PermissionSet::offline_read_only());
        let result = host.run_local(LocalOperation::NotesRead, || {
            Err::<(), _>("fixture failure")
        });
        assert!(result.unwrap_err().contains("fixture failure"));
    }

    #[test]
    fn denies_without_invoking_and_allows_after_grant() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut host = ModuleHost::new(PermissionSet::offline_read_only());
        host.register(mock("notes", WRITE, &calls)).unwrap();

        assert!(matches!(
            &host.poll_once()[0].outcome,
            PollOutcome::Denied { .. }
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        host.grant(Capability::FilesWrite);
        host.start("notes").unwrap();
        assert!(matches!(
            &host.poll_once()[0].outcome,
            PollOutcome::Sample { .. }
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        host.revoke(Capability::FilesWrite);
        assert!(matches!(
            host.poll_once()[0].outcome,
            PollOutcome::Denied { .. }
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn two_concrete_modules_share_one_host() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut host = ModuleHost::new(PermissionSet::offline_read_only());
        host.register(mock("stat", READ, &calls)).unwrap();
        host.register(Box::new(LibraryFixture {
            calls: Arc::clone(&calls),
        }))
        .unwrap();

        host.start("stat").unwrap();
        host.start("library").unwrap();
        let output = host.poll_once();
        assert_eq!(output.len(), 2);
        assert!(output
            .iter()
            .all(|entry| matches!(&entry.outcome, PollOutcome::Sample { .. })));
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
        assert_eq!(
            Capability::from_key("mesh.transmit"),
            Some(Capability::MeshTransmit)
        );
        assert_eq!(Capability::from_key("network.scan"), None);
        assert!(!PermissionSet::offline_read_only().allows(Capability::NetworkFetch));
    }
    struct Probe {
        starts: Arc<AtomicUsize>,
        stops: Arc<AtomicUsize>,
        samples: Arc<AtomicUsize>,
        fail_start: bool,
        failures: usize,
    }
    impl RuntimeModule for Probe {
        fn id(&self) -> &'static str {
            "stat"
        }
        fn capabilities(&self) -> &'static [Capability] {
            READ
        }
        fn start(&mut self) -> Result<(), String> {
            self.starts.fetch_add(1, Ordering::SeqCst);
            if self.fail_start {
                Err("fixture start failure".into())
            } else {
                Ok(())
            }
        }
        fn stop(&mut self) {
            self.stops.fetch_add(1, Ordering::SeqCst);
        }
        fn sample(&mut self) -> Result<ModuleSample, String> {
            let index = self.samples.fetch_add(1, Ordering::SeqCst);
            if index < self.failures {
                return Err("fixture sample failure".into());
            }
            Ok(ModuleSample {
                source: "fixture".into(),
                detail: "ok".into(),
                host_observation: None,
            })
        }
    }
    fn probe(fail_start: bool, failures: usize) -> (Probe, [Arc<AtomicUsize>; 3]) {
        let counts = std::array::from_fn(|_| Arc::new(AtomicUsize::new(0)));
        (
            Probe {
                starts: counts[0].clone(),
                stops: counts[1].clone(),
                samples: counts[2].clone(),
                fail_start,
                failures,
            },
            counts,
        )
    }
    fn count(value: &Arc<AtomicUsize>) -> usize {
        value.load(Ordering::SeqCst)
    }

    #[test]
    fn no_start_or_sample_before_grant_and_explicit_start() {
        let (module, [starts, stops, samples]) = probe(false, 0);
        let mut host = ModuleHost::new(PermissionSet::empty());
        host.register(Box::new(module)).unwrap();
        assert!(host.start("stat").is_err());
        assert!(matches!(
            host.poll_once()[0].outcome,
            PollOutcome::Denied { .. }
        ));
        host.grant(Capability::HostRead);
        assert!(matches!(
            host.poll_once()[0].outcome,
            PollOutcome::Inactive {
                state: State::Disabled
            }
        ));
        assert_eq!((count(&starts), count(&stops), count(&samples)), (0, 0, 0));
        host.start("stat").unwrap();
        assert!(host.start("stat").is_err());
        assert_eq!(host.state("stat"), Some(State::Running));
    }

    #[test]
    fn revocation_is_immediate_and_regrant_needs_explicit_restart() {
        let (module, [starts, stops, samples]) = probe(false, 0);
        let mut host = ModuleHost::new(PermissionSet::offline_read_only());
        host.register(Box::new(module)).unwrap();
        host.start("stat").unwrap();
        host.poll_once();
        host.revoke(Capability::HostRead);
        assert_eq!(count(&stops), 1); // No extra poll required to release.
        assert_eq!(host.state("stat"), Some(State::Disabled));
        host.poll_once();
        host.grant(Capability::HostRead);
        host.poll_once();
        assert_eq!(count(&samples), 1);
        host.start("stat").unwrap();
        host.poll_once();
        host.revoke(Capability::NetworkFetch); // Unrelated grant does not stop STAT.
        assert_eq!(host.state("stat"), Some(State::Running));
        assert_eq!(count(&starts), 2);
        drop(host);
        assert_eq!(count(&stops), 2);
    }

    #[test]
    fn failed_start_cleans_partial_resources_and_requires_reset() {
        let (module, [starts, stops, samples]) = probe(true, 0);
        let mut host = ModuleHost::new(PermissionSet::offline_read_only());
        host.register(Box::new(module)).unwrap();
        assert!(host.start("stat").is_err());
        assert_eq!(host.state("stat"), Some(State::Unavailable));
        host.poll_once();
        assert!(host.start("stat").is_err());
        assert_eq!((count(&starts), count(&stops), count(&samples)), (1, 1, 0));
        host.stop("stat").unwrap();
        assert_eq!(host.state("stat"), Some(State::Disabled));
        assert!(host.start("stat").is_err());
        drop(host);
        assert_eq!(count(&stops), 2);
    }

    #[test]
    fn repeated_sample_failures_degrade_then_recover_without_restart() {
        let (module, [starts, stops, samples]) = probe(false, 2);
        let mut host = ModuleHost::new(PermissionSet::offline_read_only());
        host.register(Box::new(module)).unwrap();
        host.start("stat").unwrap();
        for _ in 0..2 {
            let output = host.poll_once();
            assert_eq!(output[0].state, State::Degraded);
            assert!(matches!(output[0].outcome, PollOutcome::Failed { .. }));
        }
        assert_eq!(host.poll_once()[0].state, State::Running);
        host.stop("stat").unwrap();
        host.stop("stat").unwrap();
        drop(host);
        assert_eq!((count(&starts), count(&stops), count(&samples)), (1, 1, 3));
    }
}
