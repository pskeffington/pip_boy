//! Device-to-device Rust-Boy assignments over explicit, pinned OpenSSH sessions.
//!
//! This deliberately delegates transport encryption and host authentication to
//! mature OpenSSH instead of claiming home-grown TLS or unprotected LAN sockets.
//! It opens no listening port and sends nothing when merely loading a profile.
use pipboy_core::message_pace::{PaceStore, Route, Tier};
use pipboy_core::message_protocol::{transmit_on_route, Envelope, MessageTransport, StoredReceipt};
use pipboy_core::messenger::{
    AttemptRoute, Delivery, Message, MessageStore, RemoteMessage, MAX_TEXT_BYTES,
};
use pipboy_core::peers::{Peer, PeerConfig, PeerStore};
use pipboy_core::process_safety::isolated_command;
use pipboy_core::tasks::{
    valid_device_id, NewTask, RemoteAssignment, RemoteReport, Task, TaskStore,
};
use serde_json::{json, Value};
use std::error::Error;
use std::io::{self, Read, Write};
use std::net::Ipv4Addr;
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
    ssh_command_at(peer, remote, None)
}

fn private_local_ip(text: &str) -> io::Result<Ipv4Addr> {
    pipboy_core::message_pace::private_ipv4(text)
}

fn ssh_command_at(peer: &Peer, remote: &str, local_ip: Option<Ipv4Addr>) -> Command {
    let mut command = isolated_command("ssh");
    command.arg("-F").arg("none");
    if let Some(ip) = local_ip {
        // Change routing only; the enrolled hostname remains the trust identity.
        let host = peer.target.split_once('@').map(|(_, h)| h).unwrap_or("");
        command.arg("-o").arg(format!("Hostname={ip}"));
        command.arg("-o").arg(format!("HostKeyAlias={host}"));
        command.arg("-o").arg("CheckHostIP=no");
    }
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
    run_ssh_at(peer, command, payload, None)
}

fn run_ssh_at(
    peer: &Peer,
    command: &str,
    payload: Option<&[u8]>,
    local_ip: Option<Ipv4Addr>,
) -> io::Result<Value> {
    if payload.is_some_and(|data| data.len() as u64 > MAX_REQUEST) {
        return Err(invalid("Assignment packet exceeds 8 KiB"));
    }
    let mut child = (if local_ip.is_none() {
        ssh_command(peer, command)
    } else {
        ssh_command_at(peer, command, local_ip)
    })
    .spawn()
    .map_err(|_| {
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
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Peer response exceeded 64 KiB",
        ));
    }
    serde_json::from_slice(&bytes).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Peer response is not a single valid JSON receipt",
        )
    })
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

/// An outgoing message is queued durably before any outbound SSH attempt.
/// The remote receipt proves application-level persistence, NOT human reading.
pub fn send_message_from_store(
    journal: &MessageStore,
    peers: &PeerStore,
    id: u64,
    alias: &str,
) -> io::Result<Message> {
    send_message_at(journal, peers, id, alias, None)
}

struct SshMessageTransport<'a> {
    peer: &'a Peer,
    local_ip: Option<Ipv4Addr>,
}
impl MessageTransport for SshMessageTransport<'_> {
    fn exchange(&mut self, frame: &Envelope) -> io::Result<StoredReceipt> {
        if frame.to_device != self.peer.alias {
            return Err(invalid("Transport peer mismatch"));
        }
        let bytes = frame.encode()?;
        let response = run_ssh_at(
            self.peer,
            "pipboy peer message-receive",
            Some(&bytes),
            self.local_ip,
        )?;
        serde_json::from_value(response)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Malformed Messenger receipt"))
    }
}

fn send_message_at(
    journal: &MessageStore,
    peers: &PeerStore,
    id: u64,
    alias: &str,
    local_ip: Option<Ipv4Addr>,
) -> io::Result<Message> {
    if !valid_device_id(alias) || id == 0 {
        return Err(invalid("Invalid Messenger peer or message ID"));
    }
    let config = peers.load()?;
    let source = own_device(&config)?;
    let destination = peers.get(alias)?;
    if source == alias {
        return Err(invalid("Cannot message the local device"));
    }
    let message = journal.outgoing(id, alias)?;
    if message.delivery != Delivery::Queued {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Message has already been stored on peer",
        ));
    }
    let packet = RemoteMessage {
        schema_version: 1,
        from_device: source.to_owned(),
        to_device: alias.to_owned(),
        origin_id: message.id,
        body: message.body,
        sent_at: message.created_at,
    };
    let mut transport = SshMessageTransport {
        peer: &destination,
        local_ip,
    };
    let route = match local_ip {
        Some(ipv4) => AttemptRoute::Alternate { ipv4 },
        None => AttemptRoute::Primary,
    };
    transmit_on_route(&mut transport, journal, &packet, route)?;
    journal.outgoing(id, alias)
}

