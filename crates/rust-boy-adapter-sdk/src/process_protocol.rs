//! Bounded `RBPK` binary stdio framing shared with independent package units.
//!
//! The host adapter SDK's JSON envelope remains the logical API. This module
//! encodes that API into versioned, bounded process frames without requiring a
//! child binary to link the host SDK or use the same Rust ABI.

use std::io::{self, Read, Write};

use crate::{AdapterCommand, AdapterEnvelope, AdapterError, AdapterReply, UniversalPackageAdapter};
use serde_json::Value;

pub const VERSION: u16 = 1;
pub const MAGIC: [u8; 4] = *b"RBPK";
pub const MAX_FRAME_BYTES: usize = 16 * 1024;
pub const MAX_PACKAGE_ID_BYTES: usize = 96;
pub const MAX_PATH_BYTES: usize = 4096;
pub const MAX_TITLE_BYTES: usize = 128;
pub const MAX_ERROR_BYTES: usize = 256;
pub const MAX_TRACKS_PER_VIEW: usize = 32;
pub const MAX_LIBRARY_TRACKS: u16 = 2048;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessProtocolError {
    Io,
    Truncated,
    Oversized,
    Invalid(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMessage {
    pub kind: MessageKind,
    pub package_id: String,
    pub request_id: u64,
    /// Version-specific typed body, including its one-byte command tag.
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageKind {
    Request = 1,
    Reply = 2,
}

pub struct BodyWriter(Vec<u8>);
impl BodyWriter {
    pub fn new(tag: u8) -> Self {
        Self(vec![tag])
    }
    pub fn byte(&mut self, value: u8) {
        self.0.push(value);
    }
    pub fn u16(&mut self, value: u16) {
        self.0.extend_from_slice(&value.to_be_bytes());
    }
    pub fn u32(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_be_bytes());
    }
    pub fn u64(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_be_bytes());
    }
    pub fn string(&mut self, value: &str, limit: usize) -> Result<(), ProcessProtocolError> {
        if value.len() > limit
            || value.len() > u16::MAX as usize
            || value.chars().any(char::is_control)
        {
            return Err(ProcessProtocolError::Invalid(
                "string outside byte/control limit",
            ));
        }
        self.u16(value.len() as u16);
        self.0.extend_from_slice(value.as_bytes());
        Ok(())
    }
    pub fn finish(self) -> Result<Vec<u8>, ProcessProtocolError> {
        if self.0.is_empty() || self.0.len() > MAX_FRAME_BYTES {
            Err(ProcessProtocolError::Oversized)
        } else {
            Ok(self.0)
        }
    }
}

pub struct BodyReader<'a> {
    bytes: &'a [u8],
    at: usize,
}

/// One synchronous request/reply channel. Call it only from a dedicated
/// package worker; the UI and host audio callbacks must never wait on pipes.
pub struct ProcessConnection<R: Read, W: Write> {
    package_id: String,
    next_request_id: u64,
    input: R,
    output: W,
}

impl<R: Read, W: Write> ProcessConnection<R, W> {
    pub fn handshake(
        package_id: &str,
        mut input: R,
        mut output: W,
    ) -> Result<Self, ProcessProtocolError> {
        if !valid_package_id(package_id) {
            return Err(ProcessProtocolError::Invalid("package ID invalid"));
        }
        let mut hello = BodyWriter::new(0);
        hello.u16(VERSION);
        hello.u16(VERSION);
        let hello = RawMessage {
            kind: MessageKind::Request,
            package_id: package_id.to_owned(),
            request_id: 1,
            body: hello.finish()?,
        };
        write_frame(&mut output, &hello.encode()?)?;
        let payload = read_frame(&mut input)?.ok_or(ProcessProtocolError::Truncated)?;
        let reply = RawMessage::decode(&payload)?;
        if reply.kind != MessageKind::Reply
            || reply.package_id != package_id
            || reply.request_id != 1
        {
            return Err(ProcessProtocolError::Invalid(
                "handshake identity or sequence mismatch",
            ));
        }
        let mut body = BodyReader::new(&reply.body)?;
        if body.byte()? != 0 || body.u16()? != VERSION {
            return Err(ProcessProtocolError::Invalid(
                "child rejected protocol version",
            ));
        }
        body.finish()?;
        Ok(Self {
            package_id: package_id.to_owned(),
            next_request_id: 2,
            input,
            output,
        })
    }

