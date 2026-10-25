//! Explicit, bounded local host sampling. Output is NDJSON on stdout only.
//! No background service, data file, socket, or hardware device is opened.

use pipboy_core::metrics::Collector;
use pipboy_core::observation::Observation;
use std::error::Error;
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Eq)]
struct Options {
    count: usize,
    interval_ms: u64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            count: 3,
            interval_ms: 1_000,
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
            let raw = args.get(i).ok_or_else(|| format!("{key} requires a value"))?;
            i += 1;

            match key {
                "--count" if !count_seen => {
                    options.count = raw.parse::<usize>().map_err(|_| "invalid --count")?;
                    count_seen = true;
                }
                "--interval-ms" if !interval_seen => {
                    options.interval_ms = raw.parse::<u64>().map_err(|_| "invalid --interval-ms")?;
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
    let min_ms = u64::try_from(Collector::minimum_cpu_interval().as_millis())
        .unwrap_or(u64::MAX);
    if options.interval_ms < min_ms {
        return Err(format!("CPU sampling interval must be at least {min_ms} ms on this platform").into());
    }

    let mut collector = Collector::new();
    collector.prime();
    let session_start = Instant::now();
    let stdout = io::stdout();
    let mut output = stdout.lock();

    for index in 0..options.count {
        if index != 0 {
            thread::sleep(Duration::from_millis(options.interval_ms));
        }
        let snapshot = collector.sample();
        let observation = Observation::measured(
            "host.stats",
            "local:sysinfo",
            snapshot,
            session_start,
            Duration::from_millis(options.interval_ms.saturating_mul(2)),
        )?;
        writeln!(output, "{}", serde_json::to_string(&observation)?)?;
        output.flush()?;
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
            Options { count: 5, interval_ms: 2000 },
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
        ] {
            assert!(Options::parse(&args(&values)).is_err(), "{values:?}");
        }
    }
}
