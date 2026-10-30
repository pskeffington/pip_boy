//! Supervised child-process adapter transport.
//!
//! This provides protocol fault containment only. A child runs with the same
//! user privileges as Pip-Boy; OS sandboxing and process-tree containment are
//! separate platform gates. Only compile-time host policy may create a
//! `TrustedProcessSpec`; manifests and UI input never provide executable
//! paths or arguments.

use crate::process_safety::isolated_package_command;
use crate::universal_adapter::{
    AdapterEnvelope, AdapterError, AdapterReply, UniversalPackageAdapter,
};
use ring::digest::{Context, SHA256};
use rust_boy_adapter_sdk::process_protocol::ProcessRunner;
use serde::Deserialize;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub const CONTROL_DEADLINE: Duration = Duration::from_secs(2);
pub const SCAN_DEADLINE: Duration = Duration::from_secs(30);
const CHILD_POLL: Duration = Duration::from_millis(20);
const STDERR_DRAIN_BYTES: usize = 8 * 1024;
const EXECUTABLE_MAX_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmbeddedRoster {
    schema_version: u32,
    executables: Vec<EmbeddedExecutable>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmbeddedExecutable {
    package_id: String,
    relative_executable: String,
    sha256: String,
    relative_manifest: String,
    manifest_sha256: String,
    arguments: Vec<String>,
}

/// Every launchable executable is selected by a source-controlled, reviewed
/// hash pin and a fixed installation-relative path. The roster starts empty;
/// modules remain non-launchable until their release artifact is reviewed.
const EXECUTABLE_ROSTER: &str = include_str!("../../../catalog/approved_package_executables.json");

/// A validated executable selection. Fields are private so callers cannot
/// construct a selection from a package manifest or HTTP request.
#[derive(Clone, Debug)]
pub struct TrustedProcessSpec {
    package_id: String,
    executable: PathBuf,
    expected_sha256: [u8; 32],
    arguments: Vec<String>,
}

impl TrustedProcessSpec {
    pub(crate) fn package_id(&self) -> &str {
        &self.package_id
    }

    fn from_roster_entry(
        entry: &EmbeddedExecutable,
        install_root: &Path,
    ) -> Result<Self, ProcessError> {
        validate_package_id(&entry.package_id)?;
        if entry.relative_executable.is_empty()
            || entry.relative_executable.contains('\\')
            || Path::new(&entry.relative_executable).is_absolute()
            || entry
                .relative_executable
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(ProcessError::UntrustedExecutable);
        }
        let expected_sha256 = decode_sha256(&entry.sha256)?;
        if entry.arguments.len() > 8
            || entry
                .arguments
                .iter()
                .any(|arg| arg.len() > 256 || arg.contains('\0'))
        {
            return Err(ProcessError::UntrustedExecutable);
        }
        let root = fs::canonicalize(install_root).map_err(|_| ProcessError::UntrustedExecutable)?;
        let candidate = root.join(&entry.relative_executable);
        let canonical =
            fs::canonicalize(&candidate).map_err(|_| ProcessError::UntrustedExecutable)?;
        if !canonical.starts_with(&root) || canonical != candidate {
            return Err(ProcessError::UntrustedExecutable);
        }
        Ok(Self {
            package_id: entry.package_id.clone(),
            executable: canonical,
            expected_sha256,
            arguments: entry.arguments.clone(),
        })
    }

    #[cfg(test)]
    fn for_test(package_id: &str, executable: PathBuf, arguments: Vec<String>) -> Self {
        use ring::digest::digest;
        let expected_sha256 = digest(&SHA256, &fs::read(&executable).unwrap())
            .as_ref()
            .try_into()
            .unwrap();
        Self {
            package_id: package_id.into(),
            executable,
            expected_sha256,
            arguments,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessError {
    UntrustedExecutable,
    InvalidRoster,
    SpawnFailed,
    HandshakeFailed,
    TimedOut,
    Exited,
    ProtocolFailed,
}

/// Return only packages named in the compiled host roster. An empty roster is
/// a valid production-safe state and starts no child processes.
pub fn approved_process_ids() -> Result<Vec<String>, ProcessError> {
    let roster: EmbeddedRoster =
        serde_json::from_str(EXECUTABLE_ROSTER).map_err(|_| ProcessError::InvalidRoster)?;
    if roster.schema_version != 1 || roster.executables.len() > 64 {
        return Err(ProcessError::InvalidRoster);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut ids = Vec::with_capacity(roster.executables.len());
    for entry in roster.executables {
        validate_package_id(&entry.package_id)?;
        decode_sha256(&entry.sha256)?;
        decode_sha256(&entry.manifest_sha256)?;
        if !seen.insert(entry.package_id.clone()) {
            return Err(ProcessError::InvalidRoster);
        }
        ids.push(entry.package_id);
    }
    Ok(ids)
}

/// Resolve the executable and its separately pinned manifest from the same
/// installed package root. Neither path nor manifest can be chosen by the UI.
pub fn approved_process_install(
    package_id: &str,
    install_root: &Path,
) -> Result<(TrustedProcessSpec, String), ProcessError> {
    let roster: EmbeddedRoster =
        serde_json::from_str(EXECUTABLE_ROSTER).map_err(|_| ProcessError::InvalidRoster)?;
    if roster.schema_version != 1 || roster.executables.len() > 64 {
        return Err(ProcessError::InvalidRoster);
    }
    let mut seen = std::collections::BTreeSet::new();
    for entry in &roster.executables {
        if !seen.insert(entry.package_id.as_str()) {
            return Err(ProcessError::InvalidRoster);
        }
    }
    let entry = roster
        .executables
        .iter()
        .find(|entry| entry.package_id == package_id)
        .ok_or(ProcessError::UntrustedExecutable)?;
    let package_root = resolve_package_root(install_root, &entry.package_id)?;
    resolve_approved_install_entry(entry, &package_root)
}

fn resolve_approved_install_entry(
    entry: &EmbeddedExecutable,
    install_root: &Path,
) -> Result<(TrustedProcessSpec, String), ProcessError> {
    let spec = TrustedProcessSpec::from_roster_entry(entry, install_root)?;
    let root = fs::canonicalize(install_root).map_err(|_| ProcessError::UntrustedExecutable)?;
    let manifest_path = resolve_roster_path(&root, &entry.relative_manifest)?;
    let metadata =
        fs::symlink_metadata(&manifest_path).map_err(|_| ProcessError::UntrustedExecutable)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 16 * 1024 {
        return Err(ProcessError::UntrustedExecutable);
    }
    let manifest_bytes = fs::read(&manifest_path).map_err(|_| ProcessError::UntrustedExecutable)?;
    let expected_manifest_sha256 = decode_sha256(&entry.manifest_sha256)?;
    if ring::digest::digest(&SHA256, &manifest_bytes).as_ref() != expected_manifest_sha256 {
        return Err(ProcessError::UntrustedExecutable);
    }
    let manifest_json =
        String::from_utf8(manifest_bytes).map_err(|_| ProcessError::UntrustedExecutable)?;
    let manifest = crate::package_socket::PackageManifest::parse(&manifest_json)
        .map_err(|_| ProcessError::UntrustedExecutable)?;
    if manifest.package_id != entry.package_id
        || manifest.status.is_some()
        || manifest.entrypoint.kind != crate::package_socket::EntrypointKind::ExternalProcess
    {
        return Err(ProcessError::UntrustedExecutable);
    }
    Ok((spec, manifest_json))
}

fn resolve_package_root(root: &Path, package_id: &str) -> Result<PathBuf, ProcessError> {
    validate_package_id(package_id)?;
    let root = fs::canonicalize(root).map_err(|_| ProcessError::UntrustedExecutable)?;
    let candidate = root.join(package_id);
    let metadata =
        fs::symlink_metadata(&candidate).map_err(|_| ProcessError::UntrustedExecutable)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ProcessError::UntrustedExecutable);
    }
    let canonical = fs::canonicalize(&candidate).map_err(|_| ProcessError::UntrustedExecutable)?;
    if !canonical.starts_with(&root) || canonical != candidate {
        return Err(ProcessError::UntrustedExecutable);
    }
    Ok(canonical)
}

fn resolve_roster_path(root: &Path, relative: &str) -> Result<PathBuf, ProcessError> {
    if relative.is_empty()
        || relative.contains('\\')
        || Path::new(relative).is_absolute()
        || relative
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(ProcessError::UntrustedExecutable);
    }
    let candidate = root.join(relative);
    let canonical = fs::canonicalize(&candidate).map_err(|_| ProcessError::UntrustedExecutable)?;
    if !canonical.starts_with(root) || canonical != candidate {
        return Err(ProcessError::UntrustedExecutable);
    }
    Ok(canonical)
}

struct ChildRequest {
    envelope: AdapterEnvelope,
    reply: mpsc::SyncSender<Result<AdapterReply, AdapterError>>,
}

enum IoCommand {
    Request(AdapterEnvelope),
    Stop,
}
enum IoEvent {
    Handshake(Result<(), AdapterError>),
    Reply(Result<AdapterReply, AdapterError>),
}
type ExitHook = Arc<dyn Fn() + Send + Sync>;
type SharedExitHook = Arc<Mutex<Option<ExitHook>>>;

struct SupervisorState {
    requests: mpsc::Receiver<ChildRequest>,
    io: mpsc::Sender<IoCommand>,
    events: mpsc::Receiver<IoEvent>,
    io_thread: JoinHandle<()>,
    exited: Arc<AtomicBool>,
    exit_hook: SharedExitHook,
    ready: mpsc::SyncSender<Result<(), ProcessError>>,
}

/// The host-facing adapter. Its supervisor thread owns the `Child`, polls for
/// early exits, enforces deadlines, and always calls `wait()` after kill.
pub struct SupervisedProcessAdapter {
    package_id: String,
    requests: Option<mpsc::Sender<ChildRequest>>,
    supervisor: Option<JoinHandle<()>>,
    exited: Arc<AtomicBool>,
    exit_hook: SharedExitHook,
}

impl SupervisedProcessAdapter {
    pub fn spawn(spec: TrustedProcessSpec) -> Result<Self, ProcessError> {
        verify_executable(&spec)?;
        let mut command = isolated_package_command(&spec.executable);
        command
            .args(&spec.arguments)
            .current_dir(
                spec.executable
                    .parent()
                    .ok_or(ProcessError::UntrustedExecutable)?,
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|_| ProcessError::SpawnFailed)?;
        let input = child.stdout.take().ok_or(ProcessError::SpawnFailed)?;
        let output = child.stdin.take().ok_or(ProcessError::SpawnFailed)?;
        if let Some(stderr) = child.stderr.take() {
            drain_stderr(stderr);
        }

        let (requests_tx, requests_rx) = mpsc::channel::<ChildRequest>();
        let (io_tx, io_rx) = mpsc::channel::<IoCommand>();
        let (events_tx, events_rx) = mpsc::channel::<IoEvent>();
        let package_id = spec.package_id.clone();
        let io_package_id = package_id.clone();
        let io_thread = thread::Builder::new()
            .name("rb-package-io".into())
            .spawn(move || {
                let mut runner = match ProcessRunner::handshake(&io_package_id, input, output) {
                    Ok(runner) => {
                        let _ = events_tx.send(IoEvent::Handshake(Ok(())));
                        runner
                    }
                    Err(_) => {
                        let _ = events_tx.send(IoEvent::Handshake(Err(AdapterError::Unavailable(
                            "package handshake failed".into(),
                        ))));
                        return;
                    }
                };
                while let Ok(command) = io_rx.recv() {
                    match command {
                        IoCommand::Request(envelope) => {
                            let result = runner.handle(envelope);
                            if events_tx.send(IoEvent::Reply(result)).is_err() {
                                break;
                            }
                        }
                        IoCommand::Stop => break,
                    }
                }
            })
            .map_err(|_| {
                terminate_and_reap(&mut child);
                ProcessError::SpawnFailed
            })?;

        let exited = Arc::new(AtomicBool::new(false));
        let exit_hook: SharedExitHook = Arc::new(Mutex::new(None));
        let supervisor_exit = exited.clone();
        let supervisor_hook = exit_hook.clone();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let child_slot = Arc::new(Mutex::new(Some(child)));
        let io_thread_slot = Arc::new(Mutex::new(Some(io_thread)));
        let worker_child = child_slot.clone();
        let worker_io_thread = io_thread_slot.clone();
        let supervisor = match thread::Builder::new()
            .name("rb-package-supervisor".into())
            .spawn(move || {
                let child = worker_child.lock().ok().and_then(|mut slot| slot.take());
                let io_thread = worker_io_thread
                    .lock()
                    .ok()
                    .and_then(|mut slot| slot.take());
                if let (Some(child), Some(io_thread)) = (child, io_thread) {
                    supervise_child(
                        child,
                        SupervisorState {
                            requests: requests_rx,
                            io: io_tx,
                            events: events_rx,
                            io_thread,
                            exited: supervisor_exit,
                            exit_hook: supervisor_hook,
                            ready: ready_tx,
                        },
                    );
                }
            }) {
            Ok(supervisor) => supervisor,
            Err(_) => {
                if let Ok(mut slot) = child_slot.lock() {
                    if let Some(mut child) = slot.take() {
                        terminate_and_reap(&mut child);
                    }
                }
                drop(requests_tx);
                if let Ok(mut slot) = io_thread_slot.lock() {
                    if let Some(io_thread) = slot.take() {
                        let _ = io_thread.join();
                    }
                }
                return Err(ProcessError::SpawnFailed);
            }
        };

        // The constructor confirms the protocol handshake within the same
        // bounded control deadline before exposing the adapter to the host.
        match ready_rx.recv_timeout(CONTROL_DEADLINE + CHILD_POLL * 5) {
            Ok(Ok(())) => Ok(Self {
                package_id,
                requests: Some(requests_tx),
                supervisor: Some(supervisor),
                exited,
                exit_hook,
            }),
            Ok(Err(error)) => {
                drop(requests_tx);
                let _ = supervisor.join();
                Err(error)
            }
            Err(_) => {
                drop(requests_tx);
                let _ = supervisor.join();
                Err(ProcessError::TimedOut)
            }
        }
    }
}

impl UniversalPackageAdapter for SupervisedProcessAdapter {
    fn package_id(&self) -> &str {
        &self.package_id
    }

    fn handle(&mut self, envelope: AdapterEnvelope) -> Result<AdapterReply, AdapterError> {
        if self.exited.load(Ordering::Acquire) {
            return Err(AdapterError::Unavailable("package process exited".into()));
        }
        let timeout = match &envelope.body {
            rust_boy_adapter_sdk::AdapterCommand::Intent { action, .. }
                if action == "music.scan" =>
            {
                SCAN_DEADLINE
            }
            _ => CONTROL_DEADLINE,
        };
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        self.requests
            .as_ref()
            .ok_or_else(|| AdapterError::Unavailable("package supervisor stopped".into()))?
            .send(ChildRequest {
                envelope,
                reply: reply_tx,
            })
            .map_err(|_| AdapterError::Unavailable("package supervisor stopped".into()))?;
        reply_rx
            .recv_timeout(timeout + CHILD_POLL * 4)
            .map_err(|_| AdapterError::Unavailable("package process deadline exceeded".into()))?
    }

    fn set_process_exit_hook(&mut self, hook: Arc<dyn Fn() + Send + Sync>) {
        if let Ok(mut slot) = self.exit_hook.lock() {
            if self.exited.load(Ordering::Acquire) {
                drop(slot);
                hook();
            } else {
                *slot = Some(hook);
            }
        }
    }

    fn is_supervised_process(&self) -> bool {
        true
    }
}

impl Drop for SupervisedProcessAdapter {
    fn drop(&mut self) {
        // Disconnecting the command sender causes the supervisor to terminate,
        // reap and join the child rather than detach a package process.
        self.requests.take();
        if let Some(supervisor) = self.supervisor.take() {
            let _ = supervisor.join();
        }
    }
}

fn supervise_child(mut child: Child, state: SupervisorState) {
    let SupervisorState {
        requests,
        io,
        events,
        io_thread,
        exited,
        exit_hook,
        ready,
    } = state;
    let handshake_deadline = Instant::now() + CONTROL_DEADLINE;
    loop {
        if child.try_wait().ok().flatten().is_some() {
            let _ = ready.send(Err(ProcessError::Exited));
            notify_exit(&exited, &exit_hook);
            let _ = io_thread.join();
            return;
        }
        if Instant::now() >= handshake_deadline {
            terminate_and_reap(&mut child);
            let _ = ready.send(Err(ProcessError::TimedOut));
            notify_exit(&exited, &exit_hook);
            let _ = io_thread.join();
            return;
        }
        match events.recv_timeout(CHILD_POLL) {
            Ok(IoEvent::Handshake(Ok(()))) => {
                let _ = ready.send(Ok(()));
                break;
            }
            Ok(IoEvent::Handshake(Err(_))) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                terminate_and_reap(&mut child);
                let _ = ready.send(Err(ProcessError::HandshakeFailed));
                notify_exit(&exited, &exit_hook);
                let _ = io_thread.join();
                return;
            }
            Ok(IoEvent::Reply(_)) | Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }

    loop {
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => {
                let _ = child.wait();
                notify_exit(&exited, &exit_hook);
                break;
            }
            Ok(None) => {}
        }
        let request = match requests.recv_timeout(CHILD_POLL) {
            Ok(request) => request,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                terminate_and_reap(&mut child);
                notify_exit(&exited, &exit_hook);
                break;
            }
        };
        let is_shutdown = matches!(
            request.envelope.body,
            rust_boy_adapter_sdk::AdapterCommand::Shutdown
        );
        let deadline = Instant::now()
            + if is_scan(&request.envelope) {
                SCAN_DEADLINE
            } else {
                CONTROL_DEADLINE
            };
        if io.send(IoCommand::Request(request.envelope)).is_err() {
            let _ = request.reply.send(Err(AdapterError::Unavailable(
                "package protocol worker stopped".into(),
            )));
            terminate_and_reap(&mut child);
            notify_exit(&exited, &exit_hook);
            break;
        }
        let reply = loop {
            if child.try_wait().ok().flatten().is_some() {
                break Err(AdapterError::Unavailable("package process exited".into()));
            }
            let now = Instant::now();
            if now >= deadline {
                break Err(AdapterError::Unavailable(
                    "package process deadline exceeded".into(),
                ));
            }
            match events.recv_timeout((deadline - now).min(CHILD_POLL)) {
                Ok(IoEvent::Reply(reply)) => break reply,
                Ok(IoEvent::Handshake(_)) => {
                    break Err(AdapterError::Fault(
                        "unexpected package handshake event".into(),
                    ))
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break Err(AdapterError::Unavailable(
                        "package protocol worker stopped".into(),
                    ))
                }
            }
        };

        if reply.is_err() {
            terminate_and_reap(&mut child);
            notify_exit(&exited, &exit_hook);
            let _ = request.reply.send(reply);
            break;
        }
        if is_shutdown {
            // Ack is not enough: keep the host lease until the child has exited
            // or has been terminated and reaped.
            let exit_deadline = Instant::now() + CONTROL_DEADLINE;
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        let _ = child.wait();
                        break;
                    }
                    Ok(None) if Instant::now() < exit_deadline => thread::sleep(CHILD_POLL),
                    _ => {
                        terminate_and_reap(&mut child);
                        break;
                    }
                }
            }
            notify_exit(&exited, &exit_hook);
            let _ = request.reply.send(reply);
            break;
        }
        let _ = request.reply.send(reply);
    }
    let _ = io.send(IoCommand::Stop);
    let _ = io_thread.join();
}

