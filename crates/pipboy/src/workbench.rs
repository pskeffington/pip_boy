//! Explicit local workbench management. No pin operation executes a tool.
use pipboy_core::workbench::{encode_hex, inspect_hex, tool, WorkbenchStore, PALETTE};
use std::error::Error;

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let usage = "usage: pipboy workbench <list [--json]|palette [--json]|add TOOL|remove ID|rename TITLE|clear --confirm|bytes <encode TEXT|inspect HEX>>";
    let mut args = args.into_iter();
    let command = args.next().ok_or(usage)?;
    match command.as_str() {
        "palette" => {
            let flags: Vec<_> = args.collect();
            match flags.as_slice() {
                [] => {
                    println!("Rust-Boy workbench palette (pinning never runs a tool)");
                    for entry in PALETTE {
                        println!(
                            "{:<12} {:<14} {:<24} {}",
                            entry.id,
                            entry.name,
                            entry.availability.label(),
                            entry.description
                        );
                    }
                    Ok(())
                }
                [flag] if flag == "--json" => {
                    println!("{}", serde_json::to_string_pretty(PALETTE)?);
                    Ok(())
                }
                _ => Err(usage.into()),
            }
        }
        "list" | "show" => {
            let flags: Vec<_> = args.collect();
            if !flags.is_empty() && flags.as_slice() != ["--json"] {
                return Err(usage.into());
            }
            let board = WorkbenchStore::from_environment()?.load()?;
            if flags.as_slice() == ["--json"] {
                println!("{}", serde_json::to_string_pretty(&board)?);
            } else {
                println!("{} | {} pinned tools", board.title, board.slots.len());
                if board.slots.is_empty() {
                    println!("EMPTY / Press A in the WORKBENCH panel, or use: pipboy workbench palette");
                }
                for slot in &board.slots {
                    if let Some(entry) = tool(&slot.tool) {
                        println!("#{} {:<14} {}", slot.id, entry.name, entry.availability.label());
                        println!("   {}", entry.command);
                    }
                }
                println!("INFO: pinning changes only your local workbench layout; it grants no device/network access.");
            }
            Ok(())
        }
        "add" => {
            let key = args.next().ok_or(usage)?;
            if args.next().is_some() {
                return Err(usage.into());
            }
            let slot = WorkbenchStore::from_environment()?.add(&key)?;
            println!("PINNED #{} {}", slot.id, slot.tool);
            println!("INFO: no tool was executed and no hardware permission was granted");
            Ok(())
        }
        "remove" => {
            let id = args.next().ok_or(usage)?.parse::<u64>()?;
            if id == 0 || args.next().is_some() {
                return Err(usage.into());
            }
            let removed = WorkbenchStore::from_environment()?.remove(id)?;
            println!(
                "{} workbench slot #{}",
                if removed { "REMOVED" } else { "NOT FOUND" },
                id
            );
            Ok(())
        }
        "rename" => {
            let title = args.collect::<Vec<_>>().join(" ");
            if title.is_empty() {
                return Err(usage.into());
            }
            let updated = WorkbenchStore::from_environment()?.rename(&title)?;
            println!(
                "{} workbench title: {}",
                if updated { "UPDATED" } else { "UNCHANGED" },
                title
            );
            Ok(())
        }
        "clear" => {
            if args.collect::<Vec<_>>() != ["--confirm"] {
                return Err("usage: pipboy workbench clear --confirm".into());
            }
            let cleared = WorkbenchStore::from_environment()?.clear()?;
            println!(
                "{} workbench pins; saved notes, permissions and game data were not changed",
                if cleared { "CLEARED" } else { "UNCHANGED" }
            );
            Ok(())
        }
        "bytes" => {
            let mode = args.next().ok_or(usage)?;
            let input = args.collect::<Vec<_>>().join(" ");
            match mode.as_str() {
                "inspect" => print!("{}", inspect_hex(&input)?),
                "encode" => println!("{}", encode_hex(&input)?),
                _ => return Err(usage.into()),
            };
            Ok(())
        }
        _ => Err(usage.into()),
    }
}
