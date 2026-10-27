use pipboy_core::blueprint::{literature, plans, validate_blueprint};
use pipboy_core::branding::{CLI_COMMAND, PRODUCT_DESCRIPTOR, PRODUCT_NAME};
use pipboy_core::catalog::{sources, validate_sources};
use pipboy_core::device_simulator::{SessionConfirmation, SimulatedBroker};
use pipboy_core::mock_adapters::{
    DisplayBackend, InputSource, MockDisplayBackend, MockInputSource, MockSerialReader,
    SerialReader,
};
use pipboy_core::notes::NoteStore;
use pipboy_core::permissions::{
    adapter_capability, Preference, PreferenceStore, ADAPTER_CAPABILITIES,
};
use pipboy_core::runtime::{Capability, LocalOperation, ModuleHost, PermissionSet};
use pipboy_core::simulator_replay::{replay_fixture, ReplayScenario};
use pipboy_core::virtual_transport::VirtualTransport;
use std::error::Error;
mod console;
mod county_cache;
mod dashboard;
mod diagnostics;
mod endpoint_probe;
mod game;
mod game_discovery;
mod game_view;
mod network;
mod peer;
mod radio;
mod radio_http;
mod radio_output;
mod radio_playback;
mod sampling;
mod scout;
mod status;
mod tui;
mod workbench;
mod zip_index;

