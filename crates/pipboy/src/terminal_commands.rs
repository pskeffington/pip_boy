//! Selected-tool command terminal. Executes only reviewed, offline native handlers.
//! No shell, dynamic module loading, network activity or implicit permission grants.
use pipboy_core::catalog::sources;
use pipboy_core::network_suite;
use pipboy_core::workbench::{crc32_hex, encode_hex, inspect_hex, tool, Workbench, WorkbenchStore};
use std::error::Error;
use std::io::{self, Write};

const MAX_INPUT: usize = 512;
const OFFLINE_TOOLS: &[&str] = &["host", "diag", "library", "byte-lab", "net-lab"];

fn supported(id: &str) -> bool {
    OFFLINE_TOOLS.contains(&id)
}

fn selected_tools(board: &Workbench) -> Vec<&str> {
    board.slots.iter().filter_map(|slot| {
        let id = slot.tool.as_str();
        (tool(id).is_some() && supported(id)).then_some(id)
    }).collect()
}

fn commands(id: &str) -> &'static str {
    match id {
        "host" => "status",
        "diag" => "host",
        "library" => "sources [query]",
        "byte-lab" => "encode TEXT | inspect HEX | crc32 HEX",
        "net-lab" => "list | cidr INPUT | cidr6 INPUT | split4 INPUT PREFIX | route INPUT ROUTE | aggregate4 INPUT | overlap4 INPUT | mac INPUT | vlan INPUT | mtu INPUT",
        _ => "No terminal commands connected to this tool.",
    }
}

fn dispatch(id: &str, tokens: &[&str]) -> Result<(), Box<dyn Error>> {
    match (id, tokens) {
        ("host", ["status"]) => {
            let snapshot = crate::status::collect()?;
            println!("SYSTEM STAT / {} {} | RAM {} / {} MiB | UPTIME {} s",
                snapshot.os, snapshot.arch, snapshot.memory_used_mib,
                snapshot.memory_total_mib, snapshot.uptime_s);
            Ok(())
        }
        ("diag", ["host"]) => crate::diagnostics::run(vec!["host".into()]),
        ("library", ["sources", query @ ..]) if query.len() <= 8 => {
            let query = query.join(" ");
            for item in sources()?.into_iter().filter(|item| item.matches(&query)) {
                println!("{} [{}]: {}", item.project, item.license, item.summary);
            }
            Ok(())
        }
        ("byte-lab", ["encode", rest @ ..]) if !rest.is_empty() => {
            println!("{}", encode_hex(&rest.join(" "))?); Ok(())
        }
        ("byte-lab", ["inspect", value]) => { print!("{}", inspect_hex(value)?); Ok(()) }
        ("byte-lab", ["crc32", value]) => { println!("{}", crc32_hex(value)?); Ok(()) }
        ("net-lab", ["list"]) => {
            for op in network_suite::OPERATIONS.iter().filter(|op| !op.active) {
                println!("{} / {}", op.id, op.name);
            }
            Ok(())
        }
        ("net-lab", [verb, value]) if matches!(*verb, "cidr"|"cidr6"|"aggregate4"|"mac"|"vlan"|"mtu") => {
            println!("{}", network_suite::run_offline(verb, value, None)?); Ok(())
        }
        ("net-lab", [verb, value, extra]) if matches!(*verb, "split4"|"route"|"overlap4") => {
            println!("{}", network_suite::run_offline(verb, value, Some(extra))?); Ok(())
        }
        _ => Err(format!("NOT AVAILABLE: {id} supports: {}", commands(id)).into()),
    }
}

fn route<'a>(board: &'a Workbench, selected: &mut Option<&'a str>, input: &str) -> Result<bool, Box<dyn Error>> {
    if input.len() > MAX_INPUT || input.chars().any(|c| c.is_control()) {
        return Err("invalid or oversized command".into());
    }
    let tokens: Vec<&str> = input.split_whitespace().collect();
    match tokens.as_slice() {
        [] => Ok(true),
        ["quit"] | ["exit"] => Ok(false),
        ["tools"] => {
            println!("Installed terminal handlers for pinned Workbench tools:");
            for id in selected_tools(board) { println!("  {id}{}", if Some(id) == *selected { " [selected]" } else { "" }); }
            println!("Other pinned tools are not executable in this terminal.");
            Ok(true)
        }
        ["select", id] => {
            let pinned = board.slots.iter().find(|slot| slot.tool == *id && supported(id))
                .ok_or("tool is not pinned or has no validated offline terminal handler")?;
            *selected = Some(pinned.tool.as_str());
            println!("SELECTED {id}; no action started");
            Ok(true)
        }
        ["help"] => {
            println!("tools | select TOOL | help | run COMMAND | quit");
            println!("Only explicitly supported offline Rust handlers are callable.");
            if let Some(id) = selected { println!("{id}: {}", commands(id)); }
            Ok(true)
        }
        ["run", rest @ ..] => {
            let id = selected.ok_or("no offline tool selected; use tools and select TOOL")?;
            dispatch(id, rest)?;
            Ok(true)
        }
        _ => Err("unknown command; use help (no shell commands accepted)".into()),
    }
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() { return Err("usage: pipboy terminal".into()); }
    let board = WorkbenchStore::from_environment()?.load()?;
    let initial = selected_tools(&board).first().copied();
    let mut selected = initial;
    println!("Rust-Boy / selected-tool command terminal / OFFLINE HANDLERS ONLY");
    println!("Workbench: {} | profile {} | {} pins", board.title, board.profile, board.slots.len());
    println!("Selected: {} | type 'help' or 'tools'", initial.unwrap_or("none"));
    let stdin = io::stdin();
    let mut line = String::new();
    loop {
        print!("rb:{}> ", selected.unwrap_or("none"));
        io::stdout().flush()?;
        line.clear();
        if stdin.read_line(&mut line)? == 0 { break; }
        let input = line.trim_end_matches(['\r', '\n']);
        match route(&board, &mut selected, input) {
            Ok(true) => (),
            Ok(false) => break,
            Err(error) => eprintln!("DENIED/ERROR: {error}"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pipboy_core::workbench::PRESETS;

    #[test]
    fn toolset_selection_never_executes() {
        let board = Workbench::from_preset(&PRESETS[0]);
        let mut selected = None;
        assert!(route(&board, &mut selected, "select host").unwrap());
        assert_eq!(selected, Some("host"));
        assert!(route(&board, &mut selected, "run status").unwrap());
    }

    #[test]
    fn refuses_unpinned_and_unsupported_tools() {
        let board = Workbench::from_preset(&PRESETS[0]);
        let mut selected = None;
        assert!(route(&board, &mut selected, "select net-lab").is_err());
        assert!(route(&board, &mut selected, "select radio").is_err());
        assert!(route(&board, &mut selected, "run status").is_err());
        assert!(route(&board, &mut selected, "sh -c id").is_err());
        assert!(route(&board, &mut selected, "run net scan local --confirm").is_err());
    }

    #[test]
    fn limits_input() {
        let board = Workbench::from_preset(&PRESETS[0]);
        let mut selected = None;
        assert!(route(&board, &mut selected, &"A".repeat(MAX_INPUT + 1)).is_err());
        assert!(route(&board, &mut selected, "run status\u{0000}").is_err());
    }
}
