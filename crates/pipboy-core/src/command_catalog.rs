//! Shared typed command vocabulary for the terminal, TUI, and graphical shell.
//!
//! This module parses requests and maps them to stable native handler IDs. It
//! never starts a process or performs I/O. Frontends must pass parsed requests
//! to the approved Rust executor after the host policy check.

use serde::Serialize;
use std::fmt;

pub const MAX_COMMAND_BYTES: usize = 512;
pub const MAX_ARGUMENT_BYTES: usize = 256;
pub const MAX_QUERY_WORDS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlerId {
    HostStatus,
    HostDiagnostics,
    SourceSearch,
    BytesEncode,
    HexInspect,
    HexCrc32,
    NetworkList,
    NetworkCidr4,
    NetworkCidr6,
    NetworkSplit4,
    NetworkRoute,
    NetworkAggregate4,
    NetworkOverlap4,
    NetworkMac,
    NetworkVlan,
    NetworkMtu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgumentSchema {
    None,
    OptionalText,
    Text,
    HexBytes,
    NetworkInput,
    NetworkInputAndExtra,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectClass {
    ReadOnly,
    LocalWrite,
    Network,
    Device,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectivityState {
    Offline,
    Local,
    Network,
}

/// Stable descriptor; `handler` is an enum identity, never a display command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CommandDescriptor {
    pub tool_id: &'static str,
    pub verb: &'static str,
    pub handler: HandlerId,
    pub source: &'static str,
    pub arguments: ArgumentSchema,
    pub discoverable: bool,
    pub side_effect: SideEffectClass,
    pub required_scope: Option<&'static str>,
    pub confirmation_required: bool,
    pub connectivity: ConnectivityState,
}

pub const COMMANDS: &[CommandDescriptor] = &[
    descriptor(
        "host",
        "status",
        HandlerId::HostStatus,
        ArgumentSchema::None,
    ),
    descriptor(
        "diag",
        "host",
        HandlerId::HostDiagnostics,
        ArgumentSchema::None,
    ),
    descriptor(
        "library",
        "sources",
        HandlerId::SourceSearch,
        ArgumentSchema::OptionalText,
    ),
    descriptor(
        "byte-lab",
        "encode",
        HandlerId::BytesEncode,
        ArgumentSchema::Text,
    ),
    descriptor(
        "byte-lab",
        "inspect",
        HandlerId::HexInspect,
        ArgumentSchema::HexBytes,
    ),
    descriptor(
        "byte-lab",
        "crc32",
        HandlerId::HexCrc32,
        ArgumentSchema::HexBytes,
    ),
    descriptor(
        "net-lab",
        "list",
        HandlerId::NetworkList,
        ArgumentSchema::None,
    ),
    descriptor(
        "net-lab",
        "cidr",
        HandlerId::NetworkCidr4,
        ArgumentSchema::NetworkInput,
    ),
    descriptor(
        "net-lab",
        "cidr6",
        HandlerId::NetworkCidr6,
        ArgumentSchema::NetworkInput,
    ),
    descriptor(
        "net-lab",
        "split4",
        HandlerId::NetworkSplit4,
        ArgumentSchema::NetworkInputAndExtra,
    ),
    descriptor(
        "net-lab",
        "route",
        HandlerId::NetworkRoute,
        ArgumentSchema::NetworkInputAndExtra,
    ),
    descriptor(
        "net-lab",
        "aggregate4",
        HandlerId::NetworkAggregate4,
        ArgumentSchema::NetworkInput,
    ),
    descriptor(
        "net-lab",
        "overlap4",
        HandlerId::NetworkOverlap4,
        ArgumentSchema::NetworkInputAndExtra,
    ),
    descriptor(
        "net-lab",
        "mac",
        HandlerId::NetworkMac,
        ArgumentSchema::NetworkInput,
    ),
    descriptor(
        "net-lab",
        "vlan",
        HandlerId::NetworkVlan,
        ArgumentSchema::NetworkInput,
    ),
    descriptor(
        "net-lab",
        "mtu",
        HandlerId::NetworkMtu,
        ArgumentSchema::NetworkInput,
    ),
];

const fn descriptor(
    tool_id: &'static str,
    verb: &'static str,
    handler: HandlerId,
    arguments: ArgumentSchema,
) -> CommandDescriptor {
    CommandDescriptor {
        tool_id,
        verb,
        handler,
        source: handler_source(handler),
        arguments,
        discoverable: true,
        side_effect: SideEffectClass::ReadOnly,
        required_scope: None,
        confirmation_required: false,
        connectivity: ConnectivityState::Offline,
    }
}

const fn handler_source(handler: HandlerId) -> &'static str {
    match handler {
        HandlerId::HostStatus => "crates/pipboy/src/status.rs",
        HandlerId::HostDiagnostics => "crates/pipboy/src/diagnostics.rs",
        HandlerId::SourceSearch => "crates/pipboy-core/src/catalog.rs",
        HandlerId::BytesEncode | HandlerId::HexInspect | HandlerId::HexCrc32 => {
            "crates/pipboy-core/src/workbench.rs"
        }
        HandlerId::NetworkList
        | HandlerId::NetworkCidr4
        | HandlerId::NetworkCidr6
        | HandlerId::NetworkSplit4
        | HandlerId::NetworkRoute
        | HandlerId::NetworkAggregate4
        | HandlerId::NetworkOverlap4
        | HandlerId::NetworkMac
        | HandlerId::NetworkVlan
        | HandlerId::NetworkMtu => "crates/pipboy-core/src/network_suite.rs",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkOperation {
    Cidr4,
    Cidr6,
    Split4,
    Route,
    Aggregate4,
    Overlap4,
    Mac,
    Vlan,
    Mtu,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypedArguments {
    None,
    Text {
        value: String,
    },
    Hex {
        value: String,
    },
    Network {
        operation: NetworkOperation,
        input: String,
        extra: Option<String>,
    },
}

/// A request can only be obtained by parsing a registered, pinned command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandRequest {
    pub tool_id: &'static str,
    pub verb: &'static str,
    pub handler: HandlerId,
    pub arguments: TypedArguments,
    pub side_effect: SideEffectClass,
    pub required_scope: Option<&'static str>,
    pub confirmation_required: bool,
    pub connectivity: ConnectivityState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandState {
    Unavailable,
    NotInstalled,
    Disconnected,
    NeedsAuth,
    Ready,
    Running,
    Succeeded,
    Failed,
    UnknownOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandFailure {
    Denied,
    Unavailable,
    InvalidInput,
    Failed,
    UnknownOutcome,
}

impl CommandFailure {
    pub const fn safe_message(self) -> &'static str {
        match self {
            Self::Denied => "Command denied by host policy",
            Self::Unavailable => "Command handler is unavailable",
            Self::InvalidInput => "Command input is invalid",
            Self::Failed => "Command failed",
            Self::UnknownOutcome => "Command outcome could not be verified",
        }
    }
}

/// Shared structured result envelope. Callers supply the observation time so
/// the core catalog stays deterministic and testable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandResult<T> {
    pub handler: HandlerId,
    pub state: CommandState,
    pub source: &'static str,
    pub observed_at_ms: u64,
    pub value: Option<T>,
    pub error: Option<CommandFailure>,
}

impl<T> CommandResult<T> {
    pub fn succeeded(handler: HandlerId, observed_at_ms: u64, value: T) -> Self {
        Self {
            handler,
            state: CommandState::Succeeded,
            source: handler_source(handler),
            observed_at_ms,
            value: Some(value),
            error: None,
        }
    }

    pub fn failed(handler: HandlerId, observed_at_ms: u64, error: CommandFailure) -> Self {
        Self {
            handler,
            state: CommandState::Failed,
            source: handler_source(handler),
            observed_at_ms,
            value: None,
            error: Some(error),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalAction {
    Empty,
    ListTools(Vec<&'static str>),
    Help { tool_id: Option<&'static str> },
    SelectTool { tool_id: &'static str },
    Quit,
    Execute(CommandRequest),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandError {
    InvalidInput,
    Oversized,
    UnknownTool,
    ToolNotPinned,
    NoSelectedTool,
    UnsupportedCommand,
    InvalidArguments,
}

impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInput => "invalid command input",
            Self::Oversized => "command or argument exceeds the size limit",
            Self::UnknownTool => "tool has no registered command handlers",
            Self::ToolNotPinned => "tool is not pinned in the active workbench",
            Self::NoSelectedTool => "no tool selected; use tools and select TOOL",
            Self::UnsupportedCommand => "command is not registered for this tool",
            Self::InvalidArguments => "command arguments do not match the registered schema",
        })
    }
}

impl std::error::Error for CommandError {}

/// Parses bounded interactive input using only registered handlers and the
/// active workbench pin set. Selection and help are control actions, not calls.
pub fn parse_input(
    input: &str,
    selected_tool: Option<&str>,
    pinned_tools: &[&str],
) -> Result<TerminalAction, CommandError> {
    if input.len() > MAX_COMMAND_BYTES {
        return Err(CommandError::Oversized);
    }
    if input.chars().any(char::is_control) {
        return Err(CommandError::InvalidInput);
    }
    let tokens: Vec<&str> = input.split_whitespace().collect();
    match tokens.as_slice() {
        [] => Ok(TerminalAction::Empty),
        ["quit"] | ["exit"] => Ok(TerminalAction::Quit),
        ["tools"] => Ok(TerminalAction::ListTools(registered_pins(pinned_tools))),
        ["help"] => {
            let tool_id = selected_tool.map(validate_tool).transpose()?;
            if let Some(id) = tool_id {
                require_pinned(id, pinned_tools)?;
            }
            Ok(TerminalAction::Help { tool_id })
        }
        ["select", tool_id] => {
            let tool_id = validate_tool(tool_id)?;
            require_pinned(tool_id, pinned_tools)?;
            Ok(TerminalAction::SelectTool { tool_id })
        }
        ["run", rest @ ..] => {
            let tool_id = selected_tool.ok_or(CommandError::NoSelectedTool)?;
            validate_tool(tool_id)?;
            require_pinned(tool_id, pinned_tools)?;
            parse_registered(tool_id, rest)
        }
        [tool_id] if crate::workbench::tool(tool_id).is_some() => {
            let tool_id = validate_tool(tool_id)?;
            require_pinned(tool_id, pinned_tools)?;
            Ok(TerminalAction::SelectTool { tool_id })
        }
        [_] if selected_tool.is_none() => Err(CommandError::NoSelectedTool),
        _ => {
            let tool_id = selected_tool.ok_or(CommandError::NoSelectedTool)?;
            validate_tool(tool_id)?;
            require_pinned(tool_id, pinned_tools)?;
            parse_registered(tool_id, tokens.as_slice())
        }
    }
}

fn parse_registered(tool_id: &str, tokens: &[&str]) -> Result<TerminalAction, CommandError> {
    let (verb, args) = tokens.split_first().ok_or(CommandError::InvalidArguments)?;
    let descriptor = COMMANDS
        .iter()
        .find(|entry| entry.tool_id == tool_id && entry.verb == *verb)
        .ok_or(CommandError::UnsupportedCommand)?;
    let arguments = match descriptor.arguments {
        ArgumentSchema::None if args.is_empty() => TypedArguments::None,
        ArgumentSchema::OptionalText if args.len() <= MAX_QUERY_WORDS => {
            let value = args.join(" ");
            check_argument(&value, true)?;
            TypedArguments::Text { value }
        }
        ArgumentSchema::Text if !args.is_empty() => {
            let value = args.join(" ");
            check_argument(&value, false)?;
            TypedArguments::Text { value }
        }
        ArgumentSchema::HexBytes if args.len() == 1 => {
            check_argument(args[0], false)?;
            TypedArguments::Hex {
                value: args[0].to_owned(),
            }
        }
        ArgumentSchema::NetworkInput if args.len() == 1 => {
            check_argument(args[0], false)?;
            TypedArguments::Network {
                operation: network_operation(descriptor.handler)?,
                input: args[0].to_owned(),
                extra: None,
            }
        }
        ArgumentSchema::NetworkInputAndExtra if args.len() == 2 => {
            check_argument(args[0], false)?;
            check_argument(args[1], false)?;
            TypedArguments::Network {
                operation: network_operation(descriptor.handler)?,
                input: args[0].to_owned(),
                extra: Some(args[1].to_owned()),
            }
        }
        _ => return Err(CommandError::InvalidArguments),
    };
    Ok(TerminalAction::Execute(CommandRequest {
        tool_id: descriptor.tool_id,
        verb: descriptor.verb,
        handler: descriptor.handler,
        arguments,
        side_effect: descriptor.side_effect,
        required_scope: descriptor.required_scope,
        confirmation_required: descriptor.confirmation_required,
        connectivity: descriptor.connectivity,
    }))
}

fn network_operation(handler: HandlerId) -> Result<NetworkOperation, CommandError> {
    match handler {
        HandlerId::NetworkCidr4 => Ok(NetworkOperation::Cidr4),
        HandlerId::NetworkCidr6 => Ok(NetworkOperation::Cidr6),
        HandlerId::NetworkSplit4 => Ok(NetworkOperation::Split4),
        HandlerId::NetworkRoute => Ok(NetworkOperation::Route),
        HandlerId::NetworkAggregate4 => Ok(NetworkOperation::Aggregate4),
        HandlerId::NetworkOverlap4 => Ok(NetworkOperation::Overlap4),
        HandlerId::NetworkMac => Ok(NetworkOperation::Mac),
        HandlerId::NetworkVlan => Ok(NetworkOperation::Vlan),
        HandlerId::NetworkMtu => Ok(NetworkOperation::Mtu),
        _ => Err(CommandError::UnsupportedCommand),
    }
}

fn check_argument(value: &str, empty_allowed: bool) -> Result<(), CommandError> {
    if value.len() > MAX_ARGUMENT_BYTES {
        return Err(CommandError::Oversized);
    }
    if (!empty_allowed && value.is_empty()) || value.chars().any(char::is_control) {
        return Err(CommandError::InvalidArguments);
    }
    Ok(())
}

fn validate_tool(tool_id: &str) -> Result<&'static str, CommandError> {
    registered_tools()
        .iter()
        .copied()
        .find(|known| *known == tool_id)
        .ok_or(CommandError::UnknownTool)
}

fn require_pinned(tool_id: &str, pinned_tools: &[&str]) -> Result<(), CommandError> {
    if pinned_tools.contains(&tool_id) {
        Ok(())
    } else {
        Err(CommandError::ToolNotPinned)
    }
}

fn registered_pins(pinned_tools: &[&str]) -> Vec<&'static str> {
    registered_tools()
        .iter()
        .copied()
        .filter(|tool_id| pinned_tools.contains(tool_id))
        .collect()
}

pub fn registered_tools() -> &'static [&'static str] {
    &["host", "diag", "library", "byte-lab", "net-lab"]
}

/// Returns stable JSON: fixed descriptor order, field order, and enum spelling.
pub fn catalog_json() -> Result<String, serde_json::Error> {
    serde_json::to_string(COMMANDS)
}

/// Descriptors available to one pinned tool, in stable catalog order.
pub fn help_for(tool_id: &str) -> Vec<&'static CommandDescriptor> {
    COMMANDS
        .iter()
        .filter(|command| command.tool_id == tool_id && command.discoverable)
        .collect()
}

/// Executors implement only reviewed native handlers. Control actions cannot
/// reach this trait because they contain no `CommandRequest`.
pub trait CommandExecutor {
    type Output;
    type Error;

    fn execute(&mut self, request: CommandRequest) -> Result<Self::Output, Self::Error>;
}

#[derive(Debug, PartialEq, Eq)]
pub enum DispatchResult<T> {
    Control(TerminalAction),
    Executed(T),
}

pub fn dispatch<E: CommandExecutor>(
    action: TerminalAction,
    executor: &mut E,
) -> Result<DispatchResult<E::Output>, E::Error> {
    match action {
        TerminalAction::Execute(request) => executor.execute(request).map(DispatchResult::Executed),
        control => Ok(DispatchResult::Control(control)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CountExecutor(usize);

    impl CommandExecutor for CountExecutor {
        type Output = HandlerId;
        type Error = ();

        fn execute(&mut self, request: CommandRequest) -> Result<Self::Output, Self::Error> {
            self.0 += 1;
            Ok(request.handler)
        }
    }

    const PINNED: &[&str] = &["host", "byte-lab", "net-lab"];

    #[test]
    fn catalog_is_stable_and_contains_native_handler_ids_not_shell_commands() {
        let first = catalog_json().unwrap();
        let second = catalog_json().unwrap();
        assert_eq!(first, second);
        assert!(first.contains("\"handler\":\"host_status\""));
        assert!(first.contains("crates/pipboy/src/status.rs"));
        assert!(!first.contains("pipboy "));
        assert!(COMMANDS.iter().all(|command| {
            command.discoverable
                && command.side_effect == SideEffectClass::ReadOnly
                && command.required_scope.is_none()
                && !command.confirmation_required
                && command.connectivity == ConnectivityState::Offline
        }));
    }

    #[test]
    fn help_is_tool_scoped_and_exposes_native_source_provenance() {
        let help = help_for("net-lab");
        assert!(!help.is_empty());
        assert!(help.iter().all(|command| {
            command.tool_id == "net-lab"
                && command.source == "crates/pipboy-core/src/network_suite.rs"
        }));
        assert!(help_for("radio").is_empty());
    }

    #[test]
    fn command_results_keep_status_source_time_and_value_typed() {
        let result =
            CommandResult::succeeded(HandlerId::NetworkCidr4, 42, "10.0.0.0/24".to_owned());
        assert_eq!(result.state, CommandState::Succeeded);
        assert_eq!(result.source, "crates/pipboy-core/src/network_suite.rs");
        assert_eq!(result.observed_at_ms, 42);
        assert_eq!(result.value, Some("10.0.0.0/24".to_owned()));
        assert_eq!(result.error, None);
        let error = CommandResult::<()>::failed(HandlerId::HostStatus, 43, CommandFailure::Denied);
        assert_eq!(error.state, CommandState::Failed);
        assert_eq!(error.error, Some(CommandFailure::Denied));
        assert!(!error
            .error
            .unwrap()
            .safe_message()
            .contains("PRIVATE-MESSAGE-CONTENT"));
        assert!(error.value.is_none());
    }

    #[test]
    fn parse_produces_typed_requests_for_registered_native_operations() {
        let action = parse_input("run cidr6 2001:db8::1/64", Some("net-lab"), PINNED).unwrap();
        let TerminalAction::Execute(request) = action else {
            panic!("expected a typed command request");
        };
        assert_eq!(request.tool_id, "net-lab");
        assert_eq!(request.handler, HandlerId::NetworkCidr6);
        assert_eq!(
            request.arguments,
            TypedArguments::Network {
                operation: NetworkOperation::Cidr6,
                input: "2001:db8::1/64".into(),
                extra: None,
            }
        );
        let text = parse_input("run encode Hello Rust", Some("byte-lab"), PINNED).unwrap();
        assert!(matches!(
            text,
            TerminalAction::Execute(CommandRequest {
                handler: HandlerId::BytesEncode,
                arguments: TypedArguments::Text { value },
                ..
            }) if value == "Hello Rust"
        ));
    }

    #[test]
    fn selection_never_calls_executor_and_only_pinned_registered_tools_select() {
        let action = parse_input("select host", None, PINNED).unwrap();
        assert_eq!(action, TerminalAction::SelectTool { tool_id: "host" });
        let mut executor = CountExecutor(0);
        let result = dispatch(action, &mut executor).unwrap();
        assert_eq!(executor.0, 0);
        assert!(matches!(result, DispatchResult::Control(_)));
        assert_eq!(
            parse_input("radio", None, PINNED),
            Err(CommandError::UnknownTool)
        );
        assert_eq!(
            parse_input("select library", None, PINNED),
            Err(CommandError::ToolNotPinned)
        );
        assert_eq!(
            parse_input("tools", None, PINNED).unwrap(),
            TerminalAction::ListTools(vec!["host", "byte-lab", "net-lab"])
        );
    }

    #[test]
    fn invalid_or_unregistered_inputs_fail_closed() {
        for input in [
            "sh -c id",
            "run scan local",
            "scan local",
            "run status; whoami",
        ] {
            assert!(parse_input(input, Some("host"), PINNED).is_err(), "{input}");
        }
        assert_eq!(
            parse_input("run status", Some("host"), &["net-lab"]),
            Err(CommandError::ToolNotPinned)
        );
        assert_eq!(
            parse_input("run status\u{0000}", Some("host"), PINNED),
            Err(CommandError::InvalidInput)
        );
        assert_eq!(
            parse_input(&"x".repeat(MAX_COMMAND_BYTES + 1), Some("host"), PINNED),
            Err(CommandError::Oversized)
        );
        assert_eq!(
            parse_input(
                &format!("run encode {}", "x".repeat(MAX_ARGUMENT_BYTES + 1)),
                Some("byte-lab"),
                PINNED
            ),
            Err(CommandError::Oversized)
        );
    }

    #[test]
    fn network_calculator_requires_typed_bounded_arguments() {
        let action = parse_input("run split4 10.40.0.0/24 26", Some("net-lab"), PINNED).unwrap();
        assert!(matches!(
            action,
            TerminalAction::Execute(CommandRequest {
                handler: HandlerId::NetworkSplit4,
                arguments: TypedArguments::Network { extra: Some(_), .. },
                ..
            })
        ));
        assert_eq!(
            parse_input("run split4 10.0.0.0/24", Some("net-lab"), PINNED),
            Err(CommandError::InvalidArguments)
        );
    }
}
