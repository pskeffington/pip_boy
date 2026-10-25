use pipboy_core::branding::{CLI_COMMAND, PRODUCT_DESCRIPTOR, PRODUCT_NAME};
use pipboy_core::catalog::{sources, validate_sources};
use pipboy_core::metrics::Collector;
use pipboy_core::notes::NoteStore;
use pipboy_core::registry::MODULES;
use std::error::Error;
mod tui;

fn help() {
    println!("{PRODUCT_NAME} | {PRODUCT_DESCRIPTOR}");
    println!("usage: {CLI_COMMAND} [tui|status [--json]|sources [query]|modules|doctor|notes <list|add TEXT>|help]");
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{PRODUCT_NAME}: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None | Some("tui") => tui::run(),
        Some("help" | "--help" | "-h") => {
            help();
            Ok(())
        }
        Some("status") => {
            let option = args.next();
            if option.as_deref().is_some_and(|value| value != "--json") || args.next().is_some() {
                return Err(format!("usage: {CLI_COMMAND} status [--json]").into());
            }
            let mut collector = Collector::new();
            collector.prime();
            let snapshot = collector.sample();
            if option.as_deref() == Some("--json") {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            } else {
                println!("{PRODUCT_NAME} STAT / {} {}", snapshot.os, snapshot.arch);
                println!("CPU: {:.1}% across {} logical cores", snapshot.cpu_percent, snapshot.logical_cores);
                println!("RAM: {} / {} MiB", snapshot.memory_used_mib, snapshot.memory_total_mib);
                println!("UPTIME: {} s", snapshot.uptime_s);
            }
            Ok(())
        }
        Some("sources") => {
            let query = args.next().unwrap_or_default();
            if args.next().is_some() {
                return Err(format!("usage: {CLI_COMMAND} sources [query]").into());
            }
            let entries = sources()?;
            validate_sources(&entries)?;
            for entry in entries.iter().filter(|entry| entry.matches(&query)) {
                println!("{} [{} / {}]\n  {}\n  {}", entry.project, entry.license, entry.license_status, entry.url(), entry.summary);
            }
            Ok(())
        }
        Some("modules") => {
            for module in MODULES {
                println!("{:<10} {:<10} {}", module.id, format!("{:?}", module.readiness), module.description);
            }
            Ok(())
        }
        Some("doctor") => {
            let entries = sources()?;
            validate_sources(&entries)?;
            println!("PASS: catalog parsed; {} distinct references", entries.len());
            println!("PASS: {} registered modules", MODULES.len());
            let store = NoteStore::from_environment()?;
            println!("INFO: notes location {}", store.path().display());
            println!("INFO: map, radio, mesh, GPIO and device I/O are not implemented");
            println!("INFO: doctor does not probe or connect to nearby devices");
            Ok(())
        }
        Some("notes") => {
            let store = NoteStore::from_environment()?;
            match args.next().as_deref() {
                Some("list") if args.next().is_none() => {
                    for note in store.list()? {
                        println!("#{} [{}] {}", note.id, note.created_unix_s, note.text);
                    }
                    Ok(())
                }
                Some("add") => {
                    let text = args.collect::<Vec<_>>().join(" ");
                    let note = store.add(&text)?;
                    println!("Saved local note #{}", note.id);
                    Ok(())
                }
                _ => Err(format!("usage: {CLI_COMMAND} notes <list|add TEXT>").into()),
            }
        }
        Some(other) => Err(format!("unknown command: {other}. Use {CLI_COMMAND} help.").into()),
    }
}