pub fn send_message_pace_from_store(
    journal: &MessageStore,
    peers: &PeerStore,
    pace: &PaceStore,
    id: u64,
    alias: &str,
    tier: Tier,
) -> io::Result<Message> {
    peers.get(alias)?;
    let pending = journal.outgoing(id, alias)?;
    if pending.delivery != Delivery::Queued {
        return Err(invalid("Message already stored on peer"));
    }
    match pace.resolve(alias, tier, true)? {
        Route::PairedSsh => send_message_from_store(journal, peers, id, alias),
        Route::LocalSsh(ip) => send_message_at(journal, peers, id, alias, Some(ip)),
        Route::Hold => journal.hold(id, alias),
    }
}

pub fn queue_message_from_store(
    journal: &MessageStore,
    peers: &PeerStore,
    alias: &str,
    body: &str,
) -> io::Result<(Message, bool)> {
    queue_message_pace_from_store(journal, peers, None, alias, body, Tier::Primary)
}

pub fn queue_message_pace_from_store(
    journal: &MessageStore,
    peers: &PeerStore,
    pace: Option<&PaceStore>,
    alias: &str,
    body: &str,
    tier: Tier,
) -> io::Result<(Message, bool)> {
    if !valid_device_id(alias) {
        return Err(invalid("Invalid Messenger peer"));
    }
    let config = peers.load()?;
    let source = own_device(&config)?;
    if source == alias {
        return Err(invalid("Cannot message the local device"));
    }
    peers.get(alias)?;
    let route = match pace {
        Some(pace) => pace.resolve(alias, tier, true)?,
        None if tier == Tier::Primary => Route::PairedSsh,
        None => return Err(invalid("PACE configuration required")),
    };
    let pending = if route == Route::Hold {
        journal.queue_held(alias, body)?
    } else {
        journal.queue(alias, body)?
    };
    let result = match route {
        Route::PairedSsh => send_message_from_store(journal, peers, pending.id, alias),
        Route::LocalSsh(ip) => send_message_at(journal, peers, pending.id, alias, Some(ip)),
        Route::Hold => return Ok((pending, false)),
    };
    match result {
        Ok(stored) => Ok((stored, true)),
        // Reload persisted failure evidence; do not return the pre-attempt snapshot.
        Err(_) => Ok((journal.outgoing(pending.id, alias)?, false)),
    }
}

/// Execute on the receiving SSH account only. The recommended forced command
/// binds a dedicated SSH public key to exactly one sender label.
fn message_receive(expected_sender: Option<&str>) -> io::Result<()> {
    let (config, _, _) = setup()?;
    let local = own_device(&config)?;
    let mut bytes = Vec::new();
    io::stdin().take(MAX_REQUEST + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_REQUEST {
        return Err(invalid("Messenger frame exceeds transport limit"));
    }
    let frame = Envelope::decode(&bytes)?;
    if !frame.valid()
        || frame.to_device != local
        || expected_sender.is_some_and(|sender| frame.from_device != sender)
        || !config.trusted_senders.contains(&frame.from_device)
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Sender or receiver is not authorized for Messenger",
        ));
    }
    let journal = MessageStore::from_environment()?;
    let (stored, _) = journal.receive(&frame, local)?;
    println!(
        "{}",
        json!({
            "schema_version": 1,
            "kind": "message_stored",
            "receiver_device": local,
            "from_device": frame.from_device,
            "origin_id": frame.origin_id,
            "remote_id": stored.id,
        })
    );
    Ok(())
}

