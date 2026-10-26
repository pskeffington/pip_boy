//! Operator-gated Task Control and explicit peer handoff routes.
//!
//! This module owns only the HTTP adapter. Validated task persistence and
//! revision control stay in pipboy-core; native SSH delivery stays in peer.rs.
//! All calls pass through the parent dashboard's same-origin gate.
use super::{parse_json_body, positive_integer, str_field, Request, Response};
use crate::peer;
use pipboy_core::peers::PeerStore;
use pipboy_core::tasks::{NewTask, Status, TaskStore};
use pipboy_core::workbench::WorkbenchStore;
use serde_json::{json, Value};
use std::io;

pub(super) fn task_store(workbench: &WorkbenchStore) -> TaskStore {
    let root = workbench
        .path()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    TaskStore::new(root.to_path_buf())
}

fn manager_role(store: &WorkbenchStore) -> Result<(String, bool), Response> {
    let board = store
        .load()
        .map_err(|_| Response::error(409, "Workbench unavailable; original file preserved"))?;
    let manager = board.slots.iter().any(|slot| slot.tool == "mission-board");
    Ok((board.profile, manager))
}

pub(super) fn tasks_read(store: &WorkbenchStore) -> Result<Response, Response> {
    let (role, manager) = manager_role(store)?;
    let task_store = task_store(store);
    let mut board = task_store
        .load()
        .map_err(|_| Response::error(409, "Task board unavailable; original file preserved"))?;
    if !manager {
        board.tasks.retain(|task| task.role == role);
    }
    Ok(Response::json(
        200,
        &json!({
            "board": board,
            "manager": manager,
            "role": role,
            "delivery": "manual_handoff_only"
        }),
    ))
}

pub(super) fn tasks_create(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let (_, manager) = manager_role(store)?;
    if !manager {
        return Err(Response::error(
            403,
            "Pin Task Control or load Overseer to create assignments",
        ));
    }
    let body = parse_json_body(
        req,
        &[
            "title", "details", "assignee", "role", "contact", "priority", "due",
        ],
    )?;
    let task: NewTask = serde_json::from_value(body)
        .map_err(|_| Response::error(422, "Invalid new task fields"))?;
    let result = task_store(store).create(task).map_err(|_| {
        Response::error(422, "Task validation or local storage failed; no task sent")
    })?;
    Ok(Response::json(200, &json!({"task": result, "sent": false})))
}

pub(super) fn tasks_change(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let body = parse_json_body(
        req,
        &[
            "id",
            "revision",
            "operation",
            "status",
            "note",
            "assignee",
            "role",
            "contact",
        ],
    )?;
    let id = positive_integer(&body, "id")?;
    let revision = positive_integer(&body, "revision")?;
    let operation = str_field(&body, "operation")?;
    let (role, manager) = manager_role(store)?;
    let tasks = task_store(store);
    let existing = tasks
        .task(id)
        .map_err(|_| Response::error(404, "Task not found"))?;
    if !manager && existing.role != role {
        return Err(Response::error(
            403,
            "Only the assigned role or Task Control can update this task",
        ));
    }
    let changed = match operation {
        "status" => {
            let status: Status = serde_json::from_value(
                body.get("status")
                    .cloned()
                    .ok_or_else(|| Response::error(422, "Missing task status"))?,
            )
            .map_err(|_| Response::error(422, "Invalid task status"))?;
            let note = body.get("note").and_then(Value::as_str).unwrap_or("");
            tasks.set_status(id, revision, status, note)
        }
        "reassign" if manager => {
            let person = str_field(&body, "assignee")?;
            let target = str_field(&body, "role")?;
            let contact = body.get("contact").and_then(Value::as_str);
            tasks.reassign(id, revision, person, target, contact)
        }
        "sent" if manager => tasks.mark_sent(id, revision),
        "acknowledge" => tasks.acknowledge(id, revision),
        _ => return Err(Response::error(403, "Operation restricted to Task Control")),
    };
    let task = changed.map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            Response::error(409, "Task was edited elsewhere; refresh the board")
        } else if error.kind() == io::ErrorKind::NotFound {
            Response::error(404, "Task not found")
        } else {
            Response::error(422, "Task update invalid or storage unavailable")
        }
    })?;
    Ok(Response::json(200, &json!({"task": task})))
}

pub(super) fn tasks_handoff(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let (_, manager) = manager_role(store)?;
    if !manager {
        return Err(Response::error(
            403,
            "Task handoff is an Overseer/Task Control operation",
        ));
    }
    let body = parse_json_body(req, &["id"])?;
    let id = positive_integer(&body, "id")?;
    let task = task_store(store)
        .task(id)
        .map_err(|_| Response::error(404, "Task not found"))?;
    Ok(Response::json(
        200,
        &json!({"text": task.handoff(), "sent": false, "transport": "operator_controlled"}),
    ))
}

pub(super) fn peers_read(store: &WorkbenchStore) -> Result<Response, Response> {
    let (_, manager) = manager_role(store)?;
    if !manager {
        return Err(Response::error(
            403,
            "Only Task Control can inspect paired destinations",
        ));
    }
    let root = store
        .path()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let peers = PeerStore::new(root.to_path_buf());
    let status = peer::safe_peers_from_store(&peers)
        .map_err(|_| Response::error(409, "Peer roster unavailable; original file preserved"))?;
    Ok(Response::json(200, &status))
}

pub(super) fn device_send(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let (_, manager) = manager_role(store)?;
    if !manager {
        return Err(Response::error(
            403,
            "Load Overseer or explicitly pin Task Control",
        ));
    }
    let body = parse_json_body(req, &["id", "revision", "alias", "confirm"])?;
    if body.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err(Response::error(
            422,
            "Explicit device-send confirmation required",
        ));
    }
    let id = positive_integer(&body, "id")?;
    let revision = positive_integer(&body, "revision")?;
    let alias = str_field(&body, "alias")?;
    let root = store
        .path()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let peers = PeerStore::new(root.to_path_buf());
    let tasks = TaskStore::new(root.to_path_buf());
    let result =
        peer::send_from_store(&tasks, &peers, id, revision, alias).map_err(|_| {
            Response::error(409,
            "Device delivery failed or task changed; verify SSH peer and reload before retrying")
        })?;
    Ok(Response::json(
        200,
        &json!({
            "task": result,
            "transport": "openssh",
            "stored_on_receiver": true,
            "human_acknowledged": false
        }),
    ))
}

pub(super) fn device_sync(req: &Request, store: &WorkbenchStore) -> Result<Response, Response> {
    let (_, manager) = manager_role(store)?;
    if !manager {
        return Err(Response::error(
            403,
            "Only Task Control can refresh peer receipts",
        ));
    }
    let body = parse_json_body(req, &["alias", "confirm"])?;
    if body.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err(Response::error(
            422,
            "Explicit receipt-sync confirmation required",
        ));
    }
    let alias = str_field(&body, "alias")?;
    let root = store
        .path()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let peers = PeerStore::new(root.to_path_buf());
    let tasks = TaskStore::new(root.to_path_buf());
    let changed = peer::sync_from_store(&tasks, &peers, alias).map_err(|_| {
        Response::error(
            409,
            "Could not verify or process remote progress; local records preserved",
        )
    })?;
    Ok(Response::json(
        200,
        &json!({"updated": changed, "source": "openssh_explicit_sync"}),
    ))
}
