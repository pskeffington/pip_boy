//! Device-to-device Rust-Boy assignments over explicit, pinned OpenSSH sessions.
//!
//! This deliberately delegates transport encryption and host authentication to
//! mature OpenSSH instead of claiming home-grown TLS or unprotected LAN sockets.
//! It opens no listening port and sends nothing when merely loading a profile.
use pipboy_core::peers::{Peer, PeerConfig, PeerStore};
use pipboy_core::tasks::{
    valid_device_id, NewTask, RemoteAssignment, RemoteReport, Task, TaskStore,
};
use serde_json::{json, Value};
use std::error::Error;
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const MAX_REQUEST: u64 = 8192;
const MAX_RESPONSE: u64 = 65536;
const SESSION_TIMEOUT: Duration = Duration::from_secs(15);

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn setup() -> io::Result<(PeerConfig, PeerStore, TaskStore)> {
    let peers = PeerStore::from_environment()?;
    let board = TaskStore::from_environment()?;
    let config = peers.load()?;
    Ok((config, peers, board))
}

fn own_device(config: &PeerConfig) -> io::Result<&str> {
    config
        .device_id
        .as_deref()
        .ok_or_else(|| invalid("Initialize the device with 'pipboy peer init NAME --confirm'"))
}

/// Every option is fixed; target is tightly validated by PeerStore; the only
/// remote command is an authored literal or a validated device-id suffix.
/// No shell is launched on the local machine; an SSH server runs a fixed
/// 'pipboy peer' subcommand in its configured account.
fn ssh_command(peer: &Peer, remote: &str) -> Command {
    let mut command = Command::new("ssh");
    for option in [
        "BatchMode=yes",
        "StrictHostKeyChecking=yes",
        "IdentitiesOnly=yes",
        "IdentityAgent=none",
        "PasswordAuthentication=no",
        "KbdInteractiveAuthentication=no",
        "GSSAPIAuthentication=no",
        "ForwardAgent=no",
        "ClearAllForwardings=yes",
        "ControlMaster=no",
        "ControlPath=none",
        "PermitLocalCommand=no",
        "ProxyCommand=none",
        "ProxyJump=none",
        "UpdateHostKeys=no",
        "ConnectTimeout=5",
        "ConnectionAttempts=1",
        "RequestTTY=no",
        "LogLevel=ERROR",
        "GlobalKnownHostsFile=none",
    ] {
        command.arg("-o").arg(option);
    }
    command
        .arg("-o")
        .arg(format!("UserKnownHostsFile={}", peer.known_hosts_file))
        .arg("-i")
        .arg(&peer.identity_file)
        .arg("-T")
        .arg("-x")
        .arg("-a")
        .arg(&peer.target)
        .arg(remote)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
}

fn run_ssh(peer: &Peer, command: &str, payload: Option<&[u8]>) -> io::Result<Value> {
    if payload.is_some_and(|data| data.len() as u64 > MAX_REQUEST) {
        return Err(invalid("Assignment packet exceeds 8 KiB"));
    }
    let mut child = ssh_command(peer, command).spawn().map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "OpenSSH client unavailable or unable to start",
        )
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| invalid("Missing SSH output stream"))?;
    let output = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take(MAX_RESPONSE + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    // Bounded packet fits a pipe. Dropping stdin completes the remote frame.
    if let Some(mut pipe) = child.stdin.take() {
        if let Some(bytes) = payload {
            if pipe.write_all(bytes).is_err() {
                let _ = child.kill();
                let _ = child.wait();
                let _ = output.join();
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "Peer command declined the assignment packet",
                ));
            }
        }
    }
    let deadline = Instant::now() + SESSION_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = output.join();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "SSH peer request timed out; no local delivery receipt recorded",
            ));
        }
        thread::sleep(Duration::from_millis(40));
    };
    let bytes = output
        .join()
        .map_err(|_| invalid("Peer output reader failed"))??;
    if !status.success() {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied,
            "Peer rejected SSH session or remote command; verify pinned host key, SSH key, and remote Rust-Boy setup"));
    }
    if bytes.len() as u64 > MAX_RESPONSE {
        return Err(invalid("Peer response exceeded 64 KiB"));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| invalid("Peer response is not a single valid JSON receipt"))
}

fn get_id<'a>(value: &'a Value, field: &str) -> io::Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("Peer response missing identifying field"))
}

fn get_num(value: &Value, field: &str) -> io::Result<u64> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .filter(|number| *number > 0)
        .ok_or_else(|| invalid("Peer response missing numeric field"))
}

