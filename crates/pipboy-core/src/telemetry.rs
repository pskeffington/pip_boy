//! Shared, bounded host observations for the CLI and terminal frontend.
//! Collection and clocks are injected so fixtures never need host I/O or sleeps.

use crate::buffer::SampleBuffer;
use crate::host_report::{self, HostReadings};
use crate::metrics::{Collector, Snapshot};
use crate::observation::{Observation, Quality};
use std::time::Duration;

pub trait SnapshotSource {
    fn snapshot(&mut self) -> Snapshot;
}

impl SnapshotSource for Collector {
    fn snapshot(&mut self) -> Snapshot {
        self.sample()
    }
}

pub struct TelemetrySession<S> {
    source: S,
    history: SampleBuffer<Observation<Snapshot>>,
    expires_after_ms: u64,
    last_poll_ms: Option<u64>,
    poll_interval_ms: Option<u64>,
}

impl<S: SnapshotSource> TelemetrySession<S> {
    pub fn new(source: S, capacity: usize, expires_after: Duration) -> Result<Self, String> {
        let expires_after_ms = Self::validate_configuration(capacity, expires_after)?;
        Ok(Self {
            source,
            history: SampleBuffer::new(capacity)?,
            expires_after_ms,
            last_poll_ms: None,
            poll_interval_ms: None,
        })
    }

    pub(crate) fn validate_configuration(
        capacity: usize,
        expires_after: Duration,
    ) -> Result<u64, String> {
        SampleBuffer::<()>::new(capacity)?;
        let expires_after_ms = u64::try_from(expires_after.as_millis())
            .map_err(|_| "telemetry expiry exceeds supported duration")?;
        if expires_after_ms == 0 {
            return Err("telemetry expiry must be at least one millisecond".into());
        }
        Ok(expires_after_ms)
    }

    /// Caller supplies timestamps from one process-local monotonic clock.
    /// Reject clock rollback before collecting a new reading.
    pub fn poll(&mut self, session_ms: u64, wall_ms: Option<u64>) -> Result<(), String> {
        if self.last_poll_ms.is_some_and(|last| session_ms < last) {
            return Err("telemetry session clock moved backwards".into());
        }
        let observation = Observation::new(
            "host.stats",
            "local:sysinfo",
            wall_ms,
            session_ms,
            None,
            Some(self.source.snapshot()),
            Quality::Valid,
            Some(self.expires_after_ms),
        )?;
        self.history.push(observation);
        self.poll_interval_ms = self.last_poll_ms.map(|last| session_ms - last);
        self.last_poll_ms = Some(session_ms);
        Ok(())
    }

    /// A view of the latest reading with freshness evaluated at render time.
    pub fn latest(&self, session_ms: u64) -> Option<Observation<Snapshot>> {
        let mut observation = self.history.iter().last()?.clone();
        observation.update_freshness(session_ms);
        Some(observation)
    }

    pub fn retained(&self) -> usize {
        self.history.len()
    }

    pub fn latest_report(&self, session_ms: u64) -> Option<Observation<HostReadings>> {
        self.latest(session_ms)
            .map(|observation| host_report::describe(&observation, self.poll_interval_ms))
    }

    pub fn dropped(&self) -> u64 {
        self.history.dropped()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    struct Fixture(Rc<Cell<usize>>);

    impl SnapshotSource for Fixture {
        fn snapshot(&mut self) -> Snapshot {
            self.0.set(self.0.get() + 1);
            Snapshot {
                observed_unix_s: 1234,
                os: "fixture".into(),
                arch: "test".into(),
                cpu_percent: Some(0.0),
                logical_cores: 2,
                memory_used_mib: 10,
                memory_total_mib: 20,
                uptime_s: 100,
            }
        }
    }

    #[test]
    fn shared_session_bounds_history_and_marks_stale_at_expiry() {
        let calls = Rc::new(Cell::new(0));
        let mut session =
            TelemetrySession::new(Fixture(calls.clone()), 2, Duration::from_millis(500)).unwrap();
        assert!(session.latest(0).is_none());
        assert_eq!(calls.get(), 0);
        for tick in [0, 100, 200] {
            session.poll(tick, None).unwrap();
        }
        assert_eq!(session.retained(), 2);
        assert_eq!(session.dropped(), 1);
        let current = session.latest(699).unwrap();
        assert_eq!(current.quality, Quality::Valid);
        assert_eq!(current.captured_session_ms, 200);
        assert_eq!(current.provenance, "local:sysinfo");
        assert_eq!(current.value.unwrap().cpu_percent, Some(0.0));
        assert_eq!(session.latest(700).unwrap().quality, Quality::Stale);
        let report = session.latest_report(700).unwrap();
        let readings = report.value.unwrap();
        assert_eq!(readings.poll_interval_ms, Some(100));
        assert_eq!(readings.cpu_percent.quality, Quality::Stale);
        assert_eq!(readings.cpu_percent.value, Some(0.0));
        assert_eq!(calls.get(), 3); // Rendering does not collect.
    }

    #[test]
    fn rejects_clock_rollback_before_source_activity() {
        let calls = Rc::new(Cell::new(0));
        let mut session =
            TelemetrySession::new(Fixture(calls.clone()), 1, Duration::from_secs(1)).unwrap();
        session.poll(100, Some(1000)).unwrap();
        assert!(session.poll(99, Some(900)).is_err());
        assert_eq!(calls.get(), 1);
        assert_eq!(
            session.latest(100).unwrap().captured_wall_unix_ms,
            Some(1000)
        );
    }

    #[test]
    fn rejects_invalid_retention_and_expiry() {
        let source = || Fixture(Rc::new(Cell::new(0)));
        assert!(TelemetrySession::new(source(), 0, Duration::from_secs(1)).is_err());
        assert!(TelemetrySession::new(source(), 1, Duration::ZERO).is_err());
    }
}
