//! Explicit aid actions share the parent Host/Origin gate and pinned SSH path.
use super::{parse_json_body, positive_integer, str_field, Request, Response};
use crate::peer;
use pipboy_core::aid::{AidStore, Beacon, Config, Packet};
use pipboy_core::message_pace::{PaceStore, Tier};
use pipboy_core::messenger::{Delivery, Direction, Message, MessageStore};
use pipboy_core::peers::PeerStore;
use pipboy_core::workbench::WorkbenchStore;
use serde_json::{json, Value};
use std::path::PathBuf;

fn root(store: &WorkbenchStore) -> PathBuf {
    store
        .path()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .into()
}
fn unavailable() -> Response {
    Response::error(
        409,
        "Aid state or journal unavailable; saved files preserved",
    )
}
fn medical_allowed(store: &WorkbenchStore) -> Result<(), Response> {
    let board = store.load().map_err(|_| unavailable())?;
    if board.slots.iter().any(|slot| slot.tool == "medical") {
        Ok(())
    } else {
        Err(Response::error(
            403,
            "Load Medic or Doctor, or pin MEDICAL COMMS, first",
        ))
    }
}
fn confirmed(req: &Request, fields: &[&str]) -> Result<Value, Response> {
    let input = parse_json_body(req, fields)?;
    if input.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err(Response::error(
            422,
            "Explicit aid action confirmation required",
        ));
    }
    Ok(input)
}
fn dispatch_guard(aid: &AidStore) -> Result<std::fs::File, Response> {
    aid.dispatch_lock()
        .map_err(|_| Response::error(409, "Another aid action is active; refresh and retry"))
}
fn deliver(
    store: &WorkbenchStore,
    alias: &str,
    packet: &Packet,
    tier: Tier,
) -> Result<Message, Response> {
    if alias.is_empty() {
        return Err(Response::error(422, "No paired destination configured"));
    }
    let body = packet.encode().map_err(|_| {
        Response::error(
            422,
            "Invalid aid packet; each nine-line field is 1-48 UTF-8 bytes",
        )
    })?;
    let journal = MessageStore::new(root(store));
    let peers = PeerStore::new(root(store));
    let pace = PaceStore::new(root(store));
    // Reuse the durable envelope after an interruption; no automatic resend.
    let previous = journal
        .load()
        .map_err(|_| unavailable())?
        .messages
        .into_iter()
        .rev()
        .find(|m| m.direction == Direction::Outgoing && m.peer == alias && m.body == body);
    if let Some(previous) = previous.filter(|_| !matches!(packet, Packet::NineLine { .. })) {
        if previous.delivery == Delivery::Stored {
            return Ok(previous);
        }
        let _ =
            peer::send_message_pace_from_store(&journal, &peers, &pace, previous.id, alias, tier);
        return journal
            .outgoing(previous.id, alias)
            .map_err(|_| unavailable());
    }
    peer::queue_message_pace_from_store(&journal, &peers, Some(&pace), alias, &body, tier)
        .map(|(message, _)| message)
        .map_err(|_| {
            Response::error(
                409,
                "Not queued: check pairing, selected PACE route and journal capacity",
            )
        })
}
fn beacon_delivery(store: &WorkbenchStore, beacon: &Beacon) -> Value {
    if beacon.peer.is_empty() {
        return json!({"status":"local_only", "stored_on_receiver":false, "human_acknowledged":false});
    }
    match deliver(store, &beacon.peer, &beacon.packet(), beacon.tier) {
        Ok(message) => json!({"status":message.delivery, "message_id":message.id,
            "stored_on_receiver":message.delivery == Delivery::Stored, "human_acknowledged":false}),
        Err(_) => {
            json!({"status":"unconfirmed", "stored_on_receiver":false, "human_acknowledged":false})
        }
    }
}

pub(super) fn read(store: &WorkbenchStore) -> Result<Response, Response> {
    let aid = AidStore::new(root(store));
    let state = aid.load().map_err(|_| unavailable())?;
    let peers = PeerStore::new(root(store)).load().ok();
    let journal = MessageStore::new(root(store));
    let book = journal.load();
    let journal_available = book.is_ok();
    let messages = book.map(|book| book.messages).unwrap_or_default();
    let records: Vec<Value> =
        messages
            .iter()
            .rev()
            .filter_map(|message| {
                Packet::decode(&message.body).ok().map(|packet| json!({
            "id":message.id, "peer":message.peer, "direction":message.direction,
            "delivery":message.delivery, "created_at":message.created_at, "packet":packet
        }))
            })
            .collect();
    let acknowledged = state.beacon.as_ref().is_some_and(|beacon| messages.iter().any(|message| {
        message.direction == Direction::Incoming && message.peer == beacon.peer
            && matches!(Packet::decode(&message.body), Ok(Packet::Ack { incident }) if incident == beacon.id)
    }));
    let delivery = state.beacon.as_ref().and_then(|beacon| {
        let body = beacon.packet().encode().ok()?;
        messages
            .iter()
            .rev()
            .find(|message| {
                message.direction == Direction::Outgoing
                    && message.peer == beacon.peer
                    && message.body == body
            })
            .map(|message| message.delivery)
    });
    Ok(Response::json(
        200,
        &json!({"state":state, "peers":peers.as_ref().map(|p| p.peers.iter().map(|p| &p.alias).collect::<Vec<_>>()).unwrap_or_default(),
        "records":records, "human_acknowledged":acknowledged, "beacon_delivery":delivery,
        "storage":journal.storage_mode().unwrap_or("unavailable"), "journal_available":journal_available,
        "medical_allowed":medical_allowed(store).is_ok(), "automatic_transmission":false,
        "location_source":"manual_snapshot_or_unknown", "physical_beacon":false}),
    ))
}

