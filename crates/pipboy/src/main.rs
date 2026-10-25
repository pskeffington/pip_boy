use pipboy_core::branding::{CLI_COMMAND, PRODUCT_DESCRIPTOR, PRODUCT_NAME};
use pipboy_core::catalog::{sources, validate_sources};
use pipboy_core::blueprint::{literature, plans, validate_blueprint};
use pipboy_core::metrics::Collector;
use pipboy_core::notes::NoteStore;
use std::error::Error;
mod sampling;
mod tui;

fn help() {
    println!("{PRODUCT_NAME} | {PRODUCT_DESCRIPTOR}");
    println!("usage: {CLI_COMMAND} [tui|status [--json]|sample [--count N --interval-ms MS]|sources [query]|modules|plan [G0..G4]|evidence [query]|doctor|notes <list|add TEXT>|help]");
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
        Some("sample") => sampling::run(args.collect()),
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
            if args.next().is_some() {
                return Err(format!("usage: {CLI_COMMAND} modules").into());
            }
            let modules = plans()?;
            let references = literature()?;
            validate_blueprint(&modules, &references)?;
            for module in modules {
                println!("{:<16} {:<3} {:<22} {:<3} {}", module.id, module.gate, module.status, module.priority, module.name);
            }
            Ok(())
        }
        Some("plan") => {
            let gate = args.next();
            if args.next().is_some() || gate.as_deref().is_some_and(|item| !matches!(item, "G0" | "G1" | "G2" | "G3" | "G4")) {
                return Err(format!("usage: {CLI_COMMAND} plan [G0|G1|G2|G3|G4]").into());
            }
            let modules = plans()?;
            let references = literature()?;
            validate_blueprint(&modules, &references)?;
            for module in modules.iter().filter(|item| gate.is_none() || gate.as_deref() == Some(item.gate.as_str())) {
                println!("{} [{} / {}] {}", module.id, module.gate, module.status, module.name);
                println!("  Requires: {}", if module.depends_on.is_empty() { "none".to_string() } else { module.depends_on.join(", ") });
                println!("  Features: {}", module.features.join("; "));
                println!("  Evidence: {}", module.evidence.join(", "));
                println!("  Accept: {}", module.acceptance.join(" | "));
            }
            Ok(())
        }
        Some("evidence") => {
            let query = args.next().unwrap_or_default();
            if args.next().is_some() {
                return Err(format!("usage: {CLI_COMMAND} evidence [query]").into());
            }
            let modules = plans()?;
            let references = literature()?;
            validate_blueprint(&modules, &references)?;
            for entry in references.iter().filter(|item| item.matches(&query)) {
                println!("{} [{} / {}] {}", entry.id, entry.r#type, entry.year, entry.title);
                println!("  {}", entry.url);
                println!("  Design: {}", entry.design);
                println!("  Caveat: {}", entry.limitation);
            }
            Ok(())
        }
        Some("doctor") => {
            let entries = sources()?;
            validate_sources(&entries)?;
            println!("PASS: catalog parsed; {} distinct references", entries.len());
            let modules = plans()?;
            let references = literature()?;
            validate_blueprint(&modules, &references)?;
            println!("PASS: {} planned/implemented modules mapped to {} evidence records", modules.len(), references.len());
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
