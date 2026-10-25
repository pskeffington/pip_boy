//! Typed offline adapters backed only by Rust-Boy virtual fixtures.
//! There is deliberately no OS device, socket, serial or display backend.

use crate::capability::Capability;
use crate::device_simulator::{SessionConfirmation, SimulationEvent};
use crate::permissions::PreferenceEntry;
use crate::virtual_transport::{TransportError, VirtualTransport};
use serde::Serialize;
use std::error::Error;
use std::fmt;

pub const MAX_DISPLAY_CHARS: usize = 80;
pub const MAX_SERIAL_FIXTURE_BYTES: usize = 32;

#[derive(Debug, Eq, PartialEq)]
pub enum AdapterError {
    Transport(TransportError),
    InvalidDisplayFrame,
    InvalidSerialFixture,
    MissingFixtureEvent,
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(error) => error.fmt(f),
            Self::InvalidDisplayFrame => {
                f.write_str("mock display frame must contain 1..=80 printable ASCII characters")
            }
            Self::InvalidSerialFixture => {
                f.write_str("injected serial fixture must contain 1..=32 bytes")
            }
            Self::MissingFixtureEvent => f.write_str("no simulated fixture event available"),
        }
    }
}

impl Error for AdapterError {}

impl From<TransportError> for AdapterError {
    fn from(value: TransportError) -> Self {
        Self::Transport(value)
    }
}

fn one_fixture(transport: &mut VirtualTransport) -> Result<SimulationEvent, AdapterError> {
    transport.pump(1)?;
    transport
        .receive()?
        .ok_or(AdapterError::MissingFixtureEvent)
}

/// An input source reports only explicitly labeled, nonphysical fixture events.
pub trait InputSource {
    fn poll_input(&mut self) -> Result<MockInputEvent, AdapterError>;
}

/// Accepts a bounded mock display frame but renders nothing on a real device.
pub trait DisplayBackend {
    fn submit_frame(&mut self, text: &str) -> Result<MockDisplayReceipt, AdapterError>;
}

