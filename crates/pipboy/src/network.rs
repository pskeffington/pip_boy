//! Explicit network-engineering CLI. No background monitoring daemon.

use pipboy_core::network_probe::{scan, NetworkReport};
use std::error::Error;
use std::thread;
use std::time::Duration;

const USAGE: &str =
    "usage: pipboy net <scan|watch> <local|PRIVATE_CIDR> --confirm [--json] [--interval-sec 30..3600 --count 1..60 (watch only)]";

fn display(report: &NetworkReport, json: bool) -> Result<(), Box<dyn Error>> {
    if json {
        println!("{}", serde_json::to_string(report)?);
    } else {
        println!("{}", report.render_text());
    }
    Ok(())
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut iter = args.into_iter();
    let mode = iter.next().ok_or(USAGE)?;
    if !matches!(mode.as_str(), "scan" | "watch") {
        return Err(USAGE.into());
    }
    let cidr = iter.next().ok_or(USAGE)?;
    let mut confirm = false;
    let mut json = false;
    let mut interval = 60_u64;
    let mut count = 5_u8;
    let mut interval_set = false;
    let mut count_set = false;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--confirm" if !confirm => confirm = true,
            "--json" if !json => json = true,
            "--interval-sec" if mode == "watch" && !interval_set => {
                interval = iter.next().ok_or(USAGE)?.parse()?;
                interval_set = true;
            }
            "--count" if mode == "watch" && !count_set => {
                count = iter.next().ok_or(USAGE)?.parse()?;
                count_set = true;
            }
            _ => return Err(USAGE.into()),
        }
    }
    if !confirm {
        return Err("NET requires --confirm: actively probe only systems and networks you are authorized to test".into());
    }
    if mode == "watch" && (!(30..=3600).contains(&interval) || !(1..=60).contains(&count)) {
        return Err("watch interval must be 30..3600 seconds and count 1..60".into());
    }
    let passes = if mode == "scan" { 1 } else { usize::from(count) };
    for n in 0..passes {
        let report = scan(&cidr, confirm)?;
        if mode == "watch" && !json {
            println!("SWEEP {} / {}", n + 1, passes);
        }
        display(&report, json)?;
        if n + 1 < passes {
            thread::sleep(Duration::from_secs(interval));
        }
    }
    Ok(())
}
