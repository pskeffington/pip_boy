//! Selected-tool terminal backed by the shared typed command catalog.
//! No shell, dynamic module loading, network activity or implicit grants.

use pipboy_core::catalog::sources;
use pipboy_core::command_catalog::{
    self, ArgumentSchema, CommandExecutor, CommandRequest, DispatchResult, HandlerId,
    TerminalAction, TypedArguments,
};
use pipboy_core::network_suite;
use pipboy_core::workbench::{crc32_hex, encode_hex, inspect_hex, Workbench, WorkbenchStore};
use std::error::Error;
use std::io::{self, Write};

fn pinned_tools(board: &Workbench) -> Vec<&str> {
    board.slots.iter().map(|slot| slot.tool.as_str()).collect()
}

fn display_help(tool_id: Option<&str>, pinned: &[&str]) {
    println!("tools | select TOOL | help | run COMMAND | quit");
    if let Some(tool_id) = tool_id {
        if !pinned.contains(&tool_id) {
            println!("{tool_id}: not pinned");
            return;
        }
        let commands = command_catalog::help_for(tool_id);
        if commands.is_empty() {
            println!("{tool_id}: no terminal handlers are registered");
        }
        for command in commands {
            let args = match command.arguments {
                ArgumentSchema::None => "".to_owned(),
                ArgumentSchema::OptionalText => " [TEXT]".to_owned(),
                ArgumentSchema::Text => " TEXT".to_owned(),
                ArgumentSchema::HexBytes => " HEX".to_owned(),
                ArgumentSchema::NetworkInput => " INPUT".to_owned(),
                ArgumentSchema::NetworkInputAndExtra => " INPUT EXTRA".to_owned(),
            };
            println!("  {}{} — source: {}", command.verb, args, command.source);
        }
    } else {
        println!("Only registered offline Rust handlers are callable.");
    }
}

struct OfflineExecutor;

impl CommandExecutor for OfflineExecutor {
    type Output = ();
    type Error = Box<dyn Error>;

    fn execute(&mut self, request: CommandRequest) -> Result<Self::Output, Self::Error> {
        let text = match &request.arguments {
            TypedArguments::Text { value } => Some(value.as_str()),
            _ => None,
        };
        let (input, extra) = match &request.arguments {
            TypedArguments::Network { input, extra, .. } => {
                (Some(input.as_str()), extra.as_deref())
            }
            _ => (None, None),
        };
        let hex = match &request.arguments {
            TypedArguments::Hex { value } => Some(value.as_str()),
            _ => None,
        };

        match request.handler {
            HandlerId::HostStatus => {
                let snapshot = crate::status::collect()?;
                println!(
                    "SYSTEM STAT / {} {} | RAM {} / {} MiB | UPTIME {} s",
                    snapshot.os,
                    snapshot.arch,
                    snapshot.memory_used_mib,
                    snapshot.memory_total_mib,
                    snapshot.uptime_s
                );
            }
            HandlerId::HostDiagnostics => crate::diagnostics::run(vec!["host".into()])?,
            HandlerId::SourceSearch => {
                let query = text.unwrap_or_default();
                for item in sources()?.into_iter().filter(|item| item.matches(query)) {
                    println!("{} [{}]: {}", item.project, item.license, item.summary);
                }
            }
            HandlerId::BytesEncode => println!("{}", encode_hex(text.ok_or("missing text")?)?),
            HandlerId::HexInspect => print!("{}", inspect_hex(hex.ok_or("missing hex")?)?),
            HandlerId::HexCrc32 => println!("{}", crc32_hex(hex.ok_or("missing hex")?)?),
            HandlerId::NetworkList => {
                for op in network_suite::OPERATIONS.iter().filter(|op| !op.active) {
                    println!("{} / {}", op.id, op.name);
                }
            }
            handler => {
                let (mode, first, second) = match handler {
                    HandlerId::NetworkCidr4 => ("cidr", input, None),
                    HandlerId::NetworkCidr6 => ("cidr6", input, None),
                    HandlerId::NetworkSplit4 => ("split4", input, extra),
                    HandlerId::NetworkRoute => ("route", input, extra),
                    HandlerId::NetworkAggregate4 => ("aggregate4", input, None),
                    HandlerId::NetworkOverlap4 => ("overlap4", input, extra),
                    HandlerId::NetworkMac => ("mac", input, None),
                    HandlerId::NetworkVlan => ("vlan", input, None),
                    HandlerId::NetworkMtu => ("mtu", input, None),
                    _ => return Err("unavailable handler".into()),
                };
                println!(
                    "{}",
                    network_suite::run_offline(mode, first.ok_or("missing input")?, second)?
                );
            }
        }
        Ok(())
    }
}

