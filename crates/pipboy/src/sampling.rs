//! Explicit, bounded local host sampling. Output is NDJSON on stdout only.
//! No background service, data file, socket, or hardware device is opened.

use pipboy_core::host_telemetry::HostTelemetryModule;
use pipboy_core::metrics::Collector;
use pipboy_core::runtime::{ModuleHost, PermissionSet, PollOutcome};
use pipboy_core::telemetry::SnapshotSource;
use std::error::Error;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
struct Options {
    count: usize,
    interval_ms: u64,
    details: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            count: 3,
            interval_ms: 1_000,
            details: false,
        }
    }
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self::default();
        let mut count_seen = false;
        let mut interval_seen = false;
        let mut i = 0;

        while i < args.len() {
            let key = args[i].as_str();
            i += 1;
            if key == "--details" {
                if options.details {
                    return Err("--details repeated".into());
                }
                options.details = true;
                continue;
            }
            let raw = args
                .get(i)
                .ok_or_else(|| format!("{key} requires a value"))?;
            i += 1;

            match key {
                "--count" if !count_seen => {
                    options.count = raw.parse::<usize>().map_err(|_| "invalid --count")?;
                    count_seen = true;
                }
                "--interval-ms" if !interval_seen => {
                    options.interval_ms =
                        raw.parse::<u64>().map_err(|_| "invalid --interval-ms")?;
                    interval_seen = true;
                }
                "--count" | "--interval-ms" => return Err(format!("{key} repeated")),
                other => return Err(format!("unknown sampling option: {other}")),
            }
        }
        if !(1..=30).contains(&options.count) {
            return Err("--count must be 1..=30".into());
        }
        if !(500..=30_000).contains(&options.interval_ms) {
            return Err("--interval-ms must be 500..=30000".into());
        }
        Ok(options)
    }
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let options = Options::parse(&args)?;
    let min_ms = u64::try_from(Collector::minimum_cpu_interval().as_millis()).unwrap_or(u64::MAX);
    if options.interval_ms < min_ms {
        return Err(
            format!("CPU sampling interval must be at least {min_ms} ms on this platform").into(),
        );
    }

    let stdout = io::stdout();
    stream(
        || {
            let mut collector = Collector::new();
            collector.prime();
            Ok(collector)
        },
        &options,
        &mut stdout.lock(),
        thread::sleep,
    )
}

fn stream<S: SnapshotSource + Send + 'static, W: Write>(
    factory: impl FnMut() -> Result<S, String> + Send + 'static,
    options: &Options,
    output: &mut W,
    mut wait: impl FnMut(Duration),
) -> Result<(), Box<dyn Error>> {
    let mut host = ModuleHost::new(PermissionSet::offline_read_only());
    host.register(Box::new(HostTelemetryModule::new(
        factory,
        options.count,
        Duration::from_millis(options.interval_ms * 2),
    )?))?;
    host.start("stat")?;
    for index in 0..options.count {
        if index != 0 {
            wait(Duration::from_millis(options.interval_ms));
        }
        let poll = host.poll_once().pop().ok_or("missing host poll")?;
        let json = match poll.outcome {
            PollOutcome::Sample { data } if options.details => {
                serde_json::to_string(&data.host_report.ok_or("missing host field report")?)?
            }
            PollOutcome::Sample { data } => {
                serde_json::to_string(&data.host_observation.ok_or("missing host observation")?)?
            }
            other => return Err(format!("host telemetry did not sample: {other:?}").into()),
        };
        let result = writeln!(output, "{json}").and_then(|()| output.flush());
        match result {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn defaults_are_bounded_and_offline() {
        assert_eq!(Options::parse(&[]).unwrap(), Options::default());
        assert_eq!(Options::default().count, 3);
    }

    #[test]
    fn accepts_explicit_limits_in_any_order() {
        assert_eq!(
            Options::parse(&args(&["--interval-ms", "2000", "--count", "5"])).unwrap(),
            Options {
                count: 5,
                interval_ms: 2000,
                details: false
            },
        );
    }

    #[test]
    fn rejects_invalid_or_duplicated_parameters() {
        for values in [
            vec!["--count", "0"],
            vec!["--count", "31"],
            vec!["--count", "-1"],
            vec!["--interval-ms", "499"],
            vec!["--interval-ms", "30001"],
            vec!["--interval-ms", "NaN"],
            vec!["--count", "2", "--count", "3"],
            vec!["--bogus", "1"],
            vec!["--count"],
            vec!["--details", "--details"],
        ] {
            assert!(Options::parse(&args(&values)).is_err(), "{values:?}");
        }
    }
    struct Fixture(std::sync::Arc<std::sync::atomic::AtomicUsize>);

    impl SnapshotSource for Fixture {
        fn snapshot(&mut self) -> pipboy_core::metrics::Snapshot {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            pipboy_core::metrics::Snapshot {
                observed_unix_s: 1234,
                os: "fixture".into(),
                arch: "test".into(),
                cpu_percent: Some(0.0),
                logical_cores: 1,
                memory_used_mib: 0,
                memory_total_mib: 1,
                uptime_s: 10,
            }
        }
    }

    #[test]
    fn writes_finite_valid_ndjson_without_real_sleep() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut output = Vec::new();
        let mut waits = Vec::new();
        stream(
            {
                let calls = calls.clone();
                move || Ok(Fixture(calls.clone()))
            },
            &Options::default(),
            &mut output,
            |delay| waits.push(delay),
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap();
        let rows: Vec<_> = output
            .lines()
            .map(|line| {
                let row: pipboy_core::observation::Observation<pipboy_core::metrics::Snapshot> =
                    serde_json::from_str(line).unwrap();
                row.validate().unwrap();
                assert_eq!(row.source_id, "host.stats");
                assert_eq!(row.value.as_ref().unwrap().os, "fixture");
                row
            })
            .collect();
        assert_eq!(rows.len(), 3);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 3);
        assert_eq!(waits, vec![Duration::from_secs(1); 2]);
    }

    #[test]
    fn detailed_ndjson_uses_the_same_single_poll_with_units_and_real_zero() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut output = Vec::new();
        let options = Options::parse(&args(&["--details", "--count", "2"])).unwrap();
        stream(
            {
                let calls = calls.clone();
                move || Ok(Fixture(calls.clone()))
            },
            &options,
            &mut output,
            |_| {},
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap();
        let rows: Vec<
            pipboy_core::observation::Observation<pipboy_core::host_report::HostReadings>,
        > = output
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert_eq!(rows[0].value.as_ref().unwrap().poll_interval_ms, None);
        assert!(rows[1].value.as_ref().unwrap().poll_interval_ms.is_some());
        for row in rows {
            row.validate().unwrap();
            let fields = row.value.unwrap();
            assert_eq!(fields.cpu_percent.value, Some(0.0));
            assert_eq!(
                fields.cpu_percent.unit,
                pipboy_core::host_report::Unit::Percent
            );
            assert_eq!(fields.memory_used_mib.value, Some(0));
        }
    }

    struct FailedOutput(io::ErrorKind);

    impl Write for FailedOutput {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn closed_pipe_stops_before_another_sample_and_other_errors_propagate() {
        for kind in [io::ErrorKind::BrokenPipe, io::ErrorKind::PermissionDenied] {
            let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let result = stream(
                {
                    let calls = calls.clone();
                    move || Ok(Fixture(calls.clone()))
                },
                &Options::default(),
                &mut FailedOutput(kind),
                |_| panic!("must stop before waiting"),
            );
            assert_eq!(result.is_ok(), kind == io::ErrorKind::BrokenPipe);
            assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        }
    }
}