    /// Sends one typed body and waits for the matching reply. The supervising
    /// host worker must add a deadline and terminate the child on timeout.
    pub fn request(&mut self, body: BodyWriter) -> Result<RawMessage, ProcessProtocolError> {
        let request_id = self.next_request_id;
        self.next_request_id = request_id
            .checked_add(1)
            .ok_or(ProcessProtocolError::Invalid("request sequence exhausted"))?;
        let request = RawMessage {
            kind: MessageKind::Request,
            package_id: self.package_id.clone(),
            request_id,
            body: body.finish()?,
        };
        write_frame(&mut self.output, &request.encode()?)?;
        let payload = read_frame(&mut self.input)?.ok_or(ProcessProtocolError::Truncated)?;
        let reply = RawMessage::decode(&payload)?;
        if reply.kind != MessageKind::Reply
            || reply.package_id != self.package_id
            || reply.request_id != request_id
        {
            return Err(ProcessProtocolError::Invalid(
                "reply identity or sequence mismatch",
            ));
        }
        Ok(reply)
    }
}

/// Translates the ordinary SDK lifecycle/intents to one process protocol
/// session. Construct and call it only on the host's package worker thread.
pub struct ProcessRunner<R: Read + Send, W: Write + Send> {
    package_id: String,
    connection: ProcessConnection<R, W>,
}

impl<R: Read + Send, W: Write + Send> ProcessRunner<R, W> {
    pub fn handshake(package_id: &str, input: R, output: W) -> Result<Self, ProcessProtocolError> {
        let connection = ProcessConnection::handshake(package_id, input, output)?;
        Ok(Self {
            package_id: package_id.to_owned(),
            connection,
        })
    }
}

impl<R: Read + Send, W: Write + Send> UniversalPackageAdapter for ProcessRunner<R, W> {
    fn package_id(&self) -> &str {
        &self.package_id
    }

    fn handle(&mut self, request: AdapterEnvelope) -> Result<AdapterReply, AdapterError> {
        if request.package_id != self.package_id {
            return Err(AdapterError::Invalid("process request identity mismatch"));
        }
        let body = match request.body {
            AdapterCommand::Activate { viewport } => {
                let mut body = BodyWriter::new(1);
                body.u16(viewport.width);
                body.u16(viewport.height);
                body
            }
            AdapterCommand::Suspend => BodyWriter::new(2),
            AdapterCommand::Resume { viewport } => {
                let mut body = BodyWriter::new(3);
                body.u16(viewport.width);
                body.u16(viewport.height);
                body
            }
            AdapterCommand::Resize { viewport } => {
                let mut body = BodyWriter::new(4);
                body.u16(viewport.width);
                body.u16(viewport.height);
                body
            }
            AdapterCommand::Shutdown => BodyWriter::new(12),
            AdapterCommand::Intent {
                action,
                required_scope: _,
                payload,
            } => intent_body(&action, &payload)?,
        };
        let reply = self
            .connection
            .request(body)
            .map_err(|_| AdapterError::Unavailable("package process protocol failed".into()))?;
        decode_reply(&reply.body)
    }
}

fn intent_body(action: &str, payload: &Value) -> Result<BodyWriter, AdapterError> {
    match action {
        "music.scan" => {
            let root = payload
                .get("root")
                .and_then(Value::as_str)
                .ok_or(AdapterError::Invalid("music scan needs a root path"))?;
            let mut body = BodyWriter::new(5);
            body.string(root, MAX_PATH_BYTES)
                .map_err(|_| AdapterError::Invalid("music library path exceeds bounds"))?;
            Ok(body)
        }
        "ui.music.select" => {
            let index = payload
                .get("index")
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok())
                .ok_or(AdapterError::Invalid("music selection index is invalid"))?;
            let mut body = BodyWriter::new(6);
            body.u16(index);
            Ok(body)
        }
        "ui.music.previous" => Ok(BodyWriter::new(7)),
        "ui.music.next" => Ok(BodyWriter::new(8)),
        "music.play" => Ok(BodyWriter::new(9)),
        "music.stop" => Ok(BodyWriter::new(10)),
        "ui.music.snapshot" => {
            let page = payload.get("page").and_then(Value::as_u64).unwrap_or(0);
            let page = u16::try_from(page)
                .map_err(|_| AdapterError::Invalid("music page is out of range"))?;
            let mut body = BodyWriter::new(11);
            body.u16(page);
            Ok(body)
        }
        _ => Err(AdapterError::Unavailable(
            "music action is not supported by this process protocol".into(),
        )),
    }
}