pub fn safe_peers_from_store(store: &PeerStore) -> io::Result<Value> {
    let config = store.load()?;
    let entries: Vec<Value> = config
        .peers
        .iter()
        .map(|entry| json!({"alias": entry.alias, "target": entry.target}))
        .collect();
    Ok(json!({
        "ready": config.device_id.is_some() && !entries.is_empty(),
        "device_id": config.device_id,
        "peers": entries,
        "transport": "openssh_explicit_only",
    }))
}

pub fn send_from_store(
    tasks: &TaskStore,
    peers: &PeerStore,
    id: u64,
    revision: u64,
    alias: &str,
) -> io::Result<Task> {
    if !valid_device_id(alias) || id == 0 || revision == 0 {
        return Err(invalid("Invalid recipient alias, task ID or revision"));
    }
    let config = peers.load()?;
    let source = own_device(&config)?;
    let destination = peers.get(alias)?;
    let task = tasks.task(id)?;
    if task.revision != revision {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Task changed; reload before sending",
        ));
    }
    if task.origin.is_some()
        || task.delivery.is_some()
        || task.status == pipboy_core::tasks::Status::Done
    {
        return Err(invalid(
            "Incoming, completed, or already delivered tasks cannot be sent",
        ));
    }
    let packet = RemoteAssignment {
        schema_version: 1,
        from_device: source.to_owned(),
        origin_task_id: task.id,
        origin_revision: task.revision,
        task: NewTask {
            title: task.title.clone(),
            details: task.details.clone(),
            assignee: task.assignee.clone(),
            role: task.role.clone(),
            contact: None, // never forward legacy email addresses
            priority: task.priority,
            due: task.due.clone(),
        },
    };
    let bytes = serde_json::to_vec(&packet)?;
    let receipt = run_ssh(&destination, "pipboy peer receive", Some(&bytes))?;
    if receipt.get("schema_version").and_then(Value::as_u64) != Some(1)
        || get_id(&receipt, "kind")? != "stored"
        || get_id(&receipt, "receiver_device")? != alias
        || get_id(&receipt, "from_device")? != source
        || get_num(&receipt, "origin_task_id")? != id
    {
        return Err(invalid(
            "SSH peer returned an unexpected or mismatched storage receipt",
        ));
    }
    let remote_id = get_num(&receipt, "remote_task_id")?;
    tasks.record_device_delivery(id, revision, alias, remote_id)
}

pub fn send_task(id: u64, revision: u64, alias: &str) -> io::Result<Task> {
    let (_, peers, tasks) = setup()?;
    send_from_store(&tasks, &peers, id, revision, alias)
}

pub fn sync_from_store(tasks: &TaskStore, peers: &PeerStore, alias: &str) -> io::Result<usize> {
    if !valid_device_id(alias) {
        return Err(invalid("Invalid peer alias"));
    }
    let config = peers.load()?;
    let source = own_device(&config)?;
    let destination = peers.get(alias)?;
    let response = run_ssh(
        &destination,
        &format!("pipboy peer receipts {source}"),
        None,
    )?;
    if response.get("schema_version").and_then(Value::as_u64) != Some(1)
        || get_id(&response, "kind")? != "reports"
        || get_id(&response, "receiver_device")? != alias
        || get_id(&response, "for_device")? != source
    {
        return Err(invalid("Peer returned a mismatched progress response"));
    }
    let raw = response
        .get("reports")
        .cloned()
        .ok_or_else(|| invalid("Peer reports missing"))?;
    let reports: Vec<RemoteReport> =
        serde_json::from_value(raw).map_err(|_| invalid("Remote reports are malformed"))?;
    tasks.reconcile_remote_reports(alias, source, &reports)
}

pub fn sync_peer(alias: &str) -> io::Result<usize> {
    let (_, peers, tasks) = setup()?;
    sync_from_store(&tasks, &peers, alias)
}

fn receive() -> io::Result<()> {
    let (config, _, tasks) = setup()?;
    let local = own_device(&config)?;
    let mut data = Vec::new();
    io::stdin().take(MAX_REQUEST + 1).read_to_end(&mut data)?;
    if data.len() as u64 > MAX_REQUEST {
        return Err(invalid("Assignment frame exceeds limit"));
    }
    let input: RemoteAssignment =
        serde_json::from_slice(&data).map_err(|_| invalid("Invalid assignment frame"))?;
    if input.from_device == local || !config.trusted_senders.contains(&input.from_device) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Sender device label is not trusted on this device",
        ));
    }
    let from = input.from_device.clone();
    let source_id = input.origin_task_id;
    let (stored, _) = tasks.import_remote(input)?;
    println!(
        "{}",
        json!({
            "schema_version": 1,
            "kind": "stored",
            "receiver_device": local,
            "from_device": from,
            "origin_task_id": source_id,
            "remote_task_id": stored.id,
        })
    );
    Ok(())
}

