//! Offline mission/task management for the Overseer and other field roles.
//!
//! Task assignments are local records. Handoff text can be shared explicitly by
//! an operator, but Rust-Boy never sends mail, authenticates recipients, or
//! claims remote acknowledgment. Future transports must be separately gated.
use crate::workbench::PRESETS;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA: u32 = 1;
const MAX_FILE: u64 = 512 * 1024;
const MAX_TASKS: usize = 256;
const MAX_EVENTS: usize = 32;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn checked_text(value: &str, max: usize, required: bool) -> bool {
    value.len() <= max
        && (required && !value.is_empty() || !required)
        && value.trim() == value
        && value.chars().all(|ch| !ch.is_control())
}

fn valid_contact(value: &str) -> bool {
    if !checked_text(value, 120, true) || !value.is_ascii() {
        return false;
    }
    let Some((name, domain)) = value.split_once('@') else {
        return false;
    };
    !name.is_empty()
        && !domain.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && value
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, b'.' | b'_' | b'+' | b'-' | b'@'))
}

fn valid_date(value: &str) -> bool {
    let chars = value.as_bytes();
    if chars.len() != 10
        || chars[4] != b'-'
        || chars[7] != b'-'
        || !chars
            .iter()
            .enumerate()
            .all(|(i, ch)| i == 4 || i == 7 || ch.is_ascii_digit())
    {
        return false;
    }
    let year = value[0..4].parse::<u32>().unwrap_or(0);
    let month = value[5..7].parse::<u32>().unwrap_or(0);
    let day = value[8..10].parse::<u32>().unwrap_or(0);
    if !(2000..=2199).contains(&year) || !(1..=12).contains(&month) {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=max_day).contains(&day)
}

fn valid_role(value: &str) -> bool {
    PRESETS.iter().any(|profile| profile.id == value)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Todo,
    InProgress,
    Blocked,
    Done,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub at: u64,
    pub kind: String,
    pub detail: String,
}