fn help() {
    println!("{PRODUCT_NAME} | {PRODUCT_DESCRIPTOR}");
    println!("usage: {CLI_COMMAND} <command> [options]");
    for line in [
        "",
        "INTERFACES",
        "  tui                                terminal console (default)",
        "  dashboard [--port PORT] [--open] [--micro]",
        "  game --discover-ps4 --confirm [--json]",
        "  game (--connect IP:PORT | --playstation IP | --ps4 IP) --confirm",
        "       [--snapshot|--diagnose|--observe-seconds N]",
        "",
        "STATUS & OFFLINE DATA",
        "  status [--details] [--json]",
        "  sample [--count N --interval-ms MS --details]",
        "  sources [query] | modules | plan [G0..G4] | evidence [query]",
        "  notes <list|add TEXT|search QUERY|export|import FILE>",
        "  scout geo <waypoint|distance|inspect>",
        "  workbench <list|profiles|use|palette|add|remove|rename|clear|bytes|net>",
        "  radio <capabilities|source|stations [--check-ip]|check STATION_ID --confirm>",
        "  radio <probe STATION_ID --confirm|validate STATION_ID --confirm>",
        "  radio listen STATION_ID --confirm [--seconds 1..300]",
        "  radio simulate FREQ_HZ MODE",
        "",
        "NETWORK & COMMUNICATIONS (explicit authorization required)",
        "  net <scan|watch> <local|PRIVATE_CIDR> --confirm [--json]",
        "  net unifi <sites|clients SITE|compare SITE CIDR> --confirm [--json]",
        "  peer <init|trust|add|list|send|sync|message-send|message-status>",
        "  peer <message-retry|message-retry-local|message-key-init>",
        "  peer <meshtastic-capture|pace-show|pace-local|message-pace>",
        "",
        "DIAGNOSTICS & DEVELOPMENT",
        "  diag <host [--json]|ingest FILE --asset ID --confirm-authorized [--json]>",
        "  permissions <list|request KEY|deny KEY|clear KEY>",
        "  simulate <list|run ID [--confirm]|replay ID SCENARIO [--confirm]>",
        "  simulate transport ID --count N --capacity N [--confirm]",
        "  simulate adapter <input|display|serial> [--confirm]",
        "  doctor | help | --help",
    ] {
        println!("{line}");
    }
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
            let mut json = false;
            let mut details = false;
            for option in args {
                match option.as_str() {
                    "--json" if !json => json = true,
                    "--details" if !details => details = true,
                    _ => return Err(format!("usage: {CLI_COMMAND} status [--details] [--json]").into()),
                }
            }
            if details {
                let report = status::collect_details()?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    println!("{PRODUCT_NAME} STAT / field quality");
                    if let Some(readings) = &report.value {
                        println!("{}", pipboy_core::host_report::display_fields(readings));
                    }
                    println!("SOURCE: {} | {:?}", report.provenance, report.quality);
                }
                return Ok(());
            }
            let snapshot = status::collect()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            } else {
                println!("{PRODUCT_NAME} STAT / {} {}", snapshot.os, snapshot.arch);
                println!(
                    "CPU: {} across {} logical cores",
                    snapshot.cpu_percent.map(|value| format!("{value:.1}%")).unwrap_or_else(|| "Unknown (warming up or unavailable)".into()), snapshot.logical_cores
                );
                println!(
                    "RAM: {} / {} MiB",
                    snapshot.memory_used_mib, snapshot.memory_total_mib
                );
                println!("UPTIME: {} s", snapshot.uptime_s);
            }
            Ok(())
        }
        Some("game") => game::run(args.collect()),
        Some("diag") => diagnostics::run(args.collect()),
        Some("net") => network::run(args.collect()),
        Some("peer") => peer::run(args.collect()),
        Some("radio") => radio::run(args.collect()),
        Some("scout") => scout::run(args.collect()),
        Some("dashboard") => dashboard::run(args.collect()),
        Some("workbench" | "bench") => workbench::run(args.collect()),
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
                _ => Err("usage: pipboy permissions <list|request KEY|deny KEY|clear KEY>".into()),
            }
        }
        Some("simulate") => match args.next().as_deref() {
            Some("list") if args.next().is_none() => {
                for device in SimulatedBroker::fixtures() {
                    println!(
                        "{:<20} {:<18} {} (SIMULATED)",
                        device.id,
                        device.capability.key(),
                        device.description
                    );
                }
                println!("INFO: fixed in-memory fixtures; no OS device discovery");
                Ok(())
            }
            Some("run") => {
                let device_id = args
                    .next()
                    .ok_or("usage: pipboy simulate run ID [--confirm]")?;
                let confirmation = match args.next().as_deref() {
                    Some("--confirm") if args.next().is_none() => SessionConfirmation::Confirmed,
                    None => SessionConfirmation::Absent,
                    _ => return Err("usage: pipboy simulate run ID [--confirm]".into()),
                };
                let preferences = PreferenceStore::from_environment()?.list()?;
                let mut broker = SimulatedBroker::new(&preferences)?;
                let lease = broker.open(&device_id, confirmation)?;
                let event = broker.sample(&lease)?;
                broker.close(&lease)?;
                println!("{}", serde_json::to_string_pretty(&event)?);
                Ok(())
            }
            Some("transport") => {
                let usage =
                    "usage: pipboy simulate transport ID --count N --capacity N [--confirm]";
                let id = args.next().ok_or(usage)?;
                if args.next().as_deref() != Some("--count") {
                    return Err(usage.into());
                }
                let count = args.next().ok_or(usage)?.parse::<usize>()?;
                if args.next().as_deref() != Some("--capacity") {
                    return Err(usage.into());
                }
                let capacity = args.next().ok_or(usage)?.parse::<usize>()?;
                let confirmation = match args.next().as_deref() {
                    Some("--confirm") if args.next().is_none() => SessionConfirmation::Confirmed,
                    None => SessionConfirmation::Absent,
                    _ => return Err(usage.into()),
                };
                let preferences = PreferenceStore::from_environment()?.list()?;
                let mut transport = VirtualTransport::new(&preferences, capacity)?;
                transport.open(&id, confirmation)?;
                transport.pump(count)?;
                let dropped = transport.dropped();
                let mut retained = Vec::with_capacity(transport.retained());
                while let Some(event) = transport.receive()? {
                    retained.push(event);
                }
                transport.close()?;
                let capability = retained
                    .first()
                    .ok_or("virtual transport generated no events")?
                    .capability;
                let report = serde_json::json!({
                    "schema_version": 1,
                    "simulated": true,
                    "source": "rust-boy/bounded-virtual-transport",
                    "device_id": id,
                    "capability": capability,
                    "capacity": capacity,
                    "requested": count,
                    "dropped": dropped,
                    "retained": retained
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
                Ok(())
            }
            Some("adapter") => {
                let usage = "usage: pipboy simulate adapter <input|display|serial> [--confirm]";
                let kind = args.next().ok_or(usage)?;
                if !matches!(kind.as_str(), "input" | "display" | "serial") {
                    return Err(usage.into());
                }
                let confirmation = match args.next().as_deref() {
                    Some("--confirm") if args.next().is_none() => SessionConfirmation::Confirmed,
                    None => SessionConfirmation::Absent,
                    _ => return Err(usage.into()),
                };
                let preferences = PreferenceStore::from_environment()?.list()?;
                match kind.as_str() {
                    "input" => {
                        let mut input = MockInputSource::open(&preferences, confirmation)?;
                        let sample = input.poll_input()?;
                        println!("{}", serde_json::to_string_pretty(&sample)?);
                    }
                    "display" => {
                        let mut display = MockDisplayBackend::open(&preferences, confirmation)?;
                        let receipt = display.submit_frame("RUST-BOY")?;
                        println!("{}", serde_json::to_string_pretty(&receipt)?);
                    }
                    "serial" => {
                        let mut serial =
                            MockSerialReader::open(&preferences, confirmation, b"RB")?;
                        let frame = serial.read_frame()?;
                        println!("{}", serde_json::to_string_pretty(&frame)?);
                    }
                    _ => unreachable!("validated adapter name"),
                }
                Ok(())
            }
            Some("replay") => {
                let id = args
                    .next()
                    .ok_or("usage: pipboy simulate replay ID SCENARIO [--confirm]")?;
                let scenario_key = args
                    .next()
                    .ok_or("usage: pipboy simulate replay ID SCENARIO [--confirm]")?;
                let scenario = ReplayScenario::from_key(&scenario_key).ok_or(
                    "unknown replay scenario (nominal|disconnect-recover|revoke|cancel|timeout)",
                )?;
                let confirmation = match args.next().as_deref() {
                    Some("--confirm") if args.next().is_none() => SessionConfirmation::Confirmed,
                    None => SessionConfirmation::Absent,
                    _ => return Err("usage: pipboy simulate replay ID SCENARIO [--confirm]".into()),
                };
                let preferences = PreferenceStore::from_environment()?.list()?;
                let trace = replay_fixture(&id, scenario, &preferences, confirmation)?;
                println!("{}", serde_json::to_string_pretty(&trace)?);
                Ok(())
            }
            _ => Err(
                "usage: pipboy simulate <list|run ID [--confirm]|replay ID SCENARIO [--confirm]|transport ID --count N --capacity N [--confirm]|adapter <input|display|serial> [--confirm]>"
                    .into(),
            ),
        },
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
            println!(
                "INFO: {} fixed in-memory device fixtures; no real device backend",
                SimulatedBroker::fixtures().len()
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
