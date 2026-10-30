//! Selected-tool command terminal. Executes only reviewed, offline native handlers.
//! No shell, dynamic module loading, network activity or implicit permission grants.
use pipboy_core::catalog::sources;
use pipboy_core::command_catalog::{parse_input, TerminalAction, TypedArguments, MAX_COMMAND_BYTES};
use pipboy_core::network_suite;
use pipboy_core::workbench::{crc32_hex, encode_hex, inspect_hex, tool, Workbench, WorkbenchStore};
use std::error::Error;
use std::io::{self, Write};

const MAX_INPUT: usize = MAX_COMMAND_BYTES;
const OFFLINE_TOOLS: &[&str] = &["host", "diag", "library", "byte-lab", "net-lab"];

fn supported(id: &str) -> bool {
    OFFLINE_TOOLS.contains(&id)
}

fn selected_tools(board: &Workbench) -> Vec<&str> {
    board
        .slots
        .iter()
        .filter_map(|slot| {
            let id = slot.tool.as_str();
            (tool(id).is_some() && supported(id)).then_some(id)
        })
        .collect()
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
            println!(
                "SYSTEM STAT / {} {} | RAM {} / {} MiB | UPTIME {} s",
                snapshot.os,
                snapshot.arch,
                snapshot.memory_used_mib,
                snapshot.memory_total_mib,
                snapshot.uptime_s
            );
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
            println!("{}", encode_hex(&rest.join(" "))?);
            Ok(())
        }
        ("byte-lab", ["inspect", value]) => {
            print!("{}", inspect_hex(value)?);
            Ok(())
        }
        ("byte-lab", ["crc32", value]) => {
            println!("{}", crc32_hex(value)?);
            Ok(())
        }
        ("net-lab", ["list"]) => {
            for op in network_suite::OPERATIONS.iter().filter(|op| !op.active) {
                println!("{} / {}", op.id, op.name);
            }
            Ok(())
        }
        ("net-lab", [verb, value])
            if matches!(
                *verb,
                "cidr" | "cidr6" | "aggregate4" | "mac" | "vlan" | "mtu"
            ) =>
        {
            println!("{}", network_suite::run_offline(verb, value, None)?);
            Ok(())
        }
        ("net-lab", [verb, value, extra]) if matches!(*verb, "split4" | "route" | "overlap4") => {
            println!("{}", network_suite::run_offline(verb, value, Some(extra))?);
            Ok(())
        }
        _ => Err(format!("NOT AVAILABLE: {id} supports: {}", commands(id)).into()),
    }
}

fn select_tool<'a>(
    board: &'a Workbench,
    selected: &mut Option<&'a str>,
    id: &str,
) -> Result<(), Box<dyn Error>> {
    let pinned = board
        .slots
        .iter()
        .find(|slot| slot.tool == id && supported(id))
        .ok_or("tool is not pinned or has no validated offline terminal handler")?;
    *selected = Some(pinned.tool.as_str());
    println!("SELECTED {id}; available: {}", commands(id));
    Ok(())
}

