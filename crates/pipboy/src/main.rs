use pipboy_core::blueprint::{literature, plans, validate_blueprint};
use pipboy_core::branding::{CLI_COMMAND, PRODUCT_DESCRIPTOR, PRODUCT_NAME};
use pipboy_core::catalog::{sources, validate_sources};
use pipboy_core::notes::NoteStore;
use pipboy_core::permissions::{
    adapter_capability, Preference, PreferenceStore, ADAPTER_CAPABILITIES,
};
use pipboy_core::runtime::{Capability, LocalOperation, ModuleHost, PermissionSet};
use std::error::Error;
mod console;
mod sampling;
mod status;
mod tui;

fn help() {
    println!("{PRODUCT_NAME} | {PRODUCT_DESCRIPTOR}");
    println!("usage: {CLI_COMMAND} [tui|status [--json]|sample [--count N --interval-ms MS]|sources [query]|modules|plan [G0..G4]|evidence [query]|doctor|permissions <list|request KEY|deny KEY|clear KEY>|notes <list|add TEXT|search QUERY|export|import FILE>|help]");
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
            let snapshot = status::collect()?;
            if option.as_deref() == Some("--json") {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            } else {
                println!("{PRODUCT_NAME} STAT / {} {}", snapshot.os, snapshot.arch);
                println!(
                    "CPU: {:.1}% across {} logical cores",
                    snapshot.cpu_percent, snapshot.logical_cores
                );
                println!(
                    "RAM: {} / {} MiB",
                    snapshot.memory_used_mib, snapshot.memory_total_mib
                );
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
            let host = ModuleHost::new(PermissionSet::offline_read_only());
            let entries = host.run_local(LocalOperation::LibraryRead, sources)?;
            validate_sources(&entries)?;
            for entry in entries.iter().filter(|entry| entry.matches(&query)) {
                println!(
                    "{} [{} / {}]\n  {}\n  {}",
                    entry.project,
                    entry.license,
                    entry.license_status,
                    entry.url(),
                    entry.summary
                );
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
                println!(
                    "{:<16} {:<3} {:<22} {:<3} {}",
                    module.id, module.gate, module.status, module.priority, module.name
                );
            }
            Ok(())
        }
        Some("plan") => {
            let gate = args.next();
            if args.next().is_some()
                || gate
                    .as_deref()
                    .is_some_and(|item| !matches!(item, "G0" | "G1" | "G2" | "G3" | "G4"))
            {
                return Err(format!("usage: {CLI_COMMAND} plan [G0|G1|G2|G3|G4]").into());
            }
            let modules = plans()?;
            let references = literature()?;
            validate_blueprint(&modules, &references)?;
            for module in modules
                .iter()
                .filter(|item| gate.is_none() || gate.as_deref() == Some(item.gate.as_str()))
            {
                println!(
                    "{} [{} / {}] {}",
                    module.id, module.gate, module.status, module.name
                );
                println!(
                    "  Requires: {}",
                    if module.depends_on.is_empty() {
                        "none".to_string()
                    } else {
                        module.depends_on.join(", ")
                    }
                );
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
                println!(
                    "{} [{} / {}] {}",
                    entry.id, entry.r#type, entry.year, entry.title
                );
                println!("  {}", entry.url);
                println!("  Design: {}", entry.design);
                println!("  Caveat: {}", entry.limitation);
            }
            Ok(())
        }
        Some("permissions") => {
            let verb = args.next();
            match verb.as_deref() {
                Some("list") if args.next().is_none() => {
                    let store = PreferenceStore::from_environment()?;
                    let entries = store.list()?;
                    for capability in ADAPTER_CAPABILITIES {
                        let preference = entries
                            .iter()
                            .find(|item| item.capability == capability.key())
                            .map(|item| item.preference.label())
                            .unwrap_or("unset");
                        println!("{:<20} {}", capability.key(), preference);
                    }
                    println!("INFO: preferences do not grant access or start adapters");
                    Ok(())
                }
                Some("request" | "deny" | "clear") => {
                    let key = args
                        .next()
                        .ok_or("usage: pipboy permissions <request|deny|clear> KEY")?;
                    if args.next().is_some() {
                        return Err("usage: pipboy permissions <request|deny|clear> KEY".into());
                    }
                    let capability = adapter_capability(&key)?;
                    let store = PreferenceStore::from_environment()?;
                    let changed = match verb.as_deref() {
                        Some("request") => store.set(capability, Preference::Requested)?,
                        Some("deny") => store.set(capability, Preference::Denied)?,
                        _ => store.clear(capability)?,
                    };
                    println!(
                        "{}: {} {}",
                        if changed { "UPDATED" } else { "UNCHANGED" },
                        capability.key(),
                        verb.as_deref().unwrap_or("unknown")
                    );
                    println!("INFO: this is a saved preference, not an active permission");
                    Ok(())
                }
                _ => Err(
                    "usage: pipboy permissions <list|request KEY|deny KEY|clear KEY>".into(),
                ),
            }
        }
        Some("doctor") => {
            let entries = sources()?;
            validate_sources(&entries)?;
            println!(
                "PASS: catalog parsed; {} distinct references",
                entries.len()
            );
            let modules = plans()?;
            let references = literature()?;
            validate_blueprint(&modules, &references)?;
            println!(
                "PASS: {} planned/implemented modules mapped to {} evidence records",
                modules.len(),
                references.len()
            );
            let store = NoteStore::from_environment()?;
            println!("INFO: notes location {}", store.path().display());
            let preferences = PreferenceStore::from_environment()?;
            println!(
                "PASS: {} persisted adapter preferences validated",
                preferences.list()?.len()
            );
            println!("INFO: map, radio, mesh, GPIO and device I/O are not implemented");
            println!("INFO: doctor does not probe or connect to nearby devices");
            Ok(())
        }
        Some("notes") => {
            let mut host = ModuleHost::new(PermissionSet::offline_read_only());
            match args.next().as_deref() {
                Some("list") if args.next().is_none() => {
                    let entries = host.run_local(LocalOperation::NotesRead, || {
                        NoteStore::from_environment()?.list()
                    })?;
                    for note in entries {
                        println!("#{} [{}] {}", note.id, note.created_unix_s, note.text);
                    }
                    Ok(())
                }
                Some("search") => {
                    let query = args.next().ok_or("usage: pipboy notes search QUERY")?;
                    if args.next().is_some() {
                        return Err("usage: pipboy notes search QUERY".into());
                    }
                    let entries = host.run_local(LocalOperation::NotesRead, || {
                        NoteStore::from_environment()?.search(&query)
                    })?;
                    for note in entries {
                        println!("#{} [{}] {}", note.id, note.created_unix_s, note.text);
                    }
                    Ok(())
                }
                Some("export") => {
                    if args.next().is_some() {
                        return Err("usage: pipboy notes export".into());
                    }
                    host.run_local(LocalOperation::NotesRead, || {
                        NoteStore::from_environment()?.export_to(std::io::stdout().lock())
                    })?;
                    Ok(())
                }
                Some("import") => {
                    let path = args.next().ok_or("usage: pipboy notes import FILE")?;
                    if args.next().is_some() {
                        return Err("usage: pipboy notes import FILE".into());
                    }
                    host.grant(Capability::FilesWrite);
                    let report = host.run_local(LocalOperation::NotesWrite, || {
                        NoteStore::from_environment()?.import_from(std::fs::File::open(path)?)
                    })?;
                    println!(
                        "Imported {} notes; skipped {} matching records; {} total",
                        report.imported, report.skipped, report.total
                    );
                    Ok(())
                }
                Some("add") => {
                    let text = args.collect::<Vec<_>>().join(" ");
                    host.grant(Capability::FilesWrite);
                    let note = host.run_local(LocalOperation::NotesWrite, || {
                        NoteStore::from_environment()?.add(&text)
                    })?;
                    println!("Saved local note #{}", note.id);
                    Ok(())
                }
                _ => Err(format!(
                    "usage: {CLI_COMMAND} notes <list|add TEXT|search QUERY|export|import FILE>"
                )
                .into()),
            }
        }
        Some(other) => Err(format!("unknown command: {other}. Use {CLI_COMMAND} help.").into()),
    }
}
