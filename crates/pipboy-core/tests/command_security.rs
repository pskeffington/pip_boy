use pipboy_core::command_catalog::{
    parse_input, CommandError, CommandFailure, CommandResult, CommandState, HandlerId,
    TerminalAction,
};
use pipboy_core::package_socket::reference_catalog;
use pipboy_core::universal_adapter::{
    AdapterCommand, AdapterEnvelope, AdapterError, AdapterReply, DenyAllScopes, ScopeAuthority,
    UniversalAdapterHost, UniversalPackageAdapter, Viewport,
};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

struct Recorder {
    id: String,
    calls: Arc<Mutex<Vec<AdapterCommand>>>,
}

impl UniversalPackageAdapter for Recorder {
    fn package_id(&self) -> &str {
        &self.id
    }

    fn handle(&mut self, request: AdapterEnvelope) -> Result<AdapterReply, AdapterError> {
        self.calls.lock().unwrap().push(request.body);
        Ok(AdapterReply::Ack)
    }
}

fn fixture_manifest(package_id: &str, include_design_status: bool) -> Value {
    let registry = reference_catalog().unwrap();
    let mut manifest = serde_json::to_value(registry.get(package_id).unwrap()).unwrap();
    if !include_design_status {
        manifest.as_object_mut().unwrap().remove("status");
    }
    manifest
}

fn recorder(
    id: &str,
) -> (
    Box<dyn UniversalPackageAdapter>,
    Arc<Mutex<Vec<AdapterCommand>>>,
) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    (
        Box::new(Recorder {
            id: id.to_owned(),
            calls: calls.clone(),
        }),
        calls,
    )
}

#[test]
fn parser_denies_unpinned_invalid_prompt_copy_and_sensitive_echo() {
    const PINNED: &[&str] = &["host"];
    assert_eq!(
        parse_input("select net-lab", None, PINNED),
        Err(CommandError::ToolNotPinned)
    );
    assert_eq!(
        parse_input("select radio", None, PINNED),
        Err(CommandError::UnknownTool)
    );
    assert_eq!(
        parse_input("run shell secret-token", Some("host"), PINNED),
        Err(CommandError::UnsupportedCommand)
    );
    assert_eq!(
        parse_input("rb:host> run status", Some("host"), PINNED),
        Err(CommandError::UnsupportedCommand)
    );
    assert_eq!(
        parse_input(
            &"x".repeat(pipboy_core::command_catalog::MAX_COMMAND_BYTES + 1),
            Some("host"),
            PINNED
        ),
        Err(CommandError::Oversized)
    );
    assert_eq!(
        parse_input("run status\u{0000}", Some("host"), PINNED),
        Err(CommandError::InvalidInput)
    );

    let private = "PRIVATE-MESSAGE-CONTENT";
    let error = parse_input(&format!("run {private}"), Some("host"), PINNED).unwrap_err();
    assert!(!error.to_string().contains(private));
}

#[test]
fn design_manifests_invalid_json_and_external_launchers_never_mount() {
    let mut host = UniversalAdapterHost::new(Viewport::new(480, 320).unwrap());
    for id in ["org.rustboy.tv", "org.rustboy.comms"] {
        let (runner, calls) = recorder(id);
        let design = fixture_manifest(id, true).to_string();
        assert!(host.mount_trusted(&design, runner).is_err());
        assert!(calls.lock().unwrap().is_empty());

        let (runner, calls) = recorder(id);
        let mut external = fixture_manifest(id, false);
        external["entrypoint"]["type"] = json!("external_process");
        assert!(host.mount_trusted(&external.to_string(), runner).is_err());
        assert!(calls.lock().unwrap().is_empty());
    }
    let (runner, calls) = recorder("org.rustboy.comms");
    assert!(host.mount_trusted("{invalid json", runner).is_err());
    assert!(calls.lock().unwrap().is_empty());
    assert!(host.is_empty());
}

#[derive(Default)]
struct RevocableCommsGrant(AtomicBool);

impl ScopeAuthority for RevocableCommsGrant {
    fn is_granted(&self, package_id: &str, scope: &str) -> bool {
        package_id == "org.rustboy.comms"
            && scope == "network.connect.explicit"
            && self.0.load(Ordering::SeqCst)
    }
}

#[test]
fn comms_scope_is_checked_on_each_intent_and_revocation_stops_delivery() {
    let id = "org.rustboy.comms";
    let mut host = UniversalAdapterHost::new(Viewport::new(480, 320).unwrap());
    let (runner, calls) = recorder(id);
    host.mount_trusted(&fixture_manifest(id, false).to_string(), runner)
        .unwrap();
    host.select(id).unwrap();
    let grant = RevocableCommsGrant::default();
    let message = json!({"body": "PRIVATE-MESSAGE-CONTENT"});

    assert!(host
        .intent(
            "comms.connect",
            Some("network.connect.explicit"),
            message.clone(),
            &DenyAllScopes
        )
        .is_err());
    assert_eq!(
        calls.lock().unwrap().len(),
        1,
        "only activation may reach the adapter"
    );

    grant.0.store(true, Ordering::SeqCst);
    host.intent(
        "comms.connect",
        Some("network.connect.explicit"),
        message.clone(),
        &grant,
    )
    .unwrap();
    assert_eq!(calls.lock().unwrap().len(), 2);

    grant.0.store(false, Ordering::SeqCst);
    let denied = host
        .intent(
            "comms.connect",
            Some("network.connect.explicit"),
            message,
            &grant,
        )
        .unwrap_err();
    assert_eq!(
        calls.lock().unwrap().len(),
        2,
        "revoked intent must not be delivered"
    );
    assert!(!denied.to_string().contains("PRIVATE-MESSAGE-CONTENT"));
}

#[test]
fn uncertain_delivery_is_explicitly_not_success() {
    let result = CommandResult::<()> {
        handler: HandlerId::NetworkRoute,
        state: CommandState::UnknownOutcome,
        source: "crates/pipboy-core/src/network_suite.rs",
        observed_at_ms: 7,
        value: None,
        error: Some(CommandFailure::UnknownOutcome),
    };
    assert_ne!(result.state, CommandState::Succeeded);
    let value = serde_json::to_value(result).unwrap();
    assert_eq!(value["state"], "unknown_outcome");
    assert_eq!(value["error"], "unknown_outcome");
    let safe_message = CommandFailure::UnknownOutcome.safe_message();
    assert_eq!(safe_message, "Command outcome could not be verified");
    assert!(!safe_message.contains("PRIVATE-MESSAGE-CONTENT"));
}

#[test]
fn tool_selection_is_a_control_action_without_dispatch_payload() {
    let action = parse_input("select host", None, &["host"]).unwrap();
    assert!(matches!(
        action,
        TerminalAction::SelectTool { tool_id: "host" }
    ));
}