fn receipts(device: &str) -> io::Result<()> {
    if !valid_device_id(device) {
        return Err(invalid("Invalid source device"));
    }
    let (config, _, tasks) = setup()?;
    let local = own_device(&config)?;
    if !config
        .trusted_senders
        .iter()
        .any(|trusted| trusted == device)
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Source device not explicitly trusted",
        ));
    }
    let reports = tasks.reports_for(device)?;
    println!(
        "{}",
        json!({
            "schema_version": 1,
            "kind": "reports",
            "receiver_device": local,
            "for_device": device,
            "reports": reports,
        })
    );
    Ok(())
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let use_text = "usage: pipboy peer <init DEVICE --confirm|trust SENDER --confirm|add ALIAS USER@HOST ABSOLUTE_KEY ABSOLUTE_KNOWN_HOSTS --confirm|list [--json]|send TASK_ID ALIAS --confirm|sync ALIAS --confirm>";
    let (config, peers, _) = setup()?;
    match args.as_slice() {
        [command, id, flag] if command == "init" && flag == "--confirm" => {
            peers.init(id)?;
            println!("Rust-Boy device enrolled: {id} / no connection opened");
        }
        [command, id, flag] if command == "trust" && flag == "--confirm" => {
            peers.trust_sender(id)?;
            println!("Trusted sender label enrolled: {id}; SSH account access still required");
        }
        [command, alias, target, key, known_hosts, flag]
            if command == "add" && flag == "--confirm" =>
        {
            peers.add(Peer {
                alias: alias.clone(),
                target: target.clone(),
                identity_file: key.clone(),
                known_hosts_file: known_hosts.clone(),
            })?;
            println!("Paired {alias} at {target} / SSH host-key verification required");
        }
        [command] if command == "list" => {
            println!(
                "LOCAL DEVICE: {}",
                config.device_id.as_deref().unwrap_or("NOT ENROLLED")
            );
            for peer in config.peers {
                println!(
                    "{} -> {} (pinned SSH / no auto-connect)",
                    peer.alias, peer.target
                );
            }
        }
        [command, flag] if command == "list" && flag == "--json" => {
            println!("{}", safe_peers_from_store(&peers)?);
        }
        [command, id, alias, flag] if command == "send" && flag == "--confirm" => {
            let id = id.parse::<u64>()?;
            let task = TaskStore::from_environment()?.task(id)?;
            let updated = send_task(id, task.revision, alias)?;
            println!("RB-{id:04} stored on device {alias}; remote ID {} / operator acknowledgment pending",
                updated.delivery.as_ref().map(|d| d.remote_task_id).unwrap_or(0));
        }
        [command, alias, flag] if command == "sync" && flag == "--confirm" => {
            let count = sync_peer(alias)?;
            println!("{count} locally stored tasks updated with peer-reported progress");
        }
        [command] if command == "receive" => receive()?,
        [command, device] if command == "receipts" => receipts(device)?,
        _ => return Err(use_text.into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pipboy_core::peers::Peer;

    #[test]
    fn ssh_invocation_is_fixed_and_does_not_skip_host_validation() {
        let peer = Peer {
            alias: "scout-one".into(),
            target: "pi@scout.local".into(),
            identity_file: "/home/operator/.ssh/id_ed25519".into(),
            known_hosts_file: "/home/operator/.ssh/known_hosts".into(),
        };
        let cmd = ssh_command(&peer, "pipboy peer receive");
        let args = cmd
            .get_args()
            .map(|x| x.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(args.contains(&"StrictHostKeyChecking=yes".to_owned()));
        assert!(args.contains(&"BatchMode=yes".to_owned()));
        assert!(args.contains(&"ForwardAgent=no".to_owned()));
        assert!(args.contains(&"ClearAllForwardings=yes".to_owned()));
        assert!(args.contains(&"UserKnownHostsFile=/home/operator/.ssh/known_hosts".to_owned()));
        assert!(args.contains(&"pi@scout.local".to_owned()));
        assert_eq!(args.last().map(String::as_str), Some("pipboy peer receive"));
        assert!(!args.contains(&"StrictHostKeyChecking=no".to_owned()));
    }
}