pub fn decode_reply(body: &[u8]) -> Result<AdapterReply, AdapterError> {
    let mut reader = BodyReader::new(body)
        .map_err(|_| AdapterError::Fault("invalid package process reply".into()))?;
    let tag = reader
        .byte()
        .map_err(|_| AdapterError::Fault("truncated package process reply".into()))?;
    match tag {
        0 => {
            let version = reader
                .u16()
                .map_err(|_| AdapterError::Fault("truncated package handshake reply".into()))?;
            reader
                .finish()
                .map_err(|_| AdapterError::Fault("malformed package handshake reply".into()))?;
            if version != VERSION {
                return Err(AdapterError::Fault(
                    "child selected an unexpected protocol version".into(),
                ));
            }
            Ok(AdapterReply::Ack)
        }
        1 => {
            reader
                .finish()
                .map_err(|_| AdapterError::Fault("malformed package acknowledgement".into()))?;
            Ok(AdapterReply::Ack)
        }
        2 => decode_view(&mut reader),
        3 => {
            let code = reader
                .byte()
                .map_err(|_| AdapterError::Fault("truncated package error reply".into()))?;
            let message = reader
                .string(MAX_ERROR_BYTES)
                .map_err(|_| AdapterError::Fault("malformed package error reply".into()))?;
            reader
                .finish()
                .map_err(|_| AdapterError::Fault("malformed package error reply".into()))?;
            Err(match code {
                1 => AdapterError::Invalid("package rejected the request"),
                2 => AdapterError::Denied("package request was denied"),
                3 => AdapterError::Invalid("package operation is unsupported"),
                4 => AdapterError::Unavailable(message),
                5..=7 => AdapterError::Fault(message),
                _ => AdapterError::Fault("unknown package error code".into()),
            })
        }
        _ => Err(AdapterError::Fault("unknown package reply tag".into())),
    }
}

fn decode_view(reader: &mut BodyReader<'_>) -> Result<AdapterReply, AdapterError> {
    let playback = reader
        .byte()
        .map_err(|_| AdapterError::Fault("truncated package view".into()))?;
    if playback > 5 {
        return Err(AdapterError::Fault("unknown package playback state".into()));
    }
    let selected = match reader.byte().map_err(protocol_reply_error)? {
        0 => None,
        1 => Some(reader.u16().map_err(protocol_reply_error)?),
        _ => return Err(AdapterError::Fault("invalid package selection flag".into())),
    };
    let track_count = reader.u16().map_err(protocol_reply_error)?;
    let page_offset = reader.u16().map_err(protocol_reply_error)?;
    let count = usize::from(reader.byte().map_err(protocol_reply_error)?);
    if count > MAX_TRACKS_PER_VIEW
        || track_count > MAX_LIBRARY_TRACKS
        || page_offset > track_count
        || u32::from(page_offset) + count as u32 > u32::from(track_count)
    {
        return Err(AdapterError::Fault(
            "package track view outside bounds".into(),
        ));
    }
    let mut tracks = Vec::with_capacity(count);
    for offset in 0..count {
        let index = reader.u16().map_err(protocol_reply_error)?;
        let title = reader
            .string(MAX_TITLE_BYTES)
            .map_err(protocol_reply_error)?;
        let container = match reader.byte().map_err(protocol_reply_error)? {
            1 => "flac",
            2 => "wave",
            3 => "aiff",
            4 => "caf",
            5 => "mp4",
            _ => {
                return Err(AdapterError::Fault(
                    "unknown package audio container".into(),
                ))
            }
        };
        if index >= track_count || usize::from(index) != usize::from(page_offset) + offset {
            return Err(AdapterError::Fault(
                "package track index outside bounds".into(),
            ));
        }
        tracks.push(serde_json::json!({"index": index, "title": title, "container": container}));
    }
    let format = match reader.byte().map_err(protocol_reply_error)? {
        0 => Value::Null,
        1 => {
            let sample_rate = reader.u32().map_err(protocol_reply_error)?;
            let channels = reader.byte().map_err(protocol_reply_error)?;
            let bits_per_sample = reader.byte().map_err(protocol_reply_error)?;
            if !(1..=1_048_575).contains(&sample_rate)
                || !(1..=8).contains(&channels)
                || !(4..=32).contains(&bits_per_sample)
            {
                return Err(AdapterError::Fault(
                    "invalid PCM format in package view".into(),
                ));
            }
            serde_json::json!({"sample_rate": sample_rate, "channels": channels, "bits_per_sample": bits_per_sample})
        }
        _ => return Err(AdapterError::Fault("invalid package format flag".into())),
    };
    let frames_delivered = reader.u64().map_err(protocol_reply_error)?;
    let total_frames = match reader.byte().map_err(protocol_reply_error)? {
        0 => Value::Null,
        1 => serde_json::json!(reader.u64().map_err(protocol_reply_error)?),
        _ => {
            return Err(AdapterError::Fault(
                "invalid package frame-count flag".into(),
            ))
        }
    };
    let audio_claimed = match reader.byte().map_err(protocol_reply_error)? {
        0 => false,
        1 => true,
        _ => {
            return Err(AdapterError::Fault(
                "invalid package audio-claim flag".into(),
            ))
        }
    };
    reader
        .finish()
        .map_err(|_| AdapterError::Fault("trailing package view bytes".into()))?;
    if selected.is_some_and(|index| index >= track_count) {
        return Err(AdapterError::Fault(
            "selected package track outside bounds".into(),
        ));
    }
    Ok(AdapterReply::View {
        state: serde_json::json!({
            "module": "MUSIC",
            "theme": "pipboy_phosphor_green",
            "playback": playback,
            "selected": selected,
            "track_count": track_count,
            "page_offset": page_offset,
            "tracks": tracks,
            "format": format,
            "frames_delivered": frames_delivered,
            "total_frames": total_frames,
            "audio_claimed": audio_claimed
        }),
    })
}