fn route<'a>(
    board: &'a Workbench,
    selected: &mut Option<&'a str>,
    input: &str,
) -> Result<bool, Box<dyn Error>> {
    // A copied shell prompt is never a command. The shared parser owns all
    // remaining command grammar, bounds, and workbench pin checks.
    if input.starts_with("rb:") || input.starts_with("null@nulls-Laptop") {
        return Err("paste only the command text after the prompt".into());
    }
    let pinned = selected_tools(board);
    match parse_input(input, *selected, &pinned)? {
        TerminalAction::Empty => Ok(true),
        TerminalAction::Quit => Ok(false),
        TerminalAction::ListTools(tools) => {
            println!("Available offline handlers for pinned Workbench tools:");
            for id in tools {
                println!("  {id}{}", if Some(id) == *selected { " [selected]" } else { "" });
            }
            println!("Select a tool by typing its name, e.g. net-lab.");
            Ok(true)
        }
        TerminalAction::Help { tool_id } => {
            println!("tools | TOOL | select TOOL | COMMAND | run COMMAND | q | quit | exit");
            println!("The tool name selects a pinned offline handler without executing it.");
            println!("Commands are available only for the selected tool; no shell commands.");
            if let Some(id) = tool_id {
                println!("{id}: {}", commands(id));
            }
            Ok(true)
        }
        TerminalAction::SelectTool { tool_id } => {
            select_tool(board, selected, tool_id)?;
            Ok(true)
        }
        TerminalAction::Execute(request) => {
            // Only the registered offline/read-only catalog can reach the
            // legacy native executor during this bounded migration.
            if request.side_effect != pipboy_core::command_catalog::SideEffectClass::ReadOnly
                || request.connectivity != pipboy_core::command_catalog::ConnectivityState::Offline
                || request.required_scope.is_some()
                || request.confirmation_required
            {
                return Err("command is not approved for offline terminal execution".into());
            }
            let mut tokens = vec![request.verb];
            let owned: Vec<String> = match request.arguments {
                TypedArguments::None => Vec::new(),
                TypedArguments::Text { value } => value.split_whitespace().map(str::to_owned).collect(),
                TypedArguments::Hex { value } => vec![value],
                TypedArguments::Network { input, extra, .. } => {
                    let mut values = vec![input];
                    if let Some(extra) = extra { values.push(extra); }
                    values
                }
            };
            tokens.extend(owned.iter().map(String::as_str));
            dispatch(request.tool_id, &tokens)?;
            Ok(true)
        }
    }
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("usage: pipboy terminal".into());
    }
    let board = WorkbenchStore::from_environment()?.load()?;
    let initial = selected_tools(&board).first().copied();
    let mut selected = initial;
    println!("Rust-Boy / selected-tool command terminal / OFFLINE HANDLERS ONLY");
    println!(
        "Workbench: {} | profile {} | {} pins",
        board.title,
        board.profile,
        board.slots.len()
    );
    println!(
        "Selected: {} | type 'help' or 'tools'",
        initial.unwrap_or("none")
    );
    let stdin = io::stdin();
    let mut line = String::new();
    loop {
        print!("rb:{}> ", selected.unwrap_or("none"));
        io::stdout().flush()?;
        line.clear();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
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
    fn direct_selection_and_commands() {
        let board = Workbench::from_preset(&PRESETS[2]); // Security
        let mut selected = None;
        assert!(route(&board, &mut selected, "net-lab").unwrap());
        assert_eq!(selected, Some("net-lab"));
        assert!(route(&board, &mut selected, "list").unwrap());
        assert!(route(&board, &mut selected, "host").unwrap());
        assert_eq!(selected, Some("host"));
        assert!(route(&board, &mut selected, "status").unwrap());
        assert!(route(&board, &mut selected, "byte-lab").unwrap());
        assert!(route(&board, &mut selected, "encode Rust Boy").unwrap());
        assert!(!route(&board, &mut selected, "q").unwrap());
    }

    #[test]
    fn prompt_paste_and_active_network_remain_rejected() {
        let board = Workbench::from_preset(&PRESETS[2]); // Security
        let mut selected = Some("host");
        assert!(route(&board, &mut selected, "rb:host> run status").is_err());
        assert!(route(&board, &mut selected, "net scan local --confirm").is_err());
        assert!(route(&board, &mut selected, "select net-lab run").is_err());
        assert!(route(&board, &mut selected, "radio").is_err());
    }

    #[test]
    fn limits_input() {
        let board = Workbench::from_preset(&PRESETS[0]);
        let mut selected = None;
        assert!(route(&board, &mut selected, &"A".repeat(MAX_INPUT + 1)).is_err());
        assert!(route(&board, &mut selected, "run status\u{0000}").is_err());
    }
}