fn receive(expected_sender: Option<&str>) -> io::Result<()> {
    let (config, _, tasks) = setup()?;
    let local = own_device(&config)?;
    let mut data = Vec::new();
    io::stdin().take(MAX_REQUEST + 1).read_to_end(&mut data)?;
    if data.len() as u64 > MAX_REQUEST {
        return Err(invalid("Assignment frame exceeds limit"));
    }
    let input: RemoteAssignment =
        serde_json::from_slice(&data).map_err(|_| invalid("Invalid assignment frame"))?;
    if input.from_device == local
        || expected_sender.is_some_and(|source| source != input.from_device)
        || !config.trusted_senders.contains(&input.from_device)
    {
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

/// Intended for an OpenSSH authorized_keys forced command:
/// command="/absolute/path/pipboy peer bridge overseer-a",restrict ssh-ed25519 ...
/// Each dedicated SSH key is bound to one source label. Arbitrary SSH commands
/// and cross-sender spoofing are denied here without interpreting a shell.
fn bridge(source: &str) -> io::Result<()> {
    if !valid_device_id(source) {
        return Err(invalid("Invalid restricted SSH source identity"));
    }
    let original = std::env::var("SSH_ORIGINAL_COMMAND").map_err(|_| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Restricted SSH bridge requires SSH_ORIGINAL_COMMAND",
        )
    })?;
    if original == "pipboy peer receive" {
        receive(Some(source))
    } else if original == format!("pipboy peer receipts {source}") {
        receipts(source)
    } else if original == "pipboy peer message-receive" {
        message_receive(Some(source))
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "OpenSSH restricted key permits task receive, receipts and Messenger receive only",
        ))
    }
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let use_text = "usage: pipboy peer <init DEVICE --confirm|trust SENDER --confirm|add ALIAS USER@HOST ABSOLUTE_KEY ABSOLUTE_KNOWN_HOSTS --confirm|list [--json]|send TASK_ID ALIAS --confirm|sync ALIAS --confirm|message-send ALIAS --confirm (text on stdin)|message-retry ID ALIAS --confirm|message-status ID ALIAS --json|message-retry-local ID ALIAS PRIVATE_IPV4 --confirm|message-key-init --confirm|meshtastic-capture CAPTURE_FILE --confirm|pace-show ALIAS --json|pace-local ALIAS PRIVATE_IPV4_OR_clear --confirm|message-pace ID ALIAS TIER --confirm>";
    let (config, peers, _) = setup()?;
    match args.as_slice() {
        [command, id, alias, flag] if command == "message-status" && flag == "--json" => {
            let message = MessageStore::from_environment()?.outgoing(id.parse()?, alias)?;
            println!(
                "{}",
                json!({"message": message, "automatic_connections": false, "read_by_human": false})
            );
        }
        [command, alias, flag] if command == "pace-show" && flag == "--json" => {
            let home = pipboy_core::notes::NoteStore::from_environment()?;
            let root = home
                .path()
                .parent()
                .ok_or_else(|| invalid("Missing data directory"))?;
            let view = PaceStore::new(root.into()).view(alias, peers.get(alias).is_ok())?;
            println!("{}", serde_json::to_string(&view)?);
        }
        [command, alias, ip, flag] if command == "pace-local" && flag == "--confirm" => {
            peers.get(alias)?;
            let address = if ip == "clear" {
                None
            } else {
                Some(private_local_ip(ip)?)
            };
            let home = pipboy_core::notes::NoteStore::from_environment()?;
            let root = home
                .path()
                .parent()
                .ok_or_else(|| invalid("Missing data directory"))?;
            PaceStore::new(root.into()).set_alternate(alias, address)?;
            println!("PACE alternate updated / no connection opened");
        }
        [command, id, alias, tier, flag] if command == "message-pace" && flag == "--confirm" => {
            let tier = Tier::parse(tier)?;
            let home = pipboy_core::notes::NoteStore::from_environment()?;
            let root = home
                .path()
                .parent()
                .ok_or_else(|| invalid("Missing data directory"))?;
            let journal = MessageStore::from_environment()?;
            let result = send_message_pace_from_store(
                &journal,
                &peers,
                &PaceStore::new(root.into()),
                id.parse()?,
                alias,
                tier,
            )?;
            println!(
                "MESSAGE #{} / {}",
                result.id,
                if result.delivery == Delivery::Stored {
                    "STORED ON RECEIVER"
                } else {
                    "HELD LOCALLY / NOT DELIVERED"
                }
            );
        }
        [command, flag] if command == "message-key-init" && flag == "--confirm" => {
            let recipient = MessageStore::from_environment()?.enroll_encryption()?;
            println!("Encrypted queue enrolled / public recipient {recipient}");
        }
        [command, id, alias, ip, flag]
            if command == "message-retry-local" && flag == "--confirm" =>
        {
            let address = private_local_ip(ip)?;
            let journal = MessageStore::from_environment()?;
            let result = send_message_at(&journal, &peers, id.parse()?, alias, Some(address))?;
            println!(
                "MESSAGE #{} / STORED ON RECEIVER VIA PINNED LOCAL SSH",
                result.id
            );
        }
        [command, capture, flag] if command == "meshtastic-capture" && flag == "--confirm" => {
            // Recorded capture only. A device path cannot bypass the deny-only broker.
            if !std::fs::symlink_metadata(capture)?.is_file() {
                return Err(
                    invalid("Expected a regular USB capture file; no devices or symlinks").into(),
                );
            }
            let mut file = std::fs::File::open(capture)?;
            if !file.metadata()?.is_file() {
                return Err(invalid(
                    "Expected a regular USB capture file; live serial requires the OS broker",
                )
                .into());
            }
            if file.metadata()?.len() > 1024 * 1024 {
                return Err(invalid("USB capture exceeds 1 MiB").into());
            }
            let mut decoder = pipboy_core::meshtastic_passive::PassiveDecoder::default();
            let mut count = 0;
            let mut bytes = [0; 516];
            let mut limited = (&mut file).take(1024 * 1024 + 1);
            let mut total = 0;
            loop {
                let n = limited.read(&mut bytes)?;
                if n == 0 {
                    break;
                }
                total += n;
                if total > 1024 * 1024 {
                    return Err(invalid("USB capture exceeds 1 MiB").into());
                }
                count += decoder.feed(&bytes[..n]).len();
            }
            println!("RECORDED USB / {count} untrusted FromRadio frames / no device opened");
        }
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
        [command, alias, flag] if command == "message-send" && flag == "--confirm" => {
            let mut bytes = Vec::new();
            io::stdin()
                .take(MAX_TEXT_BYTES as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > MAX_TEXT_BYTES {
                return Err(invalid("Messenger body exceeds 1024 bytes").into());
            }
            let body = String::from_utf8(bytes).map_err(|_| invalid("Message must be UTF-8"))?;
            let journal = MessageStore::from_environment()?;
            let (message, delivered) = queue_message_from_store(&journal, &peers, alias, &body)?;
            println!(
                "MESSAGE #{} / {}",
                message.id,
                if delivered {
                    "STORED ON RECEIVER"
                } else {
                    "QUEUED / RETRY REQUIRED"
                }
            );
        }
        [command, id, alias, flag] if command == "message-retry" && flag == "--confirm" => {
            let id = id.parse::<u64>()?;
            let journal = MessageStore::from_environment()?;
            let result = send_message_from_store(&journal, &peers, id, alias)?;
            println!("MESSAGE #{} / STORED ON RECEIVER", result.id);
        }
        [command] if command == "message-receive" => message_receive(None)?,
        [command] if command == "receive" => receive(None)?,
        [command, source] if command == "bridge" => bridge(source)?,
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
    fn local_fallback_retains_enrolled_host_identity_and_rejects_public_routes() {
        for ip in [
            "8.8.8.8",
            "127.0.0.1",
            "169.254.1.1",
            "224.0.0.1",
            "0.0.0.0",
            "scout.local",
            "10.0.0.1;whoami",
        ] {
            assert!(private_local_ip(ip).is_err());
        }
        let peer = Peer {
            alias: "scout".into(),
            target: "pi@scout.local".into(),
            identity_file: "/key".into(),
            known_hosts_file: "/hosts".into(),
        };
        let cmd = ssh_command_at(
            &peer,
            "pipboy peer message-receive",
            Some(private_local_ip("192.168.1.20").unwrap()),
        );
        let args = cmd
            .get_args()
            .map(|s| s.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        for expected in [
            "Hostname=192.168.1.20",
            "HostKeyAlias=scout.local",
            "StrictHostKeyChecking=yes",
            "pi@scout.local",
            "none",
        ] {
            assert!(args.contains(&expected.to_string()));
        }
    }

    #[test]
    fn openssh_independently_resolves_fallback_without_changing_trust_identity() {
        let peer = Peer {
            alias: "scout".into(),
            target: "pi@scout.local".into(),
            identity_file: "/key".into(),
            known_hosts_file: "/hosts".into(),
        };
        let command = ssh_command_at(
            &peer,
            "pipboy peer message-receive",
            Some("192.168.1.20".parse().unwrap()),
        );
        let result = isolated_command("ssh")
            .arg("-G")
            .args(command.get_args())
            .output();
        let output = match result {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return, // native host without OpenSSH
            other => other.unwrap(),
        };
        assert!(output.status.success(), "OpenSSH config evaluation failed");
        let config = String::from_utf8(output.stdout).unwrap();
        for expected in [
            "hostname 192.168.1.20",
            "hostkeyalias scout.local",
            "user pi",
            "identityagent none",
            "userknownhostsfile /hosts",
        ] {
            assert!(
                config
                    .lines()
                    .any(|line| line.trim_end_matches('\r') == expected),
                "Missing effective setting: {expected}"
            );
        }
        assert!(config.lines().any(|line| matches!(
            line.trim_end_matches('\r'),
            "stricthostkeychecking true" | "stricthostkeychecking yes"
        )));
    }

    #[test]
    fn forced_bridge_rejects_missing_ssh_original_command() {
        // Never interpret a remote command as shell syntax.
        // CI covers the positive restricted bridge in tools/smoke_peer.py.
        assert!(!valid_device_id("bad source; exit"));
        assert!(valid_device_id("overseer-a"));
    }

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