fn protocol_reply_error(_: ProcessProtocolError) -> AdapterError {
    AdapterError::Fault("malformed package view".into())
}
impl<'a> BodyReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, ProcessProtocolError> {
        if bytes.is_empty() || bytes.len() > MAX_FRAME_BYTES {
            return Err(ProcessProtocolError::Oversized);
        }
        Ok(Self { bytes, at: 0 })
    }
    pub fn byte(&mut self) -> Result<u8, ProcessProtocolError> {
        Ok(self.take(1)?[0])
    }
    pub fn u16(&mut self) -> Result<u16, ProcessProtocolError> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    pub fn u32(&mut self) -> Result<u32, ProcessProtocolError> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn u64(&mut self) -> Result<u64, ProcessProtocolError> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    pub fn string(&mut self, limit: usize) -> Result<String, ProcessProtocolError> {
        let len = usize::from(self.u16()?);
        if len > limit {
            return Err(ProcessProtocolError::Invalid("string exceeds byte limit"));
        }
        let value = std::str::from_utf8(self.take(len)?)
            .map_err(|_| ProcessProtocolError::Invalid("string is not UTF-8"))?;
        if value.chars().any(char::is_control) {
            return Err(ProcessProtocolError::Invalid("control character in string"));
        }
        Ok(value.to_owned())
    }
    pub fn finish(&mut self) -> Result<(), ProcessProtocolError> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(ProcessProtocolError::Invalid("trailing body bytes"))
        }
    }
    fn take(&mut self, len: usize) -> Result<&'a [u8], ProcessProtocolError> {
        let end = self
            .at
            .checked_add(len)
            .ok_or(ProcessProtocolError::Truncated)?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .ok_or(ProcessProtocolError::Truncated)?;
        self.at = end;
        Ok(bytes)
    }
}

impl RawMessage {
    pub fn encode(&self) -> Result<Vec<u8>, ProcessProtocolError> {
        if !valid_package_id(&self.package_id) || self.request_id == 0 {
            return Err(ProcessProtocolError::Invalid(
                "package ID/request ID invalid",
            ));
        }
        if self.body.is_empty() || self.body.len() > MAX_FRAME_BYTES {
            return Err(ProcessProtocolError::Oversized);
        }
        let mut out = Vec::with_capacity(12 + self.package_id.len() + self.body.len());
        out.push(VERSION as u8);
        out.push(self.kind as u8);
        out.extend_from_slice(&self.request_id.to_be_bytes());
        out.extend_from_slice(&(self.package_id.len() as u16).to_be_bytes());
        out.extend_from_slice(self.package_id.as_bytes());
        out.extend_from_slice(&self.body);
        if out.len() > MAX_FRAME_BYTES {
            return Err(ProcessProtocolError::Oversized);
        }
        Ok(out)
    }