pub(super) fn configure(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let input = confirmed(req, &["config", "confirm"])?;
    let config: Config =
        serde_json::from_value(input.get("config").cloned().unwrap_or(Value::Null))
            .map_err(|_| Response::error(422, "Invalid aid configuration"))?;
    let peers = PeerStore::new(root(store))
        .load()
        .map_err(|_| unavailable())?;
    for alias in [&config.doctor_peer, &config.emergency_peer] {
        if !alias.is_empty() && !peers.peers.iter().any(|p| &p.alias == alias) {
            return Err(Response::error(
                422,
                "Choose an explicitly paired destination",
            ));
        }
    }
    let aid = AidStore::new(root(store));
    let _guard = dispatch_guard(&aid)?;
    aid.configure(config)
        .map_err(|_| Response::error(422, "Aid configuration refused; original preserved"))?;
    read(store)
}

pub(super) fn beacon_action(
    req: &Request,
    store: &WorkbenchStore,
    action: &str,
) -> Result<Response, Response> {
    let input = confirmed(req, &["id", "confirm"])?;
    let aid = AidStore::new(root(store));
    let _guard = dispatch_guard(&aid)?;
    // Commit activation/cancellation before attempting delivery. Failure cannot
    // erase the latch or imply the remote device has learned of cancellation.
    let state = match action {
        "activate" => aid.activate().map_err(|_| unavailable())?,
        "cancel" => aid
            .cancel(positive_integer(&input, "id")?)
            .map_err(|_| Response::error(409, "Incident changed; refresh before cancelling"))?,
        _ => aid.load().map_err(|_| unavailable())?,
    };
    let beacon = state
        .beacon
        .ok_or_else(|| Response::error(422, "No emergency incident"))?;
    if action == "transmit" && positive_integer(&input, "id")? != beacon.id {
        return Err(Response::error(
            409,
            "Incident changed; refresh before transmitting",
        ));
    }
    let delivery = beacon_delivery(store, &beacon);
    Ok(Response::json(
        200,
        &json!({"beacon":beacon, "delivery":delivery}),
    ))
}

pub(super) fn nine_line(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    medical_allowed(store)?;
    let input = confirmed(req, &["packet", "confirm"])?;
    let packet: Packet =
        serde_json::from_value(input.get("packet").cloned().unwrap_or(Value::Null))
            .map_err(|_| Response::error(422, "A complete typed nine-line report is required"))?;
    if !matches!(packet, Packet::NineLine { .. }) {
        return Err(Response::error(422, "Expected nine-line report"));
    }
    let aid = AidStore::new(root(store));
    let _guard = dispatch_guard(&aid)?;
    let state = aid.load().map_err(|_| unavailable())?;
    let message = deliver(store, &state.config.doctor_peer, &packet, state.config.tier)?;
    Ok(Response::json(
        200,
        &json!({"message":message, "stored_on_receiver":message.delivery == Delivery::Stored,
        "read_by_doctor":false}),
    ))
}

pub(super) fn acknowledge(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let input = confirmed(req, &["message_id", "confirm"])?;
    let id = positive_integer(&input, "message_id")?;
    let aid = AidStore::new(root(store));
    let _guard = dispatch_guard(&aid)?;
    let messages = MessageStore::new(root(store))
        .load()
        .map_err(|_| unavailable())?
        .messages;
    let received = messages
        .iter()
        .find(|m| m.id == id && m.direction == Direction::Incoming)
        .ok_or_else(|| Response::error(404, "Received alarm not found"))?;
    let Packet::Alarm { incident, .. } = Packet::decode(&received.body)
        .map_err(|_| Response::error(422, "Not an emergency alarm"))?
    else {
        return Err(Response::error(
            422,
            "Only incoming alarms can be acknowledged",
        ));
    };
    if messages.iter().any(|m| m.direction == Direction::Incoming && m.peer == received.peer
        && matches!(Packet::decode(&m.body), Ok(Packet::Cancel { incident: cancelled }) if cancelled == incident)) {
        return Err(Response::error(409, "This alarm has been cancelled"));
    }
    let tier = aid.load().map_err(|_| unavailable())?.config.tier;
    let message = deliver(store, &received.peer, &Packet::Ack { incident }, tier)?;
    Ok(Response::json(
        200,
        &json!({"message":message, "stored_on_receiver":message.delivery == Delivery::Stored,
        "meaning":"Operator acknowledged alert; response or rescue not promised"}),
    ))
}