/// Origin is a correlation label, not standalone authentication. SSH access
/// and the receiving OS account define the current trust boundary.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RemoteOrigin {
    pub device: String,
    pub task_id: u64,
    pub source_revision: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeviceDelivery {
    pub peer: String,
    pub remote_task_id: u64,
    pub delivered_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RemoteProgress {
    pub status: Status,
    pub acknowledged: bool,
    pub observed_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RemoteReport {
    pub from_device: String,
    pub origin_task_id: u64,
    pub remote_task_id: u64,
    pub status: Status,
    pub acknowledged: bool,
}

pub fn valid_device_id(value: &str) -> bool {
    (2..=32).contains(&value.len())
        && value.is_ascii()
        && value
            .bytes()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteAssignment {
    pub schema_version: u32,
    pub from_device: String,
    pub origin_task_id: u64,
    pub origin_revision: u64,
    pub task: NewTask,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: u64,
    pub revision: u64,
    pub title: String,
    pub details: String,
    pub assignee: String,
    pub role: String,
    pub contact: Option<String>,
    pub priority: Priority,
    pub status: Status,
    pub due: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    /// Set only on explicit user confirmation that a handoff happened.
    /// It does not prove delivery to or receipt by a remote operator.
    pub sent_at: Option<u64>,
    /// Manually recorded acknowledgment; not an authenticated remote receipt.
    pub acknowledged_at: Option<u64>,
    /// This task was imported from another independently stored Rust-Boy.
    #[serde(default)]
    pub origin: Option<RemoteOrigin>,
    /// Authenticated SSH receiver-store receipt, not operator acknowledgment.
    #[serde(default)]
    pub delivery: Option<DeviceDelivery>,
    /// Last explicitly requested SSH progress synchronization.
    #[serde(default)]
    pub remote_progress: Option<RemoteProgress>,
    pub events: Vec<Event>,
}

impl Task {
    fn valid(&self) -> bool {
        self.id > 0
            && self.revision > 0
            && !self.events.is_empty()
            && self
                .acknowledged_at
                .is_none_or(|ack| self.sent_at.is_some_and(|sent| ack >= sent))
            && self.origin.as_ref().is_none_or(|origin| {
                valid_device_id(&origin.device) && origin.task_id > 0 && origin.source_revision > 0
            })
            && self.delivery.as_ref().is_none_or(|receipt| {
                valid_device_id(&receipt.peer)
                    && receipt.remote_task_id > 0
                    && self.sent_at.is_some()
            })
            && self.remote_progress.as_ref().is_none_or(|progress| {
                self.delivery.is_some() && progress.observed_at >= self.created_at
            })
            && checked_text(&self.title, 96, true)
            && checked_text(&self.details, 400, false)
            && checked_text(&self.assignee, 64, true)
            && valid_role(&self.role)
            && self.contact.as_deref().is_none_or(valid_contact)
            && self.due.as_deref().is_none_or(valid_date)
            && self.created_at <= self.updated_at
            && self.events.len() <= MAX_EVENTS
            && self.events.iter().all(|event| {
                checked_text(&event.kind, 32, true)
                    && checked_text(&event.detail, 128, false)
                    && event.at >= self.created_at
            })
    }

    pub fn handoff(&self) -> String {
        let due = self.due.as_deref().unwrap_or("Not set");
        let contact = self.contact.as_deref().unwrap_or("Address manually");
        format!(
            "RUST-BOY / ASSIGNMENT\n\
             TASK: RB-{:04}\n\
             TITLE: {}\n\
             ASSIGNED TO: {}\n\
             ROLE: {}\n\
             CONTACT: {}\n\
             PRIORITY: {:?}\n\
             DUE: {}\n\
             DETAILS: {}\n\n\
             STATUS: Awaiting operator update\n\
             Reply with the task reference, acknowledgment and current status.\n\
             This is a manually prepared handoff, not an authenticated transmission.",
            self.id,
            self.title,
            self.assignee,
            self.role,
            contact,
            self.priority,
            due,
            self.details
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewTask {
    pub title: String,
    #[serde(default)]
    pub details: String,
    pub assignee: String,
    pub role: String,
    #[serde(default)]
    pub contact: Option<String>,
    pub priority: Priority,
    #[serde(default)]
    pub due: Option<String>,
}

impl NewTask {
    fn valid(&self) -> bool {
        checked_text(&self.title, 96, true)
            && checked_text(&self.details, 400, false)
            && checked_text(&self.assignee, 64, true)
            && valid_role(&self.role)
            && self.contact.as_deref().is_none_or(valid_contact)
            && self.due.as_deref().is_none_or(valid_date)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskBoard {
    pub schema_version: u32,
    pub next_id: u64,
    pub tasks: Vec<Task>,
}

impl Default for TaskBoard {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA,
            next_id: 1,
            tasks: Vec::new(),
        }
    }
}

impl TaskBoard {
    fn validate(&self) -> io::Result<()> {
        if self.schema_version != SCHEMA || self.next_id == 0 || self.tasks.len() > MAX_TASKS {
            return Err(invalid(
                "Unknown or oversized task board; original file preserved",
            ));
        }
        let mut ids = HashSet::new();
        for task in &self.tasks {
            if !task.valid() || !ids.insert(task.id) || task.id >= self.next_id {
                return Err(invalid("Invalid task record; original file preserved"));
            }
        }
        Ok(())
    }

    fn read(source: impl Read) -> io::Result<Self> {
        let mut data = Vec::new();
        source.take(MAX_FILE + 1).read_to_end(&mut data)?;
        if data.len() as u64 > MAX_FILE {
            return Err(invalid("Task board exceeds maximum size"));
        }
        let board: Self = serde_json::from_slice(&data)
            .map_err(|_| invalid("Unrecognized task board; original file preserved"))?;
        board.validate()?;
        Ok(board)
    }
}

pub struct TaskStore {
    path: PathBuf,
}

impl TaskStore {
    pub fn new(root: PathBuf) -> Self {
        Self {
            path: root.join("tasks.json"),
        }
    }

    pub fn from_environment() -> io::Result<Self> {
        let root = match env::var_os("PIPBOY_HOME") {
            Some(value) if !value.is_empty() => PathBuf::from(value),
            _ => {
                let home = env::var_os("HOME")
                    .or_else(|| env::var_os("USERPROFILE"))
                    .ok_or_else(|| invalid("Home folder unavailable"))?;
                PathBuf::from(home).join(".pipboy")
            }
        };
        Ok(Self::new(root))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn directory(&self) -> &Path {
        self.path.parent().unwrap_or_else(|| Path::new("."))
    }

    fn writer_lock(&self) -> io::Result<fs::File> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(self.directory())?;
        }
        #[cfg(not(unix))]
        fs::create_dir_all(self.directory())?;
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(self.directory().join("tasks.lock"))?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|error| {
            if error.kind() == io::ErrorKind::WouldBlock {
                invalid("Another writer is using the task board")
            } else {
                error
            }
        })?;
        Ok(lock)
    }

    pub fn load(&self) -> io::Result<TaskBoard> {
        match fs::File::open(&self.path) {
            Ok(file) => TaskBoard::read(file),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(TaskBoard::default()),
            Err(error) => Err(error),
        }
    }

    fn save(&self, board: &TaskBoard) -> io::Result<()> {
        board.validate()?;
        let encoded = serde_json::to_vec_pretty(board)?;
        if encoded.len() as u64 > MAX_FILE {
            return Err(invalid("Task board storage limit reached"));
        }
        let mut file = tempfile::Builder::new()
            .prefix(".task-board-")
            .tempfile_in(self.directory())?;
        file.as_file_mut().write_all(&encoded)?;
        file.as_file().sync_all()?;
        file.persist(&self.path).map_err(|error| error.error)?;
        #[cfg(unix)]
        fs::File::open(self.directory())?.sync_all()?;
        Ok(())
    }

    pub fn create(&self, input: NewTask) -> io::Result<Task> {
        if !input.valid() {
            return Err(invalid("Invalid task fields, role, email or calendar date"));
        }
        let _lock = self.writer_lock()?;
        let mut board = self.load()?;
        if board.tasks.len() >= MAX_TASKS {
            return Err(invalid("Task board has reached its 256-task limit"));
        }
        let at = now();
        let id = board.next_id;
        board.next_id = id
            .checked_add(1)
            .ok_or_else(|| invalid("Task IDs exhausted"))?;
        let task = Task {
            id,
            revision: 1,
            title: input.title,
            details: input.details,
            assignee: input.assignee,
            role: input.role,
            contact: input.contact,
            priority: input.priority,
            status: Status::Todo,
            due: input.due,
            created_at: at,
            updated_at: at,
            sent_at: None,
            acknowledged_at: None,
            origin: None,
            delivery: None,
            remote_progress: None,
            events: vec![Event {
                at,
                kind: "created".into(),
                detail: "Assigned locally; handoff has not been sent".into(),
            }],
        };
        board.tasks.push(task.clone());
        self.save(&board)?;
        Ok(task)
    }

    pub fn task(&self, id: u64) -> io::Result<Task> {
        self.load()?
            .tasks
            .into_iter()
            .find(|task| task.id == id)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Task not found"))
    }

    fn change<F>(&self, id: u64, revision: u64, update: F) -> io::Result<Task>
    where
        F: FnOnce(&mut Task) -> io::Result<(&'static str, String)>,
    {
        let _lock = self.writer_lock()?;
        let mut board = self.load()?;
        let task = board
            .tasks
            .iter_mut()
            .find(|task| task.id == id)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Task not found"))?;
        if revision != task.revision {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Task changed since last refresh; reload before editing",
            ));
        }
        let (kind, detail) = update(task)?;
        let at = now();
        task.updated_at = at.max(task.updated_at);
        task.revision = task
            .revision
            .checked_add(1)
            .ok_or_else(|| invalid("Revision exhausted"))?;
        task.events.push(Event {
            at,
            kind: kind.into(),
            detail,
        });
        if task.events.len() > MAX_EVENTS {
            task.events.remove(0);
        }
        let result = task.clone();
        self.save(&board)?;
        Ok(result)
    }

    pub fn set_status(
        &self,
        id: u64,
        revision: u64,
        status: Status,
        note: &str,
    ) -> io::Result<Task> {
        if !checked_text(note, 110, false) {
            return Err(invalid(
                "Status note must be at most 110 printable characters",
            ));
        }
        self.change(id, revision, |task| {
            if task.status == status {
                return Err(invalid("Task is already in that status"));
            }
            task.status = status;
            Ok(("status", format!("{status:?}: {note}")))
        })
    }

    pub fn reassign(
        &self,
        id: u64,
        revision: u64,
        name: &str,
        role: &str,
        contact: Option<&str>,
    ) -> io::Result<Task> {
        if !checked_text(name, 64, true) || !valid_role(role) || !contact.is_none_or(valid_contact)
        {
            return Err(invalid("Invalid assignee, target job or recipient address"));
        }
        self.change(id, revision, |task| {
            task.assignee = name.to_owned();
            task.role = role.to_owned();
            if task.origin.is_some() {
                return Err(invalid("Incoming assignments cannot be reassigned locally"));
            }
            task.contact = contact.map(str::to_owned);
            task.delivery = None;
            task.remote_progress = None;
            task.sent_at = None;
            task.acknowledged_at = None;
            Ok((
                "reassigned",
                format!("Assigned to {name} / {role}; handoff reset"),
            ))
        })
    }

    /// Persist an assignment imported from a trusted SSH session. Redelivery of
    /// the *same* source and revision is idempotent, including after restarts.
    /// An edited upstream task must be reconciled explicitly, never overwrite
    /// the receiver's local status or acknowledgment.
    pub fn import_remote(&self, input: RemoteAssignment) -> io::Result<(Task, bool)> {
        if input.schema_version != SCHEMA
            || !valid_device_id(&input.from_device)
            || input.origin_task_id == 0
            || input.origin_revision == 0
            || !input.task.valid()
            || input.task.contact.is_some()
        {
            return Err(invalid("Invalid remote assignment envelope"));
        }
        let _lock = self.writer_lock()?;
        let mut board = self.load()?;
        if let Some(existing) = board.tasks.iter().find(|task| {
            task.origin.as_ref().is_some_and(|origin| {
                origin.device == input.from_device && origin.task_id == input.origin_task_id
            })
        }) {
            let source = existing.origin.as_ref().expect("verified origin");
            if source.source_revision == input.origin_revision
                && existing.title == input.task.title
                && existing.details == input.task.details
                && existing.assignee == input.task.assignee
                && existing.role == input.task.role
                && existing.priority == input.task.priority
                && existing.due == input.task.due
            {
                return Ok((existing.clone(), false));
            }
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Upstream revision conflicts with received local task; manual reconciliation needed",
            ));
        }
        if board.tasks.len() >= MAX_TASKS {
            return Err(invalid("Task board capacity reached"));
        }
        let id = board.next_id;
        board.next_id = id
            .checked_add(1)
            .ok_or_else(|| invalid("Task IDs exhausted"))?;
        let at = now();
        let task = Task {
            id,
            revision: 1,
            title: input.task.title,
            details: input.task.details,
            assignee: input.task.assignee,
            role: input.task.role,
            contact: None,
            priority: input.task.priority,
            status: Status::Todo,
            due: input.task.due,
            created_at: at,
            updated_at: at,
            // A receiving device has actually accepted this assignment.
            sent_at: Some(at),
            acknowledged_at: None,
            origin: Some(RemoteOrigin {
                device: input.from_device,
                task_id: input.origin_task_id,
                source_revision: input.origin_revision,
            }),
            delivery: None,
            remote_progress: None,
            events: vec![Event {
                at,
                kind: "peer_received".into(),
                detail: "Stored through an SSH session; human acknowledgment outstanding".into(),
            }],
        };
        board.tasks.push(task.clone());
        self.save(&board)?;
        Ok((task, true))
    }

    /// Called only after the sender verifies the SSH remote host and its
    /// machine-readable receipt for a durably stored assignment.
    pub fn record_device_delivery(
        &self,
        id: u64,
        revision: u64,
        alias: &str,
        remote_id: u64,
    ) -> io::Result<Task> {
        if !valid_device_id(alias) || remote_id == 0 {
            return Err(invalid("Invalid destination device or receipt"));
        }
        self.change(id, revision, |task| {
            if task.delivery.is_some() || task.origin.is_some() {
                return Err(invalid("Task has already been delivered or is inbound"));
            }
            let at = now();
            task.sent_at = Some(at);
            task.delivery = Some(DeviceDelivery {
                peer: alias.to_owned(),
                remote_task_id: remote_id,
                delivered_at: at,
            });
            task.remote_progress = None;
            Ok((
                "peer_delivered",
                format!(
                    "Receiver confirmed durable storage at peer {alias}; no human acknowledgment"
                ),
            ))
        })
    }

    pub fn reports_for(&self, device: &str) -> io::Result<Vec<RemoteReport>> {
        if !valid_device_id(device) {
            return Err(invalid("Invalid source device"));
        }
        Ok(self
            .load()?
            .tasks
            .into_iter()
            .filter_map(|task| {
                let origin = task.origin?;
                if origin.device != device {
                    return None;
                }
                Some(RemoteReport {
                    from_device: origin.device,
                    origin_task_id: origin.task_id,
                    remote_task_id: task.id,
                    status: task.status,
                    acknowledged: task.acknowledged_at.is_some(),
                })
            })
            .collect())
    }

    /// Deliberate refresh from an SSH-authenticated peer. Do not overwrite
    /// locally managed status or equate transport receipts with a real person.
    pub fn reconcile_remote_reports(
        &self,
        alias: &str,
        device: &str,
        reports: &[RemoteReport],
    ) -> io::Result<usize> {
        if !valid_device_id(alias)
            || !valid_device_id(device)
            || reports.len() > MAX_TASKS
            || reports.iter().any(|report| {
                report.from_device != device
                    || report.origin_task_id == 0
                    || report.remote_task_id == 0
            })
        {
            return Err(invalid("Invalid remote progress report"));
        }
        let _lock = self.writer_lock()?;
        let mut board = self.load()?;
        let mut changed = 0;
        for task in &mut board.tasks {
            let Some(delivery) = &task.delivery else {
                continue;
            };
            if delivery.peer != alias {
                continue;
            }
            let matching = reports.iter().find(|report| {
                report.origin_task_id == task.id && report.remote_task_id == delivery.remote_task_id
            });
            let Some(report) = matching else {
                continue;
            };
            if task.remote_progress.as_ref().is_some_and(|old| {
                old.status == report.status && old.acknowledged == report.acknowledged
            }) {
                continue;
            }
            let at = now().max(task.updated_at);
            task.remote_progress = Some(RemoteProgress {
                status: report.status,
                acknowledged: report.acknowledged,
                observed_at: at,
            });
            task.updated_at = at;
            task.revision = task
                .revision
                .checked_add(1)
                .ok_or_else(|| invalid("Task revision overflow"))?;
            task.events.push(Event {
                at,
                kind: "peer_progress".into(),
                detail: format!(
                    "Peer {alias} reports {:?}; acknowledged={}",
                    report.status, report.acknowledged
                ),
            });
            if task.events.len() > MAX_EVENTS {
                task.events.remove(0);
            }
            changed += 1;
        }
        if changed > 0 {
            self.save(&board)?;
        }
        Ok(changed)
    }

    pub fn mark_sent(&self, id: u64, revision: u64) -> io::Result<Task> {
        self.change(id, revision, |task| {
            if task.sent_at.is_some() {
                return Err(invalid("Already marked sent"));
            }
            task.sent_at = Some(now());
            Ok((
                "handoff",
                "Operator manually confirmed sending; delivery not verified".into(),
            ))
        })
    }

    pub fn acknowledge(&self, id: u64, revision: u64) -> io::Result<Task> {
        self.change(id, revision, |task| {
            if task.sent_at.is_none() {
                return Err(invalid("Record handoff before manually acknowledging"));
            }
            if task.acknowledged_at.is_some() {
                return Err(invalid("Acknowledgment already recorded"));
            }
            task.acknowledged_at = Some(now());
            Ok((
                "acknowledged",
                "Operator manually recorded acknowledgment; no remote proof".into(),
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_task() -> NewTask {
        NewTask {
            title: "Inspect north access".into(),
            details: "Check safe passage and report findings".into(),
            assignee: "Scout One".into(),
            role: "reclamation-scout".into(),
            contact: Some("scout@example.org".into()),
            priority: Priority::High,
            due: Some("2026-11-02".into()),
        }
    }

    fn incoming_task() -> NewTask {
        let mut task = new_task();
        task.contact = None;
        task
    }

    #[test]
    fn incoming_peer_assignment_is_durable_idempotent_and_role_scoped() {
        let temp = tempfile::tempdir().unwrap();
        let store = TaskStore::new(temp.path().join("receiver"));
        let packet = RemoteAssignment {
            schema_version: 1,
            from_device: "overseer-a".into(),
            origin_task_id: 14,
            origin_revision: 2,
            task: incoming_task(),
        };
        let (imported, created) = store
            .import_remote(RemoteAssignment {
                schema_version: packet.schema_version,
                from_device: packet.from_device.clone(),
                origin_task_id: packet.origin_task_id,
                origin_revision: packet.origin_revision,
                task: incoming_task(),
            })
            .unwrap();
        assert!(created);
        assert_eq!(imported.id, 1);
        assert_eq!(imported.origin.as_ref().unwrap().device, "overseer-a");
        assert!(imported.sent_at.is_some());
        assert!(imported.acknowledged_at.is_none());
        assert!(imported.delivery.is_none());
        let (same, added) = store.import_remote(packet).unwrap();
        assert!(!added);
        assert_eq!(same.id, imported.id);
        assert_eq!(store.load().unwrap().tasks.len(), 1);
        let ack = store.acknowledge(imported.id, imported.revision).unwrap();
        let working = store
            .set_status(
                ack.id,
                ack.revision,
                Status::InProgress,
                "Route verification",
            )
            .unwrap();
        let reports = store.reports_for("overseer-a").unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].origin_task_id, 14);
        assert_eq!(reports[0].status, Status::InProgress);
        assert!(reports[0].acknowledged);
        assert_eq!(working.status, Status::InProgress);
        assert_eq!(
            TaskStore::new(temp.path().join("receiver"))
                .load()
                .unwrap()
                .tasks
                .len(),
            1
        );
        let old_revision = RemoteAssignment {
            schema_version: 1,
            from_device: "overseer-a".into(),
            origin_task_id: 14,
            origin_revision: 1,
            task: incoming_task(),
        };
        assert_eq!(
            store.import_remote(old_revision).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        let unknown = RemoteAssignment {
            schema_version: 1,
            from_device: "evil".into(),
            origin_task_id: 15,
            origin_revision: 1,
            task: NewTask {
                role: "unrecognized".into(),
                ..incoming_task()
            },
        };
        assert!(store.import_remote(unknown).is_err());
    }

    #[test]
    fn device_delivery_and_explicit_sync_keep_manual_status_separate() {
        let temp = tempfile::tempdir().unwrap();
        let store = TaskStore::new(temp.path().join("manager"));
        let task = store.create(new_task()).unwrap();
        let sent = store
            .record_device_delivery(task.id, task.revision, "scout-one", 42)
            .unwrap();
        assert_eq!(sent.revision, 2);
        assert!(sent.delivery.is_some());
        assert!(sent.acknowledged_at.is_none());
        assert!(sent.remote_progress.is_none());
        assert_eq!(
            store
                .record_device_delivery(task.id, 1, "scout-one", 42)
                .unwrap_err()
                .kind(),
            io::ErrorKind::AlreadyExists
        );
        let first = RemoteReport {
            from_device: "overseer-a".into(),
            origin_task_id: task.id,
            remote_task_id: 42,
            status: Status::InProgress,
            acknowledged: true,
        };
        assert!(store
            .reconcile_remote_reports("scout-one", "wrong", std::slice::from_ref(&first))
            .is_err());
        assert_eq!(
            store
                .reconcile_remote_reports("scout-one", "overseer-a", std::slice::from_ref(&first))
                .unwrap(),
            1
        );
        let after = store.task(task.id).unwrap();
        assert_eq!(after.revision, 3);
        assert_eq!(after.status, Status::Todo);
        assert_eq!(
            after.remote_progress.as_ref().unwrap().status,
            Status::InProgress
        );
        assert!(after.remote_progress.as_ref().unwrap().acknowledged);
        assert!(after.acknowledged_at.is_none());
        assert_eq!(
            store
                .reconcile_remote_reports("scout-one", "overseer-a", &[first])
                .unwrap(),
            0
        );
        assert_eq!(store.load().unwrap().tasks.len(), 1);
        // Future schema and corruption must never be destroyed by import.
        fs::write(store.path(), "{\"schema_version\":999}").unwrap();
        assert!(store
            .reconcile_remote_reports("scout-one", "overseer-a", &[])
            .is_err());
        assert_eq!(
            fs::read_to_string(store.path()).unwrap(),
            "{\"schema_version\":999}"
        );
    }

    #[test]
    fn fresh_board_reads_without_creating_any_files() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("new");
        let store = TaskStore::new(path.clone());
        assert_eq!(store.load().unwrap().tasks.len(), 0);
        assert!(!path.exists());
    }

    #[test]
    fn creation_dispatch_ack_status_and_restart_persist() {
        let temp = tempfile::tempdir().unwrap();
        let store = TaskStore::new(temp.path().join("state"));
        let task = store.create(new_task()).unwrap();
        assert_eq!(task.id, 1);
        assert_eq!(task.revision, 1);
        assert!(task.sent_at.is_none());
        assert!(task.handoff().contains("TASK: RB-0001"));
        assert!(task.handoff().contains("scout@example.org"));
        assert!(task.handoff().contains("not an authenticated transmission"));
        assert!(store.acknowledge(task.id, 1).is_err());
        let sent = store.mark_sent(task.id, 1).unwrap();
        assert_eq!(sent.revision, 2);
        assert!(sent.sent_at.is_some());
        let ack = store.acknowledge(task.id, 2).unwrap();
        assert_eq!(ack.revision, 3);
        assert!(ack.acknowledged_at.is_some());
        let started = store
            .set_status(task.id, 3, Status::InProgress, "Working")
            .unwrap();
        assert_eq!(started.revision, 4);
        let done = store
            .set_status(task.id, 4, Status::Done, "Reviewed")
            .unwrap();
        assert_eq!(done.status, Status::Done);
        let again = TaskStore::new(temp.path().join("state"));
        assert_eq!(again.task(1).unwrap().status, Status::Done);
        assert_eq!(again.load().unwrap().tasks.len(), 1);
        assert_eq!(again.load().unwrap().next_id, 2);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(again.path()).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn optimistic_conflict_reassign_resets_delivery_but_keeps_history() {
        let temp = tempfile::tempdir().unwrap();
        let store = TaskStore::new(temp.path().to_path_buf());
        let task = store.create(new_task()).unwrap();
        let sent = store.mark_sent(task.id, task.revision).unwrap();
        assert_eq!(
            store.mark_sent(task.id, task.revision).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        let changed = store
            .reassign(
                task.id,
                sent.revision,
                "Engineer Two",
                "head-engineer",
                None,
            )
            .unwrap();
        assert!(changed.sent_at.is_none() && changed.acknowledged_at.is_none());
        assert_eq!(changed.role, "head-engineer");
        assert!(changed
            .events
            .iter()
            .any(|event| event.kind == "reassigned"));
        assert!(store.acknowledge(changed.id, changed.revision).is_err());
    }

    #[test]
    fn validation_rejects_invalid_dates_bad_contacts_and_control_characters() {
        assert!(valid_date("2028-02-29"));
        for date in [
            "2027-02-29",
            "2026-04-31",
            "2026-13-01",
            "2026-1-2",
            "1999-12-31",
            "2030/02/01",
            "2026-00-12",
        ] {
            assert!(!valid_date(date), "{date}");
        }
        assert!(valid_contact("ops+night@example.org"));
        assert!(!valid_contact("evil\nBcc:other@example.org"));
        for mut task in [new_task(), new_task(), new_task(), new_task()] {
            task.details = "Some notes".into();
            assert!(task.valid());
        }
        let temp = tempfile::tempdir().unwrap();
        let store = TaskStore::new(temp.path().to_path_buf());
        let mut bad = new_task();
        bad.due = Some("2026-02-30".into());
        assert!(store.create(bad).is_err());
        let mut bad = new_task();
        bad.assignee = "bad\nheader".into();
        assert!(store.create(bad).is_err());
        let mut bad = new_task();
        bad.contact = Some("bad@example.org\nbcc:evil@example.org".into());
        assert!(store.create(bad).is_err());
        assert!(!store.path().exists());
    }

    #[test]
    fn malformed_or_future_schema_does_not_get_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let store = TaskStore::new(temp.path().to_path_buf());
        fs::write(store.path(), "{\"schema_version\":99}").unwrap();
        assert!(store.load().is_err());
        assert!(store.create(new_task()).is_err());
        assert_eq!(
            fs::read_to_string(store.path()).unwrap(),
            "{\"schema_version\":99}"
        );
    }

    #[test]
    fn bounded_history_and_revisions_protect_against_spam() {
        let temp = tempfile::tempdir().unwrap();
        let store = TaskStore::new(temp.path().to_path_buf());
        let task = store.create(new_task()).unwrap();
        let mut revision = task.revision;
        for _ in 0..45 {
            let next = store.set_status(task.id, revision, Status::Blocked, "Waiting");
            if let Ok(updated) = next {
                revision = updated.revision;
            } else {
                let updated = store
                    .set_status(task.id, revision, Status::Todo, "Ready")
                    .unwrap();
                revision = updated.revision;
            }
        }
        let found = store.task(task.id).unwrap();
        assert!(found.events.len() <= MAX_EVENTS);
        assert!(found.revision > 33);
    }
}
