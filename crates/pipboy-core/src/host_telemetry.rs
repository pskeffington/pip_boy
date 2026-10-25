//! Lazy host collector adapter: no collection until ModuleHost permits start.

use crate::capability::Capability;
use crate::runtime::{ModuleSample, RuntimeModule};
use crate::telemetry::{SnapshotSource, TelemetrySession};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct HostTelemetryModule<S, F> {
    factory: F,
    capacity: usize,
    expiry: Duration,
    session: Option<TelemetrySession<S>>,
    session_start: Instant,
}

impl<S: SnapshotSource, F: FnMut() -> Result<S, String>> HostTelemetryModule<S, F> {
    pub fn new(factory: F, capacity: usize, expiry: Duration) -> Result<Self, String> {
        TelemetrySession::<S>::validate_configuration(capacity, expiry)?;
        Ok(Self {
            factory,
            capacity,
            expiry,
            session: None,
            session_start: Instant::now(),
        })
    }
    /// Frontends use this same clock when displaying observation age.
    pub fn session_start(&self) -> Instant {
        self.session_start
    }
}

impl<S: SnapshotSource + Send, F: FnMut() -> Result<S, String> + Send> RuntimeModule
    for HostTelemetryModule<S, F>
{
    fn id(&self) -> &'static str {
        "stat"
    }
    fn capabilities(&self) -> &'static [Capability] {
        &[Capability::HostRead]
    }

    fn start(&mut self) -> Result<(), String> {
        if self.session.is_some() {
            return Err("telemetry is already started".into());
        }
        self.session = Some(TelemetrySession::new(
            (self.factory)()?,
            self.capacity,
            self.expiry,
        )?);
        Ok(())
    }

    fn stop(&mut self) {
        // Dropping the session releases its collector and all buffered values.
        self.session = None;
    }

    fn sample(&mut self) -> Result<ModuleSample, String> {
        let session = self.session.as_mut().ok_or("telemetry is not started")?;
        let started = self.session_start;
        let now = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let wall = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok());
        session.poll(now, wall)?;
        Ok(ModuleSample {
            source: "host.stats".into(),
            detail: format!(
                "{} retained / {} dropped",
                session.retained(),
                session.dropped()
            ),
            host_observation: session.latest(now),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::State;
    use crate::metrics::Snapshot;
    use crate::runtime::{ModuleHost, PermissionSet, PollOutcome};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    struct Source {
        drops: Arc<AtomicUsize>,
    }
    impl Drop for Source {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }
    impl SnapshotSource for Source {
        fn snapshot(&mut self) -> Snapshot {
            Snapshot {
                observed_unix_s: 1234,
                os: "fixture".into(),
                arch: "test".into(),
                cpu_percent: 0.0,
                logical_cores: 1,
                memory_used_mib: 1,
                memory_total_mib: 2,
                uptime_s: 5,
            }
        }
    }

    #[test]
    fn lazy_factory_and_collector_release_obey_host_lifecycle() {
        let creations = Arc::new(AtomicUsize::new(0));
        let drops = Arc::new(AtomicUsize::new(0));
        let module = HostTelemetryModule::new(
            {
                let creations = creations.clone();
                let drops = drops.clone();
                move || {
                    creations.fetch_add(1, Ordering::SeqCst);
                    Ok(Source {
                        drops: drops.clone(),
                    })
                }
            },
            2,
            Duration::from_secs(1),
        )
        .unwrap();
        let mut host = ModuleHost::new(PermissionSet::empty());
        host.register(Box::new(module)).unwrap();
        assert!(host.start("stat").is_err());
        host.poll_once();
        assert_eq!(creations.load(Ordering::SeqCst), 0);
        host.grant(Capability::HostRead);
        host.start("stat").unwrap();
        for _ in 0..3 {
            host.poll_once();
        }
        let PollOutcome::Sample { data } = host.poll_once().pop().unwrap().outcome else {
            panic!("no sample");
        };
        assert_eq!(data.detail, "2 retained / 2 dropped");
        assert_eq!(data.host_observation.unwrap().source_id, "host.stats");
        host.revoke(Capability::HostRead);
        assert_eq!(host.state("stat"), Some(State::Disabled));
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        host.grant(Capability::HostRead);
        host.poll_once();
        assert_eq!(creations.load(Ordering::SeqCst), 1);
        host.start("stat").unwrap();
        let PollOutcome::Sample { data } = host.poll_once().pop().unwrap().outcome else {
            panic!("no sample");
        };
        assert_eq!(data.detail, "1 retained / 0 dropped");
        drop(host);
        assert_eq!(creations.load(Ordering::SeqCst), 2);
        assert_eq!(drops.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn invalid_configuration_never_invokes_factory() {
        for (capacity, expiry) in [(0, Duration::from_secs(1)), (1, Duration::ZERO)] {
            let result = HostTelemetryModule::<Source, _>::new(
                || panic!("must remain lazy"),
                capacity,
                expiry,
            );
            assert!(result.is_err());
        }
    }
}