fn run_action(action: TerminalAction) -> Result<(), Box<dyn Error>> {
    match command_catalog::dispatch(action, &mut OfflineExecutor)? {
        DispatchResult::Control(TerminalAction::ListTools(tools)) => {
            println!("Available registered tools: {}", tools.join(", "));
        }
        DispatchResult::Control(TerminalAction::Help { tool_id }) => {
            display_help(tool_id, command_catalog::registered_tools());
        }
        DispatchResult::Control(TerminalAction::SelectTool { tool_id }) => {
            println!("SELECTED {tool_id}; available:");
            display_help(Some(tool_id), command_catalog::registered_tools());
        }
        DispatchResult::Control(TerminalAction::Empty) => {}
        DispatchResult::Control(TerminalAction::Quit) => {}
        DispatchResult::Executed(()) => {}
        DispatchResult::Control(TerminalAction::Execute(_)) => {
            unreachable!("execute is dispatched")
        }
    }
    Ok(())
}

fn route(
    input: &str,
    selected: &mut Option<&'static str>,
    pinned: &[&str],
) -> Result<bool, Box<dyn Error>> {
    if input.trim() == "q" {
        return Ok(false);
    }
    // Preserve the legacy `run COMMAND` form while letting the shared parser
    // perform the actual pin, schema, and handler resolution.
    let normalized = if let Some(command) = input.strip_prefix("run ") {
        command
    } else {
        input
    };
    let action = command_catalog::parse_input(normalized, *selected, pinned)?;
    match &action {
        TerminalAction::SelectTool { tool_id } => *selected = Some(tool_id),
        TerminalAction::Quit => return Ok(false),
        _ => {}
    }
    run_action(action)?;
    Ok(true)
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("usage: pipboy terminal".into());
    }
    let board = WorkbenchStore::from_environment()?.load()?;
    let pinned = pinned_tools(&board);
    let initial = command_catalog::registered_tools()
        .iter()
        .copied()
        .find(|tool| pinned.contains(tool));
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
        match route(input, &mut selected, &pinned) {
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
    fn tool_selection_is_catalog_control_and_never_executes() {
        let board = Workbench::from_preset(&PRESETS[0]);
        let pinned = pinned_tools(&board);
        let mut selected = None;
        assert!(route("select host", &mut selected, &pinned).unwrap());
        assert_eq!(selected, Some("host"));
    }

    #[test]
    fn catalog_denies_unpinned_unsupported_and_active_operations() {
        let board = Workbench::from_preset(&PRESETS[0]);
        let pinned = pinned_tools(&board);
        let mut selected = None;
        assert!(route("select net-lab", &mut selected, &pinned).is_err());
        assert!(route("select radio", &mut selected, &pinned).is_err());
        assert!(route("net scan local --confirm", &mut selected, &pinned).is_err());
        assert!(route("sh -c id", &mut selected, &pinned).is_err());
        assert!(route("rb:host> run status", &mut selected, &pinned).is_err());
    }

    #[test]
    fn typed_catalog_requests_execute_and_legacy_run_prefix_is_supported() {
        let board = Workbench::from_preset(&PRESETS[2]);
        let pinned = pinned_tools(&board);
        let mut selected = None;
        assert!(route("net-lab", &mut selected, &pinned).unwrap());
        assert!(route("run list", &mut selected, &pinned).unwrap());
        assert!(route("host", &mut selected, &pinned).unwrap());
        assert_eq!(selected, Some("host"));
        assert!(route("run status", &mut selected, &pinned).unwrap());
        assert!(route("exit", &mut selected, &pinned).is_ok_and(|keep| !keep));
    }

    #[test]
    fn oversized_or_control_input_is_denied_by_shared_parser() {
        let board = Workbench::from_preset(&PRESETS[0]);
        let pinned = pinned_tools(&board);
        let mut selected = None;
        assert!(route(
            &"A".repeat(command_catalog::MAX_COMMAND_BYTES + 1),
            &mut selected,
            &pinned
        )
        .is_err());
        assert!(route("run status\0", &mut selected, &pinned).is_err());
    }
}
