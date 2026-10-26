//! Explicit Messenger and PACE HTTP routes for the loopback-only dashboard.
//!
//! The parent dashboard validates Host/Origin and JSON request bounds before
//! dispatch. Persistent envelope/journal validation stays in pipboy-core;
//! explicit pinned OpenSSH delivery stays in crate::peer. Opening LINK or
//! reading this module must never initiate a network connection.
use super::{parse_json_body, positive_integer, str_field, Request, Response};
use crate::peer;
use pipboy_core::message_pace::{PaceStore, Tier};
use pipboy_core::messenger::{valid_text, Direction, MessageStore};
use pipboy_core::peers::PeerStore;
use pipboy_core::workbench::WorkbenchStore;
use serde_json::{json, Value};

fn messenger_store(store: &WorkbenchStore) -> MessageStore {
    let root = store
        .path()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    MessageStore::new(root.to_path_buf())
}

fn messenger_peers(store: &WorkbenchStore) -> PeerStore {
    let root = store
        .path()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    PeerStore::new(root.to_path_buf())
}

fn messenger_pace(store: &WorkbenchStore) -> PaceStore {
    PaceStore::new(
        store
            .path()
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .into(),
    )
}

fn message_tier(input: &Value) -> Result<Tier, Response> {
    match input.get("tier") {
        None => Ok(Tier::Primary),
        Some(Value::String(value)) => {
            Tier::parse(value).map_err(|_| Response::error(422, "Unknown PACE tier"))
        }
        _ => Err(Response::error(422, "PACE tier must be a string")),
    }
}

pub(super) fn messages_read(store: &WorkbenchStore) -> Result<Response, Response> {
    let peers = messenger_peers(store)
        .load()
        .map_err(|_| Response::error(409, "Messenger pairing configuration unavailable"))?;
    let book = messenger_store(store)
        .load()
        .map_err(|_| Response::error(409, "Messenger journal unavailable; original preserved"))?;
    let aliases: Vec<&str> = peers.peers.iter().map(|peer| peer.alias.as_str()).collect();
    let mut conversations: Vec<String> = aliases.iter().map(|id| (*id).to_owned()).collect();
    for message in &book.messages {
        if !conversations.contains(&message.peer) {
            conversations.push(message.peer.clone());
        }
    }
    conversations.sort();
    let pace = messenger_pace(store);
    let plans = conversations
        .iter()
        .map(|alias| pace.view(alias, aliases.contains(&alias.as_str())))
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|_| Response::error(409, "PACE configuration unavailable; original preserved"))?;
    let storage = messenger_store(store)
        .storage_mode()
        .map_err(|_| Response::error(409, "Messenger encryption key unavailable"))?;
    Ok(Response::json(
        200,
        &json!({
            "schema_version": 1,
            "device_id": peers.device_id,
            "ready": peers.device_id.is_some() && !aliases.is_empty(),
            "peers": aliases,
            "conversations": conversations,
            "messages": book.messages,
            "transport": "pinned_openssh_explicit",
            "storage": storage,
            "pace": plans,
            "e2ee": false,
            "automatic_connections": false
        }),
    ))
}

pub(super) fn messages_send(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let input = parse_json_body(req, &["alias", "body", "confirm", "tier"])?;
    if input.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err(Response::error(
            422,
            "Explicit Messenger send confirmation required",
        ));
    }
    let alias = str_field(&input, "alias")?;
    let body = str_field(&input, "body")?;
    if !valid_text(body) {
        return Err(Response::error(
            422,
            "Message must be 1-1024 UTF-8 bytes without control characters",
        ));
    }
    let journal = messenger_store(store);
    let peers = messenger_peers(store);
    let tier = message_tier(&input)?;
    let pace = messenger_pace(store);
    let (message, stored) =
        peer::queue_message_pace_from_store(&journal, &peers, Some(&pace), alias, body, tier)
            .map_err(|_| {
                Response::error(409, "Messenger device not paired or journal unavailable")
            })?;
    Ok(Response::json(
        200,
        &json!({
            "message": message,
            "stored_on_receiver": stored,
            "held_locally": tier == Tier::Emergency,
            "pace_tier": tier,
            "transport": if tier == Tier::Emergency { "local_hold" } else { "pinned_openssh_explicit" },
            "read_by_human": false
        }),
    ))
}

pub(super) fn messages_retry(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let input = parse_json_body(req, &["id", "confirm", "tier"])?;
    if input.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err(Response::error(
            422,
            "Explicit Messenger retry confirmation required",
        ));
    }
    let id = positive_integer(&input, "id")?;
    let journal = messenger_store(store);
    let entry = journal
        .load()
        .map_err(|_| Response::error(409, "Messenger journal unavailable"))?
        .messages
        .into_iter()
        .find(|message| message.id == id && message.direction == Direction::Outgoing)
        .ok_or_else(|| Response::error(404, "Outgoing message not found"))?;
    let peers = messenger_peers(store);
    let tier = message_tier(&input)?;
    let message = peer::send_message_pace_from_store(
        &journal,
        &peers,
        &messenger_pace(store),
        id,
        &entry.peer,
        tier,
    )
    .map_err(|_| Response::error(409, "Messenger retry failed; queued message preserved"))?;
    Ok(Response::json(
        200,
        &json!({
            "stored_on_receiver": message.delivery == pipboy_core::messenger::Delivery::Stored,
            "held_locally": tier == Tier::Emergency, "pace_tier": tier,
            "message": message, "read_by_human": false
        }),
    ))
}