/// Reads injected test bytes only, never from an OS serial port.
pub trait SerialReader {
    fn read_frame(&mut self) -> Result<MockSerialFrame, AdapterError>;
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct MockInputEvent {
    pub schema_version: u8,
    pub simulated: bool,
    pub source: &'static str,
    pub device_id: &'static str,
    pub event: &'static str,
    pub sequence: u64,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct MockDisplayReceipt {
    pub schema_version: u8,
    pub simulated: bool,
    pub rendered_to_hardware: bool,
    pub source: &'static str,
    pub device_id: &'static str,
    pub event: &'static str,
    pub accepted_chars: usize,
    pub sequence: u64,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct MockSerialFrame {
    pub schema_version: u8,
    pub simulated: bool,
    pub injected: bool,
    pub source: &'static str,
    pub device_id: &'static str,
    pub event: &'static str,
    pub fixture_bytes: Vec<u8>,
    pub sequence: u64,
}

pub struct MockInputSource {
    transport: VirtualTransport,
}

impl MockInputSource {
    pub fn open(
        preferences: &[PreferenceEntry],
        confirmation: SessionConfirmation,
    ) -> Result<Self, AdapterError> {
        let mut transport = VirtualTransport::new(preferences, 1)?;
        transport.open("sim:gpio", confirmation)?;
        Ok(Self { transport })
    }

    pub fn revoke(&mut self) -> bool {
        self.transport.revoke(Capability::GpioRead)
    }
}

impl InputSource for MockInputSource {
    fn poll_input(&mut self) -> Result<MockInputEvent, AdapterError> {
        let event = one_fixture(&mut self.transport)?;
        Ok(MockInputEvent {
            schema_version: 1,
            simulated: true,
            source: "rust-boy/mock-input-adapter",
            device_id: "sim:gpio",
            event: "no_physical_input_observed",
            sequence: event.sequence,
        })
    }
}

pub struct MockDisplayBackend {
    transport: VirtualTransport,
}

impl MockDisplayBackend {
    pub fn open(
        preferences: &[PreferenceEntry],
        confirmation: SessionConfirmation,
    ) -> Result<Self, AdapterError> {
        let mut transport = VirtualTransport::new(preferences, 1)?;
        transport.open("sim:display", confirmation)?;
        Ok(Self { transport })
    }

    pub fn cancel(&mut self) -> Result<(), AdapterError> {
        self.transport.cancel()?;
        Ok(())
    }
}

impl DisplayBackend for MockDisplayBackend {
    fn submit_frame(&mut self, text: &str) -> Result<MockDisplayReceipt, AdapterError> {
        if text.is_empty()
            || text.len() > MAX_DISPLAY_CHARS
            || !text.bytes().all(|byte| (32..=126).contains(&byte))
        {
            return Err(AdapterError::InvalidDisplayFrame);
        }
        let event = one_fixture(&mut self.transport)?;
        Ok(MockDisplayReceipt {
            schema_version: 1,
            simulated: true,
            rendered_to_hardware: false,
            source: "rust-boy/mock-display-adapter",
            device_id: "sim:display",
            event: "mock_frame_accepted_not_rendered",
            accepted_chars: text.len(),
            sequence: event.sequence,
        })
    }
}

pub struct MockSerialReader {
    transport: VirtualTransport,
    fixture_bytes: Vec<u8>,
}

impl MockSerialReader {
    pub fn open(
        preferences: &[PreferenceEntry],
        confirmation: SessionConfirmation,
        injected_bytes: &[u8],
    ) -> Result<Self, AdapterError> {
        if injected_bytes.is_empty() || injected_bytes.len() > MAX_SERIAL_FIXTURE_BYTES {
            return Err(AdapterError::InvalidSerialFixture);
        }
        let mut transport = VirtualTransport::new(preferences, 1)?;
        transport.open("sim:serial", confirmation)?;
        Ok(Self {
            transport,
            fixture_bytes: injected_bytes.to_vec(),
        })
    }

    pub fn revoke(&mut self) -> bool {
        self.fixture_bytes.clear();
        self.transport.revoke(Capability::SerialRead)
    }
}

impl SerialReader for MockSerialReader {
    fn read_frame(&mut self) -> Result<MockSerialFrame, AdapterError> {
        let event = one_fixture(&mut self.transport)?;
        Ok(MockSerialFrame {
            schema_version: 1,
            simulated: true,
            injected: true,
            source: "rust-boy/mock-serial-adapter",
            device_id: "sim:serial",
            event: "explicitly_injected_fixture_bytes",
            fixture_bytes: self.fixture_bytes.clone(),
            sequence: event.sequence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_simulator::BrokerError;
    use crate::permissions::Preference;

    fn preference(capability: &str, preference: Preference) -> PreferenceEntry {
        PreferenceEntry {
            capability: capability.to_owned(),
            preference,
        }
    }

    #[test]
    fn trait_object_input_emits_no_invented_gpio_value() {
        let mut input = MockInputSource::open(&[], SessionConfirmation::Confirmed).unwrap();
        let source: &mut dyn InputSource = &mut input;
        let event = source.poll_input().unwrap();
        assert_eq!(event.device_id, "sim:gpio");
        assert!(event.simulated);
        assert_eq!(event.event, "no_physical_input_observed");
        assert_eq!(event.sequence, 1);
        let json = serde_json::to_value(event).unwrap();
        assert!(json.get("pin_voltage").is_none());
        assert!(json.get("button_pressed").is_none());
    }

    #[test]
    fn display_trait_never_renders_to_hardware() {
        let mut display = MockDisplayBackend::open(&[], SessionConfirmation::Confirmed).unwrap();
        let backend: &mut dyn DisplayBackend = &mut display;
        let receipt = backend.submit_frame("RUST-BOY").unwrap();
        assert!(!receipt.rendered_to_hardware);
        assert_eq!(receipt.accepted_chars, 8);
        assert_eq!(receipt.event, "mock_frame_accepted_not_rendered");
        assert_eq!(receipt.sequence, 1);
    }

    #[test]
    fn display_rejects_controls_unicode_and_oversize_frames() {
        let mut display = MockDisplayBackend::open(&[], SessionConfirmation::Confirmed).unwrap();
        for text in ["", "\u{1b}[31m", "line\nfeed", "naive\u{00e9}"] {
            assert_eq!(
                display.submit_frame(text),
                Err(AdapterError::InvalidDisplayFrame)
            );
        }
        let oversized = "X".repeat(MAX_DISPLAY_CHARS + 1);
        assert_eq!(
            display.submit_frame(&oversized),
            Err(AdapterError::InvalidDisplayFrame)
        );
        assert_eq!(display.submit_frame("OK").unwrap().sequence, 1);
    }

    #[test]
    fn serial_reader_is_explicitly_injected_not_discovered() {
        let bytes = [0x52, 0x42, 0x00, 0xff];
        let mut serial = MockSerialReader::open(&[], SessionConfirmation::Confirmed, &bytes)
            .unwrap();
        let reader: &mut dyn SerialReader = &mut serial;
        let event = reader.read_frame().unwrap();
        assert_eq!(event.fixture_bytes, bytes);
        assert!(event.injected);
        assert!(event.simulated);
        assert_eq!(event.sequence, 1);
        assert_eq!(event.event, "explicitly_injected_fixture_bytes");
    }

    #[test]
    fn empty_or_unbounded_serial_fixtures_reject() {
        assert!(matches!(
            MockSerialReader::open(&[], SessionConfirmation::Confirmed, &[]),
            Err(AdapterError::InvalidSerialFixture)
        ));
        let excessive = vec![0xaa; MAX_SERIAL_FIXTURE_BYTES + 1];
        assert!(matches!(
            MockSerialReader::open(&[], SessionConfirmation::Confirmed, &excessive),
            Err(AdapterError::InvalidSerialFixture)
        ));
    }

    #[test]
    fn saved_requests_cannot_replace_session_confirmation() {
        let preferences = [preference("gpio.read", Preference::Requested)];
        let result = MockInputSource::open(&preferences, SessionConfirmation::Absent);
        assert!(matches!(
            result,
            Err(AdapterError::Transport(TransportError::Broker(
                BrokerError::ConfirmationRequired
            )))
        ));
    }

    #[test]
    fn saved_denial_overrides_confirmation_across_adapter_types() {
        let input = [preference("gpio.read", Preference::Denied)];
        let display = [preference("display.write", Preference::Denied)];
        let serial = [preference("serial.read", Preference::Denied)];
        assert!(matches!(
            MockInputSource::open(&input, SessionConfirmation::Confirmed),
            Err(AdapterError::Transport(TransportError::Broker(
                BrokerError::DeniedPreference
            )))
        ));
        assert!(matches!(
            MockDisplayBackend::open(&display, SessionConfirmation::Confirmed),
            Err(AdapterError::Transport(TransportError::Broker(
                BrokerError::DeniedPreference
            )))
        ));
        assert!(matches!(
            MockSerialReader::open(&serial, SessionConfirmation::Confirmed, &[0x52]),
            Err(AdapterError::Transport(TransportError::Broker(
                BrokerError::DeniedPreference
            )))
        ));
    }

    #[test]
    fn revoked_input_and_serial_tokens_fail_closed() {
        let mut input = MockInputSource::open(&[], SessionConfirmation::Confirmed).unwrap();
        assert_eq!(input.poll_input().unwrap().sequence, 1);
        assert!(input.revoke());
        assert!(input.poll_input().is_err());
        let mut serial = MockSerialReader::open(&[], SessionConfirmation::Confirmed, &[0x42])
            .unwrap();
        assert!(serial.revoke());
        assert!(serial.read_frame().is_err());
    }

    #[test]
    fn canceled_display_never_accepts_another_frame() {
        let mut display = MockDisplayBackend::open(&[], SessionConfirmation::Confirmed).unwrap();
        display.submit_frame("READY").unwrap();
        display.cancel().unwrap();
        assert!(display.submit_frame("NOPE").is_err());
    }

    #[test]
    fn adapter_outputs_are_byte_stable_for_fixed_fixtures() {
        fn one_run() -> Vec<u8> {
            let mut serial = MockSerialReader::open(&[], SessionConfirmation::Confirmed, &[82, 66])
                .unwrap();
            serde_json::to_vec(&serial.read_frame().unwrap()).unwrap()
        }
        assert_eq!(one_run(), one_run());
    }
}
