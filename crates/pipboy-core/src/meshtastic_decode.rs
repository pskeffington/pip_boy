//! Strict, read-only decoder for recorded Meshtastic FromRadio protobuf bytes.
//!
//! This is an offline capture inspection helper, not a transport. Its output is
//! explicitly unverified: Meshtastic node numbers are not identities, and a
//! decoded payload is not proof of delivery, authentication, or receipt by
//! Rust-Boy. Only TEXT_MESSAGE_APP is surfaced; all other application data,
//! including location, admin, and telemetry, is ignored.

pub const MAX_CAPTURE_BYTES: usize = 512;
pub const MAX_TEXT_BYTES: usize = 233;
pub const TEXT_MESSAGE_APP: u64 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnverifiedTextObservation {
    /// Source node number as recorded in the capture; not authenticated.
    pub source_node_unverified: Option<u64>,
    /// Destination node number as recorded in the capture; not authenticated.
    pub destination_node_unverified: Option<u64>,
    /// Channel index as recorded in the capture; not authenticated.
    pub channel_index_unverified: Option<u64>,
    /// Decoded text from a capture, not a delivery or receipt record.
    pub text_unverified: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    CaptureTooLarge,
    Malformed(&'static str),
    Duplicate(&'static str),
    InvalidText(&'static str),
}

#[derive(Clone, Copy)]
struct Field<'a> {
    number: u32,
    wire: u8,
    value: Value<'a>,
}

#[derive(Clone, Copy)]
enum Value<'a> {
    Varint(u64),
    Bytes(&'a [u8]),
    Fixed,
}

struct Fields<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> Fields<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    fn next(&mut self) -> Result<Option<Field<'a>>, DecodeError> {
        if self.offset == self.input.len() {
            return Ok(None);
        }
        let key = read_varint(self.input, &mut self.offset)?;
        let number = (key >> 3) as u32;
        let wire = (key & 7) as u8;
        if number == 0 || number > 536_870_911 || (19000..=19999).contains(&number) {
            return Err(DecodeError::Malformed("invalid protobuf field number"));
        }
        let value = match wire {
            0 => Value::Varint(read_varint(self.input, &mut self.offset)?),
            1 => {
                take(self.input, &mut self.offset, 8)?;
                Value::Fixed
            }
            2 => {
                let len = read_varint(self.input, &mut self.offset)?;
                let len =
                    usize::try_from(len).map_err(|_| DecodeError::Malformed("length overflow"))?;
                Value::Bytes(take(self.input, &mut self.offset, len)?)
            }
            5 => {
                take(self.input, &mut self.offset, 4)?;
                Value::Fixed
            }
            3 | 4 => return Err(DecodeError::Malformed("protobuf groups are not accepted")),
            _ => return Err(DecodeError::Malformed("invalid protobuf wire type")),
        };
        Ok(Some(Field {
            number,
            wire,
            value,
        }))
    }
}

impl<'a> Iterator for Fields<'a> {
    type Item = Result<Field<'a>, DecodeError>;
    fn next(&mut self) -> Option<Self::Item> {
        match Fields::next(self) {
            Ok(Some(field)) => Some(Ok(field)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        }
    }
}

fn take<'a>(input: &'a [u8], offset: &mut usize, len: usize) -> Result<&'a [u8], DecodeError> {
    let end = offset
        .checked_add(len)
        .ok_or(DecodeError::Malformed("length overflow"))?;
    if end > input.len() {
        return Err(DecodeError::Malformed("truncated field"));
    }
    let value = &input[*offset..end];
    *offset = end;
    Ok(value)
}