    pub fn decode(payload: &[u8]) -> Result<Self, ProcessProtocolError> {
        if payload.len() > MAX_FRAME_BYTES {
            return Err(ProcessProtocolError::Oversized);
        }
        if payload.len() < 12 || payload[0] != VERSION as u8 {
            return Err(ProcessProtocolError::Invalid(
                "protocol version or envelope truncated",
            ));
        }
        let kind = match payload[1] {
            1 => MessageKind::Request,
            2 => MessageKind::Reply,
            _ => return Err(ProcessProtocolError::Invalid("unknown message kind")),
        };
        let request_id = u64::from_be_bytes(payload[2..10].try_into().unwrap());
        let id_len = usize::from(u16::from_be_bytes(payload[10..12].try_into().unwrap()));
        if request_id == 0 || id_len == 0 || id_len > MAX_PACKAGE_ID_BYTES {
            return Err(ProcessProtocolError::Invalid(
                "package ID/request ID invalid",
            ));
        }
        let id_end = 12usize
            .checked_add(id_len)
            .ok_or(ProcessProtocolError::Truncated)?;
        let package_id = std::str::from_utf8(
            payload
                .get(12..id_end)
                .ok_or(ProcessProtocolError::Truncated)?,
        )
        .map_err(|_| ProcessProtocolError::Invalid("package ID is not UTF-8"))?
        .to_owned();
        if !valid_package_id(&package_id) || id_end == payload.len() {
            return Err(ProcessProtocolError::Invalid(
                "package ID invalid or body absent",
            ));
        }
        let message = Self {
            kind,
            package_id,
            request_id,
            body: payload[id_end..].to_vec(),
        };
        if message.encode()? != payload {
            return Err(ProcessProtocolError::Invalid("noncanonical envelope"));
        }
        Ok(message)
    }
}