fn is_scan(envelope: &AdapterEnvelope) -> bool {
    matches!(&envelope.body, rust_boy_adapter_sdk::AdapterCommand::Intent { action, .. } if action == "music.scan")
}

fn notify_exit(exited: &AtomicBool, hook: &Mutex<Option<ExitHook>>) {
    if !exited.swap(true, Ordering::AcqRel) {
        if let Ok(mut slot) = hook.lock() {
            if let Some(hook) = slot.take() {
                hook();
            }
        }
    }
}

fn terminate_and_reap(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}

fn drain_stderr<R: Read + Send + 'static>(mut stderr: R) {
    let _ = thread::Builder::new()
        .name("rb-package-stderr".into())
        .spawn(move || {
            let mut buf = [0u8; 1024];
            let mut drained = 0usize;
            loop {
                match stderr.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        drained = drained.saturating_add(n);
                        if drained >= STDERR_DRAIN_BYTES {
                            drained = 0;
                        }
                    }
                }
            }
        });
}

fn verify_executable(spec: &TrustedProcessSpec) -> Result<(), ProcessError> {
    if !spec.executable.is_absolute() {
        return Err(ProcessError::UntrustedExecutable);
    }
    let metadata =
        fs::symlink_metadata(&spec.executable).map_err(|_| ProcessError::UntrustedExecutable)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > EXECUTABLE_MAX_BYTES
    {
        return Err(ProcessError::UntrustedExecutable);
    }
    let mut file = File::open(&spec.executable).map_err(|_| ProcessError::UntrustedExecutable)?;
    let mut context = Context::new(&SHA256);
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|_| ProcessError::UntrustedExecutable)?;
        if n == 0 {
            break;
        }
        context.update(&buf[..n]);
    }
    if context.finish().as_ref() != spec.expected_sha256 {
        return Err(ProcessError::UntrustedExecutable);
    }
    Ok(())
}