fn read_varint(input: &[u8], offset: &mut usize) -> Result<u64, DecodeError> {
    let mut value = 0u64;
    for index in 0..10 {
        let byte = *input
            .get(*offset)
            .ok_or(DecodeError::Malformed("truncated varint"))?;
        *offset += 1;
        if index == 9 && byte > 1 {
            return Err(DecodeError::Malformed("varint overflow"));
        }
        value |= u64::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(DecodeError::Malformed("varint overflow"))
}

fn bytes<'a>(field: Field<'a>, label: &'static str) -> Result<&'a [u8], DecodeError> {
    if field.wire != 2 {
        return Err(DecodeError::Malformed(label));
    }
    match field.value {
        Value::Bytes(value) => Ok(value),
        _ => Err(DecodeError::Malformed(label)),
    }
}
fn varint(field: Field<'_>, label: &'static str) -> Result<u64, DecodeError> {
    if field.wire != 0 {
        return Err(DecodeError::Malformed(label));
    }
    match field.value {
        Value::Varint(value) => Ok(value),
        _ => Err(DecodeError::Malformed(label)),
    }
}

/// Decode a recorded FromRadio protobuf payload, bounded to 512 bytes.
/// Unknown fields with valid protobuf encodings are skipped. Only the path
/// FromRadio.packet -> MeshPacket.decoded -> Data(TEXT_MESSAGE_APP) is read.
pub fn decode_recorded_from_radio(
    input: &[u8],
) -> Result<Option<UnverifiedTextObservation>, DecodeError> {
    if input.len() > MAX_CAPTURE_BYTES {
        return Err(DecodeError::CaptureTooLarge);
    }
    let mut packet = None;
    let mut fields = Fields::new(input);
    while let Some(field) = fields.next()? {
        if field.number == 2 {
            if packet.is_some() {
                return Err(DecodeError::Duplicate("FromRadio.packet oneof"));
            }
            packet = Some(bytes(field, "FromRadio.packet must be length-delimited")?);
        }
    }
    let Some(packet) = packet else {
        return Ok(None);
    };
    decode_packet(packet)
}

fn decode_packet(input: &[u8]) -> Result<Option<UnverifiedTextObservation>, DecodeError> {
    let (mut source, mut destination, mut channel) = (None, None, None);
    let (mut decoded, mut encrypted) = (None, false);
    let mut fields = Fields::new(input);
    while let Some(field) = fields.next()? {
        match field.number {
            1 => set_once(
                &mut source,
                varint(field, "MeshPacket.from must be varint")?,
                "MeshPacket.from",
            )?,
            2 => set_once(
                &mut destination,
                varint(field, "MeshPacket.to must be varint")?,
                "MeshPacket.to",
            )?,
            3 => set_once(
                &mut channel,
                varint(field, "MeshPacket.channel must be varint")?,
                "MeshPacket.channel",
            )?,
            4 => {
                if decoded.is_some() || encrypted {
                    return Err(DecodeError::Duplicate("MeshPacket decoded/encrypted oneof"));
                }
                decoded = Some(bytes(field, "MeshPacket.decoded must be length-delimited")?);
            }
            5 => {
                if decoded.is_some() || encrypted {
                    return Err(DecodeError::Duplicate("MeshPacket decoded/encrypted oneof"));
                }
                let _opaque_ciphertext =
                    bytes(field, "MeshPacket.encrypted must be length-delimited")?;
                encrypted = true;
            }
            _ => {}
        }
    }
    let Some(decoded) = decoded else {
        return Ok(None);
    };
    decode_data(decoded, source, destination, channel)
}

fn set_once<T>(slot: &mut Option<T>, value: T, label: &'static str) -> Result<(), DecodeError> {
    if slot.is_some() {
        return Err(DecodeError::Duplicate(label));
    }
    *slot = Some(value);
    Ok(())
}

fn decode_data(
    input: &[u8],
    source: Option<u64>,
    destination: Option<u64>,
    channel: Option<u64>,
) -> Result<Option<UnverifiedTextObservation>, DecodeError> {
    let (mut port, mut payload) = (None, None);
    let mut fields = Fields::new(input);
    while let Some(field) = fields.next()? {
        match field.number {
            1 => set_once(
                &mut port,
                varint(field, "Data.portnum must be varint")?,
                "Data.portnum",
            )?,
            2 => set_once(
                &mut payload,
                bytes(field, "Data.payload must be length-delimited")?,
                "Data.payload",
            )?,
            _ => {}
        }
    }
    if port != Some(TEXT_MESSAGE_APP) {
        return Ok(None);
    }
    let Some(payload) = payload else {
        return Ok(None);
    };
    let text = validate_text(payload)?;
    Ok(Some(UnverifiedTextObservation {
        source_node_unverified: source,
        destination_node_unverified: destination,
        channel_index_unverified: channel,
        text_unverified: text,
    }))
}

fn validate_text(payload: &[u8]) -> Result<String, DecodeError> {
    if payload.len() > MAX_TEXT_BYTES {
        return Err(DecodeError::InvalidText("text exceeds 233 bytes"));
    }
    let text = std::str::from_utf8(payload)
        .map_err(|_| DecodeError::InvalidText("text is not valid UTF-8"))?;
    if text.chars().any(is_forbidden_text_char) {
        return Err(DecodeError::InvalidText(
            "text contains controls or bidi controls",
        ));
    }
    Ok(text.to_owned())
}

fn is_forbidden_text_char(c: char) -> bool {
    c.is_control()
        || matches!(c as u32,
            0x061c | 0x200e..=0x200f | 0x202a..=0x202e | 0x2066..=0x2069 | 0x200b..=0x200d | 0xfeff)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Golden fixture encoded from Meshtastic protobuf schema
    // v2.8.0 (upstream commit 7b2464c): FromRadio.packet=2, MeshPacket.from=1,
    // to=2, channel=3, decoded=4; Data.portnum=1 (TEXT_MESSAGE_APP=1),
    // payload=2. Values: from=1, to=2, channel=0, text="hi".
    const GOLDEN: &[u8] = &[
        0x12, 0x0c, 0x08, 0x01, 0x10, 0x02, 0x22, 0x06, 0x08, 0x01, 0x12, 0x02, b'h', b'i',
    ];

    #[test]
    fn decodes_schema_golden_text_as_unverified() {
        assert_eq!(
            decode_recorded_from_radio(GOLDEN).unwrap(),
            Some(UnverifiedTextObservation {
                source_node_unverified: Some(1),
                destination_node_unverified: Some(2),
                channel_index_unverified: None,
                text_unverified: "hi".into(),
            })
        );
    }

    #[test]
    fn ignores_non_text_apps_and_missing_packet() {
        assert_eq!(decode_recorded_from_radio(&[]).unwrap(), None);
        assert_eq!(
            decode_recorded_from_radio(&[0x12, 0x02, 0x22, 0x00]).unwrap(),
            None
        );
        assert_eq!(
            decode_recorded_from_radio(&[0x12, 0x06, 0x22, 0x04, 0x08, 0x02, 0x12, 0x00]).unwrap(),
            None
        );
    }

    #[test]
    fn skips_well_formed_unknown_fields() {
        let mut input = GOLDEN.to_vec();
        input.extend([0x78, 0x07]); // unknown field 15, varint
        assert!(decode_recorded_from_radio(&input).unwrap().is_some());
    }

    #[test]
    fn rejects_duplicate_interpreted_fields_and_oneof_conflicts() {
        let mut duplicate_packet = GOLDEN.to_vec();
        duplicate_packet.extend([0x12, 0x00]);
        assert!(matches!(
            decode_recorded_from_radio(&duplicate_packet),
            Err(DecodeError::Duplicate(_))
        ));
        let packet = [0x22, 0x00, 0x2a, 0x00];
        let mut input = vec![0x12, packet.len() as u8];
        input.extend(packet);
        assert!(matches!(
            decode_recorded_from_radio(&input),
            Err(DecodeError::Duplicate(_))
        ));
        // Duplicate Data.portnum within the decoded nested message.
        let duplicate_port = [
            0x12, 0x09, 0x22, 0x07, 0x08, 0x01, 0x08, 0x01, 0x12, 0x01, b'x',
        ];
        assert!(matches!(
            decode_recorded_from_radio(&duplicate_port),
            Err(DecodeError::Duplicate(_))
        ));
    }

    #[test]
    fn rejects_malformed_wire_and_boundaries() {
        for input in [
            &[0x80][..],
            &[0x12, 0x80],
            &[0x12, 0x02, 0x08],
            &[0x0b],
            &[0x0c],
            &[0x0f],
        ] {
            assert!(
                decode_recorded_from_radio(input).is_err(),
                "accepted {input:?}"
            );
        }
        assert_eq!(
            decode_recorded_from_radio(&vec![0; 513]),
            Err(DecodeError::CaptureTooLarge)
        );
        let overflow = [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02];
        assert!(decode_recorded_from_radio(&overflow).is_err());
    }

    #[test]
    fn rejects_oversized_invalid_or_spoofing_text() {
        // Capture cap is checked before text cap, so use an otherwise valid
        // compact nested encoding at the 234-byte boundary by testing helper.
        assert!(matches!(
            validate_text(&vec![b'a'; 234]),
            Err(DecodeError::InvalidText(_))
        ));
        assert!(matches!(
            validate_text(&[0xff]),
            Err(DecodeError::InvalidText(_))
        ));
        assert!(matches!(
            validate_text("hello\u{202e}".as_bytes()),
            Err(DecodeError::InvalidText(_))
        ));
        assert!(matches!(
            validate_text(b"a\nb"),
            Err(DecodeError::InvalidText(_))
        ));
        assert!(validate_text(&vec![b'a'; 233]).is_ok());
    }
}