fn valid_package_id(value: &str) -> bool {
    value.len() <= MAX_PACKAGE_ID_BYTES
        && value.split('.').count() >= 3
        && value.split('.').all(|part| {
            !part.is_empty()
                && part.len() <= 32
                && part.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

pub fn write_frame<W: Write>(writer: &mut W, payload: &[u8]) -> Result<(), ProcessProtocolError> {
    if payload.is_empty() || payload.len() > MAX_FRAME_BYTES {
        return Err(ProcessProtocolError::Oversized);
    }
    writer
        .write_all(&MAGIC)
        .map_err(|_| ProcessProtocolError::Io)?;
    writer
        .write_all(&(payload.len() as u32).to_be_bytes())
        .map_err(|_| ProcessProtocolError::Io)?;
    writer
        .write_all(payload)
        .map_err(|_| ProcessProtocolError::Io)?;
    writer.flush().map_err(|_| ProcessProtocolError::Io)
}

pub fn read_frame<R: Read>(reader: &mut R) -> Result<Option<Vec<u8>>, ProcessProtocolError> {
    let mut header = [0u8; 8];
    loop {
        match reader.read(&mut header[..1]) {
            Ok(0) => return Ok(None),
            Ok(1) => break,
            Ok(_) => unreachable!(),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(ProcessProtocolError::Io),
        }
    }
    reader
        .read_exact(&mut header[1..])
        .map_err(|_| ProcessProtocolError::Truncated)?;
    if header[..4] != MAGIC {
        return Err(ProcessProtocolError::Invalid("frame magic mismatch"));
    }
    let len = u32::from_be_bytes(header[4..8].try_into().unwrap()) as usize;
    if len == 0 {
        return Err(ProcessProtocolError::Invalid("empty frame"));
    }
    if len > MAX_FRAME_BYTES {
        return Err(ProcessProtocolError::Oversized);
    }
    let mut payload = vec![0; len];
    reader
        .read_exact(&mut payload)
        .map_err(|_| ProcessProtocolError::Truncated)?;
    Ok(Some(payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO_V1: &[u8] = &[
        1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 17, b'o', b'r', b'g', b'.', b'r', b'u', b's', b't', b'b',
        b'o', b'y', b'.', b'm', b'u', b's', b'i', b'c', 0, 0, 1, 0, 1,
    ];

    #[test]
    fn matches_rust_music_hello_vector_and_frame_limits() {
        let msg = RawMessage {
            kind: MessageKind::Request,
            package_id: "org.rustboy.music".into(),
            request_id: 1,
            body: vec![0, 0, 1, 0, 1],
        };
        assert_eq!(msg.encode().unwrap(), HELLO_V1);
        assert_eq!(RawMessage::decode(HELLO_V1).unwrap(), msg);
        let mut frame = Vec::new();
        write_frame(&mut frame, HELLO_V1).unwrap();
        assert_eq!(&frame[..8], b"RBPK\0\0\0\x22");
        assert_eq!(
            read_frame(&mut frame.as_slice()).unwrap(),
            Some(HELLO_V1.to_vec())
        );
    }

    #[test]
    fn malformed_ids_and_unbounded_frames_are_rejected() {
        for id in ["", "org", "org.RustBoy.music", "org.rustboy.music/"] {
            assert!(RawMessage {
                kind: MessageKind::Request,
                package_id: id.into(),
                request_id: 1,
                body: vec![0]
            }
            .encode()
            .is_err());
        }
        let oversized = [
            MAGIC.as_slice(),
            &((MAX_FRAME_BYTES as u32 + 1).to_be_bytes()),
        ]
        .concat();
        assert_eq!(
            read_frame(&mut oversized.as_slice()),
            Err(ProcessProtocolError::Oversized)
        );
        assert!(BodyWriter::new(0)
            .string(&"x".repeat(MAX_PATH_BYTES + 1), MAX_PATH_BYTES)
            .is_err());
        assert!(BodyReader::new(&[]).is_err());
    }

    #[test]
    fn body_text_is_strict_utf8_bounded_and_fully_consumed() {
        let mut writer = BodyWriter::new(5);
        writer.string("/music/álbum", MAX_PATH_BYTES).unwrap();
        let bytes = writer.finish().unwrap();
        let mut reader = BodyReader::new(&bytes).unwrap();
        assert_eq!(reader.byte().unwrap(), 5);
        assert_eq!(reader.string(MAX_PATH_BYTES).unwrap(), "/music/álbum");
        reader.finish().unwrap();
        assert!(BodyWriter::new(5)
            .string("bad\npath", MAX_PATH_BYTES)
            .is_err());
    }

    #[test]
    fn connection_checks_handshake_identity_and_monotonic_reply_id() {
        let hello = RawMessage {
            kind: MessageKind::Reply,
            package_id: "org.rustboy.music".into(),
            request_id: 1,
            body: vec![0, 0, 1],
        }
        .encode()
        .unwrap();
        let ack = RawMessage {
            kind: MessageKind::Reply,
            package_id: "org.rustboy.music".into(),
            request_id: 2,
            body: vec![1],
        }
        .encode()
        .unwrap();
        let mut input = Vec::new();
        write_frame(&mut input, &hello).unwrap();
        write_frame(&mut input, &ack).unwrap();
        let mut output = Vec::new();
        let mut connection = ProcessConnection::handshake(
            "org.rustboy.music",
            std::io::Cursor::new(input),
            &mut output,
        )
        .unwrap();
        let reply = connection.request(BodyWriter::new(2)).unwrap();
        assert_eq!(reply.request_id, 2);
        let mut frames = output.as_slice();
        assert_eq!(
            RawMessage::decode(&read_frame(&mut frames).unwrap().unwrap())
                .unwrap()
                .request_id,
            1
        );
        assert_eq!(
            RawMessage::decode(&read_frame(&mut frames).unwrap().unwrap())
                .unwrap()
                .request_id,
            2
        );
    }

    #[test]
    fn completed_process_frames_flush_the_output_stream() {
        #[derive(Default)]
        struct FlushProbe {
            bytes: Vec<u8>,
            flushed: bool,
        }
        impl Write for FlushProbe {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.bytes.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                self.flushed = true;
                Ok(())
            }
        }
        let mut output = FlushProbe::default();
        write_frame(&mut output, &[1]).unwrap();
        assert!(output.flushed);
    }
}