fn decode_sha256(value: &str) -> Result<[u8; 32], ProcessError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(ProcessError::InvalidRoster);
    }
    let mut out = [0u8; 32];
    let (pairs, _) = value.as_bytes().as_chunks::<2>();
    for (index, pair) in pairs.iter().enumerate() {
        let hi = (pair[0] as char)
            .to_digit(16)
            .ok_or(ProcessError::InvalidRoster)? as u8;
        let lo = (pair[1] as char)
            .to_digit(16)
            .ok_or(ProcessError::InvalidRoster)? as u8;
        out[index] = (hi << 4) | lo;
    }
    Ok(out)
}

fn validate_package_id(value: &str) -> Result<(), ProcessError> {
    let mut parts = value.split('.');
    if parts.clone().count() < 3
        || value.len() > 96
        || parts.any(|part| {
            part.is_empty()
                || part.len() > 32
                || !part.as_bytes()[0].is_ascii_lowercase()
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
    {
        return Err(ProcessError::UntrustedExecutable);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_focus::{AudioFocusBroker, AudioModule};
    use crate::universal_adapter::{AdapterState, UniversalAdapterHost, Viewport};
    use serde_json::Value;

    #[test]
    fn roster_denies_unknown_modules_and_requires_reviewed_paths() {
        assert!(approved_process_ids().unwrap().is_empty());
        assert!(matches!(
            approved_process_install("org.rustboy.music", Path::new("/tmp")),
            Err(ProcessError::UntrustedExecutable)
        ));
        let entry = EmbeddedExecutable {
            package_id: "org.rustboy.music".into(),
            relative_executable: "../evil".into(),
            sha256: "0".repeat(64),
            relative_manifest: "../manifest.json".into(),
            manifest_sha256: "0".repeat(64),
            arguments: vec![],
        };
        assert!(matches!(
            TrustedProcessSpec::from_roster_entry(&entry, Path::new("/tmp")),
            Err(ProcessError::UntrustedExecutable)
        ));
    }

    #[test]
    fn package_install_root_is_namespaced_and_rejects_symlinks() {
        let packages = tempfile::tempdir().unwrap();
        let package_root = packages.path().join("org.rustboy.music");
        fs::create_dir(&package_root).unwrap();
        assert_eq!(
            resolve_package_root(packages.path(), "org.rustboy.music").unwrap(),
            fs::canonicalize(&package_root).unwrap()
        );
        assert_eq!(
            resolve_package_root(packages.path(), "org.rustboy.tv"),
            Err(ProcessError::UntrustedExecutable)
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let outside = tempfile::tempdir().unwrap();
            symlink(outside.path(), packages.path().join("org.rustboy.tv")).unwrap();
            assert_eq!(
                resolve_package_root(packages.path(), "org.rustboy.tv"),
                Err(ProcessError::UntrustedExecutable)
            );
        }
    }

    #[test]
    fn executable_hash_pin_rejects_replacement_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("module");
        fs::write(&path, b"approved-bytes").unwrap();
        let spec = TrustedProcessSpec::for_test("org.rustboy.music", path.clone(), vec![]);
        verify_executable(&spec).unwrap();
        fs::write(path, b"replacement-bytes").unwrap();
        assert_eq!(
            verify_executable(&spec),
            Err(ProcessError::UntrustedExecutable)
        );
    }

    #[test]
    fn approved_install_requires_matching_executable_and_manifest_pins() {
        use ring::digest::digest;

        let dir = tempfile::tempdir().unwrap();
        let package_root = dir.path().join("org.rustboy.music");
        fs::create_dir(&package_root).unwrap();
        let executable_dir = package_root.join("bin");
        fs::create_dir(&executable_dir).unwrap();
        let executable = executable_dir.join("rust-music");
        fs::write(&executable, b"approved executable bytes").unwrap();
        let manifest_path = package_root.join("manifest.json");
        let mut manifest: Value = serde_json::from_str(include_str!(
            "../../../catalog/host_packages/rust_music.example.json"
        ))
        .unwrap();
        manifest.as_object_mut().unwrap().remove("status");
        manifest["entrypoint"]["type"] = Value::String("external_process".into());
        let manifest_json = serde_json::to_string(&manifest).unwrap();
        fs::write(&manifest_path, &manifest_json).unwrap();
        let executable_hash = digest(&SHA256, b"approved executable bytes");
        let manifest_hash = digest(&SHA256, manifest_json.as_bytes());
        let entry = EmbeddedExecutable {
            package_id: "org.rustboy.music".into(),
            relative_executable: "bin/rust-music".into(),
            sha256: executable_hash
                .as_ref()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            relative_manifest: "manifest.json".into(),
            manifest_sha256: manifest_hash
                .as_ref()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            arguments: vec!["serve".into()],
        };

        let (spec, resolved_manifest) =
            resolve_approved_install_entry(&entry, &package_root).unwrap();
        assert_eq!(spec.package_id(), "org.rustboy.music");
        assert_eq!(resolved_manifest, manifest_json);

        let mut tampered_value: Value = serde_json::from_str(&resolved_manifest).unwrap();
        tampered_value["name"] = Value::String("OTHER".into());
        let tampered_manifest = serde_json::to_string(&tampered_value).unwrap();
        assert_ne!(tampered_manifest, resolved_manifest);
        fs::write(&manifest_path, tampered_manifest).unwrap();
        assert_eq!(
            resolve_approved_install_entry(&entry, &package_root).unwrap_err(),
            ProcessError::UntrustedExecutable
        );
    }

    #[cfg(unix)]
    #[test]
    fn approved_install_rejects_symlinked_manifest() {
        use ring::digest::digest;
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let package_root = dir.path().join("org.rustboy.music");
        fs::create_dir(&package_root).unwrap();
        let executable = package_root.join("module");
        fs::write(&executable, b"executable").unwrap();
        let manifest = serde_json::from_str::<Value>(include_str!(
            "../../../catalog/host_packages/rust_music.example.json"
        ))
        .unwrap();
        let manifest_json = serde_json::to_string(&manifest).unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        fs::write(outside.path(), &manifest_json).unwrap();
        symlink(outside.path(), package_root.join("manifest.json")).unwrap();
        let entry = EmbeddedExecutable {
            package_id: "org.rustboy.music".into(),
            relative_executable: "module".into(),
            sha256: digest(&SHA256, b"executable")
                .as_ref()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            relative_manifest: "manifest.json".into(),
            manifest_sha256: digest(&SHA256, manifest_json.as_bytes())
                .as_ref()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            arguments: vec![],
        };
        assert_eq!(
            resolve_approved_install_entry(&entry, &package_root).unwrap_err(),
            ProcessError::UntrustedExecutable
        );
    }

    #[cfg(unix)]
    fn python_executable() -> PathBuf {
        let path = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|dir| dir.join("python3"))
            .find(|candidate| candidate.is_file())
            .expect("Python 3 is available in the developer and CI toolchain");
        fs::canonicalize(path).unwrap()
    }

    #[cfg(unix)]
    fn fixture_spec(mode: &str) -> TrustedProcessSpec {
        // Test-only fixed protocol peer. Production selection never accepts
        // interpreter source, PATH lookup, or arbitrary runtime arguments.
        let script = format!(
            r#"import struct,sys,time
pkg=b'org.rustboy.music'
def readf():
 h=sys.stdin.buffer.read(8)
 if len(h)!=8:return None
 return sys.stdin.buffer.read(struct.unpack('>I',h[4:])[0])
def send(i,b):
 p=bytes([1,2])+i.to_bytes(8,'big')+struct.pack('>H',len(pkg))+pkg+b
 sys.stdout.buffer.write(b'RBPK'+struct.pack('>I',len(p))+p);sys.stdout.buffer.flush()
if readf() is None:sys.exit(2)
send(1,bytes([0,0,1]))
while True:
 p=readf()
 if p is None:sys.exit(0)
 i=int.from_bytes(p[2:10],'big');n=int.from_bytes(p[10:12],'big');tag=p[12+n]
 if {mode:?}=='exit_on_activate' and tag==1:sys.exit(0)
 if {mode:?}=='hang_on_activate' and tag==1:time.sleep(60)
 if {mode:?}=='hang_on_shutdown' and tag==12:time.sleep(60)
 send(i,bytes([1]))
 if tag==12:sys.exit(0)
"#
        );
        TrustedProcessSpec::for_test(
            "org.rustboy.music",
            python_executable(),
            vec!["-c".into(), script],
        )
    }

    #[cfg(unix)]
    fn trusted_music_manifest() -> String {
        let mut manifest: Value = serde_json::from_str(include_str!(
            "../../../catalog/host_packages/rust_music.example.json"
        ))
        .unwrap();
        manifest.as_object_mut().unwrap().remove("status");
        manifest["entrypoint"]["type"] = Value::String("external_process".into());
        manifest.to_string()
    }

    #[cfg(unix)]
    #[test]
    fn actual_rust_music_child_runs_under_host_supervisor_when_configured() {
        let Some(executable) = std::env::var_os("RUST_MUSIC_ADAPTER_BIN") else {
            eprintln!("skipping real-module integration; set RUST_MUSIC_ADAPTER_BIN");
            return;
        };
        let executable = fs::canonicalize(executable).expect("Rust Music executable exists");
        let spec =
            TrustedProcessSpec::for_test("org.rustboy.music", executable, vec!["serve".into()]);
        let broker = AudioFocusBroker::default();
        let mut host = UniversalAdapterHost::with_audio_focus(
            Viewport::new(480, 320).unwrap(),
            broker.clone(),
        );
        host.mount_approved_process(&trusted_music_manifest(), spec)
            .expect("real module completes supervised handshake");
        host.select("org.rustboy.music")
            .expect("real module activates without playback");
        assert_eq!(broker.owner(), Some(AudioModule::Music));
        assert!(host.foreground_view().is_some());
        host.shutdown("org.rustboy.music")
            .expect("real module acknowledges shutdown and exits");
        assert_eq!(broker.owner(), None, "reaped child releases host lease");
    }

    #[cfg(unix)]
    #[test]
    fn shutdown_reaps_child_before_releasing_host_audio_lease() {
        let broker = AudioFocusBroker::default();
        let spec = fixture_spec("normal");
        let mut host = UniversalAdapterHost::with_audio_focus(
            Viewport::new(480, 320).unwrap(),
            broker.clone(),
        );
        host.mount_approved_process(&trusted_music_manifest(), spec)
            .unwrap();
        host.select("org.rustboy.music").unwrap();
        assert_eq!(broker.owner(), Some(AudioModule::Music));
        host.intent(
            "ui.music.next",
            None,
            serde_json::json!({"index": 0}),
            &crate::universal_adapter::DenyAllScopes,
        )
        .expect("host routes UI input through the supervised process protocol");
        assert_eq!(
            host.state("org.rustboy.music"),
            Some(AdapterState::Foreground)
        );
        host.shutdown("org.rustboy.music").unwrap();
        assert_eq!(broker.owner(), None);
    }

    #[cfg(unix)]
    #[test]
    fn timed_out_child_is_killed_reaped_and_releases_audio_lease() {
        let broker = AudioFocusBroker::default();
        let spec = fixture_spec("hang_on_activate");
        let mut host = UniversalAdapterHost::with_audio_focus(
            Viewport::new(480, 320).unwrap(),
            broker.clone(),
        );
        host.mount_approved_process(&trusted_music_manifest(), spec)
            .unwrap();
        let start = Instant::now();
        assert!(host.select("org.rustboy.music").is_err());
        assert!(start.elapsed() < CONTROL_DEADLINE + Duration::from_secs(2));
        assert_eq!(broker.owner(), None);
        assert_eq!(host.state("org.rustboy.music"), Some(AdapterState::Faulted));
    }

    #[cfg(unix)]
    #[test]
    fn unexpected_exit_releases_audio_lease_without_ui_polling() {
        let broker = AudioFocusBroker::default();
        let spec = fixture_spec("exit_on_activate");
        let mut host = UniversalAdapterHost::with_audio_focus(
            Viewport::new(480, 320).unwrap(),
            broker.clone(),
        );
        host.mount_approved_process(&trusted_music_manifest(), spec)
            .unwrap();
        assert!(host.select("org.rustboy.music").is_err());
        let deadline = Instant::now() + CONTROL_DEADLINE;
        while broker.owner().is_some() && Instant::now() < deadline {
            thread::sleep(CHILD_POLL);
        }
        assert_eq!(broker.owner(), None);
    }

    #[cfg(unix)]
    #[test]
    fn failed_shutdown_drop_kills_child_then_releases_audio_claim() {
        let broker = AudioFocusBroker::default();
        let spec = fixture_spec("hang_on_shutdown");
        let mut host = UniversalAdapterHost::with_audio_focus(
            Viewport::new(480, 320).unwrap(),
            broker.clone(),
        );
        host.mount_approved_process(&trusted_music_manifest(), spec)
            .unwrap();
        host.select("org.rustboy.music").unwrap();
        assert_eq!(broker.owner(), Some(AudioModule::Music));
        assert!(host.shutdown("org.rustboy.music").is_err());
        drop(host);
        assert_eq!(broker.owner(), None);
    }
}
