//! Explicit one-shot STAT collection through the same lifecycle host as the TUI
//! and finite sampling command. The CLI's raw Snapshot JSON shape is unchanged.

use pipboy_core::host_report::HostReadings;
use pipboy_core::host_telemetry::HostTelemetryModule;
use pipboy_core::metrics::{Collector, Snapshot};
use pipboy_core::observation::Observation;
use pipboy_core::runtime::{ModuleHost, ModuleSample, PermissionSet, PollOutcome};
use pipboy_core::telemetry::SnapshotSource;
use std::time::Duration;

/// Running the local `status` command is an explicit request for host metrics.
/// Sensitive device/network capabilities are not granted.
pub fn collect() -> Result<Snapshot, String> {
    collect_native()?
        .host_observation
        .and_then(|observation| observation.value)
        .ok_or_else(|| "missing STAT snapshot".to_owned())
}

pub fn collect_details() -> Result<Observation<HostReadings>, String> {
    collect_native()?
        .host_report
        .ok_or_else(|| "missing STAT field report".to_owned())
}

fn collect_native() -> Result<ModuleSample, String> {
    collect_with(PermissionSet::offline_read_only(), || {
        let mut collector = Collector::new();
        collector.prime();
        Ok(collector)
    })
}

/// Inject both policy and a lazy collector factory for deny-before-activity
/// fixtures. No factory is called until ModuleHost accepts its grant.
fn collect_with<S, F>(permissions: PermissionSet, factory: F) -> Result<ModuleSample, String>
where
    S: SnapshotSource + Send + 'static,
    F: FnMut() -> Result<S, String> + Send + 'static,
{
    let mut host = ModuleHost::new(permissions);
    host.register(Box::new(HostTelemetryModule::new(
        factory,
        1,
        Duration::from_secs(4),
    )?))?;
    host.start("stat")?;

    let result = host.poll_once().pop().ok_or("missing STAT poll")?;
    match result.outcome {
        PollOutcome::Sample { data } => Ok(*data),
        other => Err(format!("STAT did not sample: {other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct Fixture {
        samples: Arc<AtomicUsize>,
    }

    impl SnapshotSource for Fixture {
        fn snapshot(&mut self) -> Snapshot {
            self.samples.fetch_add(1, Ordering::SeqCst);
            Snapshot {
                observed_unix_s: 1234,
                os: "fixture".into(),
                arch: "test".into(),
                cpu_percent: Some(0.0),
                logical_cores: 1,
                memory_used_mib: 0,
                memory_total_mib: 64,
                uptime_s: 5,
            }
        }
    }

    #[test]
    fn missing_grant_never_constructs_or_samples_a_collector() {
        let factory_calls = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&factory_calls);
        let result = collect_with(PermissionSet::empty(), move || {
            count.fetch_add(1, Ordering::SeqCst);
            Ok(Fixture {
                samples: Arc::new(AtomicUsize::new(0)),
            })
        });
        assert!(result.unwrap_err().contains("denied"));
        assert_eq!(factory_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn a_one_shot_status_read_keeps_the_original_snapshot_json_shape() {
        let samples = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&samples);
        let data = collect_with(PermissionSet::offline_read_only(), move || {
            Ok(Fixture {
                samples: Arc::clone(&counter),
            })
        })
        .unwrap();

        assert_eq!(samples.load(Ordering::SeqCst), 1);
        let report = data.host_report.unwrap();
        assert_eq!(report.value.as_ref().unwrap().cpu_percent.value, Some(0.0));
        assert_eq!(report.value.as_ref().unwrap().poll_interval_ms, None);
        let snapshot = data.host_observation.unwrap().value.unwrap();
        let json = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(json["os"], "fixture");
        assert_eq!(json["cpu_percent"], 0.0);
        assert_eq!(json["memory_total_mib"], 64);
        assert_eq!(json.as_object().unwrap().len(), 8);
        assert!(json.get("schema_version").is_none());
        assert!(json.get("quality").is_none());
    }

    #[test]
    fn unavailable_collector_propagates_error_instead_of_fabricating_metrics() {
        let error = collect_with::<Fixture, _>(PermissionSet::offline_read_only(), || {
            Err("collector unavailable".to_owned())
        })
        .unwrap_err();
        assert_eq!(error, "collector unavailable");
    }
}
