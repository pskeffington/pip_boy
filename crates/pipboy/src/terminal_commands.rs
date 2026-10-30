//! Selected-tool command terminal backed by the shared typed command catalog.
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
                let query = text.unwrap_or("");
                for item in sources()?.into_iter().filter(|item| item.matches(query)) {
                    println!("{} [{}]: {}", item.project, item.license, item.summary);
                }
            }
            HandlerId::BytesEncode => println!("{}", encode_hex(text.unwrap_or(""))?),
            HandlerId::HexInspect => print!("{}", inspect_hex(hex.unwrap_or(""))?),
            HandlerId::HexCrc32 => println!("{}", crc32_hex(hex.unwrap_or(""))?),
            HandlerId::NetworkList => {
                for op in network_suite::OPERATIONS.iter().filter(|op| !op.active) {
                    println!("{} / {}", op.id, op.name);
                }
            }
            handler => {
                let (verb, needs_extra) = match handler {
                    HandlerId::NetworkCidr4 => ("cidr", false),
                    HandlerId::NetworkCidr6 => ("cidr6", false),
                    HandlerId::NetworkSplit4 => ("split4", true),
                    HandlerId::NetworkRoute => ("route", true),
                    HandlerId::NetworkAggregate4 => ("aggregate4", false),
                    HandlerId::NetworkOverlap4 => ("overlap4", true),
                    HandlerId::NetworkMac => ("mac", false),
                    HandlerId::NetworkVlan => ("vlan", false),
                    HandlerId::NetworkMtu => ("mtu", false),
                    _ => return Err("unregistered native handler".into()),
                };
                let input = input.ok_or("typed network input is missing")?;
                let extra = if needs_extra { extra } else { None };
                if needs_extra && extra.is_none() {
                    return Err("typed network extra argument is missing".into());
                }
                println!("{}", network_suite::run_offline(verb, input, extra)?);
            }
        }
        Ok(())
    }
}

fn route<'a>(
    board: &'a Workbench,
    selected: &mut Option<&'a str>,
    input: &str,
) -> Result<bool, Box<dyn Error>> {
    let pinned = pinned_tools(board);
    let action = command_catalog::parse_input(input, *selected, &pinned)?;
    match action {
        TerminalAction::Empty => Ok(true),
        TerminalAction::Quit => Ok(false),
        TerminalAction::ListTools(tools) => {
            println!("Pinned tools with registered terminal handlers:");
            for id in tools {
                println!(
                    "  {id}{}",
                    if Some(id) == *selected {
                        " [selected]"
                    } else {
                        ""
                    }
                );
            }
            println!("Other pinned tools are not executable in this terminal.");
            Ok(true)
        }
        TerminalAction::Help { tool_id } => {
            display_help(tool_id, &pinned);
            Ok(true)
        }
        TerminalAction::SelectTool { tool_id } => {
            *selected = Some(tool_id);
            println!("SELECTED {tool_id}; no action started");
            Ok(true)
        }
        TerminalAction::Execute(request) => {
            let descriptor = command_catalog::COMMANDS
                .iter()
                .find(|item| item.handler == request.handler)
                .ok_or("command descriptor disappeared")?;
            println!("SOURCE {}", descriptor.source);
            let result =
                command_catalog::dispatch(TerminalAction::Execute(request), &mut OfflineExecutor)?;
            if !matches!(result, DispatchResult::Executed(())) {
                return Err("typed command was not dispatched".into());
            }
            Ok(true)
        }
    }
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    if !args.is_empty() {
        return Err("usage: pipboy terminal".into());
    }
    let board = WorkbenchStore::from_environment()?.load()?;
    let initial = pinned_tools(&board)
        .into_iter()
        .find(|id| command_catalog::registered_tools().contains(id));
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
    fn tool_selection_uses_shared_parser_and_never_dispatches() {
        let board = Workbench::from_preset(&PRESETS[0]);
        let mut selected = None;
        assert!(route(&board, &mut selected, "select host").unwrap());
        assert_eq!(selected, Some("host"));
        assert!(route(&board, &mut selected, "help").unwrap());
    }

    #[test]
    fn refuses_unpinned_and_unsupported_tools_and_shell_input() {
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
        assert!(route(
            &board,
            &mut selected,
            &"A".repeat(command_catalog::MAX_COMMAND_BYTES + 1)
        )
        .is_err());
        assert!(route(&board, &mut selected, "run status\u{0000}").is_err());
    }
}
