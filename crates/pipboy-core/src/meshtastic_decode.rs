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
    pub source_node_unverified: Option<u32>,
    /// Destination node number as recorded in the capture; not authenticated.
    pub destination_node_unverified: Option<u32>,
    /// Channel index as recorded in the capture; not authenticated.
    pub channel_index_unverified: Option<u32>,
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
    Fixed32(u32),
    Fixed64,
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
        let number_wide = key >> 3;
        if number_wide == 0 || number_wide > 536_870_911 {
            return Err(DecodeError::Malformed("invalid protobuf field number"));
        }
        let number = number_wide as u32;
        let wire = (key & 7) as u8;
        if (19000..=19999).contains(&number) {
            return Err(DecodeError::Malformed("invalid protobuf field number"));
        }
        let value = match wire {
            0 => Value::Varint(read_varint(self.input, &mut self.offset)?),
            1 => {
                take(self.input, &mut self.offset, 8)?;
                Value::Fixed64
            }
            2 => {
                let len = read_varint(self.input, &mut self.offset)?;
                let len =
                    usize::try_from(len).map_err(|_| DecodeError::Malformed("length overflow"))?;
                Value::Bytes(take(self.input, &mut self.offset, len)?)
            }
            5 => {
                let bytes = take(self.input, &mut self.offset, 4)?;
                let bytes: [u8; 4] = bytes
                    .try_into()
                    .map_err(|_| DecodeError::Malformed("invalid fixed32 field"))?;
                Value::Fixed32(u32::from_le_bytes(bytes))
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

fn fixed32(field: Field<'_>, label: &'static str) -> Result<u32, DecodeError> {
    if field.wire != 5 {
        return Err(DecodeError::Malformed(label));
    }
    match field.value {
        Value::Fixed32(value) => Ok(value),
        _ => Err(DecodeError::Malformed(label)),
    }
}

fn uint32(value: u64, label: &'static str) -> Result<u32, DecodeError> {
    u32::try_from(value).map_err(|_| DecodeError::Malformed(label))
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
    let (mut id, mut packet, mut oneof_seen) = (None, None, false);
    let mut fields = Fields::new(input);
    while let Some(field) = fields.next()? {
        match field.number {
            1 => set_once(
                &mut id,
                uint32(
                    varint(field, "FromRadio.id must be uint32")?,
                    "FromRadio.id overflow",
                )?,
                "FromRadio.id",
            )?,
            2..=17 => {
                let expected_wire = if matches!(field.number, 7 | 8) { 0 } else { 2 };
                if field.wire != expected_wire {
                    return Err(DecodeError::Malformed("invalid FromRadio oneof wire type"));
                }
                if oneof_seen {
                    return Err(DecodeError::Duplicate("FromRadio.payload_variant oneof"));
                }
                oneof_seen = true;
                if field.number == 2 {
                    packet = Some(bytes(field, "FromRadio.packet must be length-delimited")?);
                }
            }
            _ => {}
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
                fixed32(field, "MeshPacket.from must be fixed32")?,
                "MeshPacket.from",
            )?,
            2 => set_once(
                &mut destination,
                fixed32(field, "MeshPacket.to must be fixed32")?,
                "MeshPacket.to",
            )?,
            3 => set_once(
                &mut channel,
                uint32(
                    varint(field, "MeshPacket.channel must be uint32")?,
                    "MeshPacket.channel overflow",
                )?,
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
    source: Option<u32>,
    destination: Option<u32>,
    channel: Option<u32>,
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

    // Golden fixture encoded from Meshtastic protobuf schema v2.8.0
    // (upstream commit 7b2464c9b8c1521f93852261e4123826e5b25e11):
    // FromRadio.packet=2, MeshPacket.from=1, MeshPacket.to=2 are fixed32;
    // channel=3 and Data.portnum=1
    // are uint32; decoded=4 and Data.payload=2 are length-delimited.
    // Values: from=1, to=2, channel=0, text="hi".
    const GOLDEN: &[u8] = &[
        0x12, 0x12, 0x0d, 0x01, 0x00, 0x00, 0x00, 0x15, 0x02, 0x00, 0x00, 0x00, 0x22, 0x06, 0x08,
        0x01, 0x12, 0x02, b'h', b'i',
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
        input.extend([0xa0, 0x01, 0x07]); // unknown field 20, varint
        assert!(decode_recorded_from_radio(&input).unwrap().is_some());
    }

    #[test]
    fn rejects_field_number_aliases_and_packet_oneof_conflicts() {
        let mut aliased_field_number = Vec::new();
        let mut key = ((u64::from(u32::MAX) + 3) << 3) | 2;
        while key >= 0x80 {
            aliased_field_number.push((key as u8 & 0x7f) | 0x80);
            key >>= 7;
        }
        aliased_field_number.push(key as u8);
        aliased_field_number.push(0); // zero-length payload for oversized field
        assert!(matches!(
            decode_recorded_from_radio(&aliased_field_number),
            Err(DecodeError::Malformed(_))
        ));

        // FromRadio.my_info (field 3) conflicts with packet (field 2), in
        // either wire order. The parser must not apply last-oneof-wins silently.
        let mut packet_then_info = GOLDEN.to_vec();
        packet_then_info.extend([0x1a, 0x00]);
        assert!(matches!(
            decode_recorded_from_radio(&packet_then_info),
            Err(DecodeError::Duplicate(_))
        ));
        let mut info_then_packet = vec![0x1a, 0x00];
        info_then_packet.extend_from_slice(GOLDEN);
        assert!(matches!(
            decode_recorded_from_radio(&info_then_packet),
            Err(DecodeError::Duplicate(_))
        ));
    }

    #[test]
    fn enforces_fixed32_node_fields_and_uint32_channel() {
        let invalid_from_wire = [
            0x12, 0x0a, 0x08, 0x01, 0x22, 0x06, 0x08, 0x01, 0x12, 0x02, b'h', b'i',
        ];
        assert!(matches!(
            decode_recorded_from_radio(&invalid_from_wire),
            Err(DecodeError::Malformed(_))
        ));

        let mut oversized_channel = GOLDEN.to_vec();
        // Add MeshPacket.channel=2^32, then repair enclosing lengths by
        // building a complete valid nested message explicitly.
        let packet = [
            0x0d, 1, 0, 0, 0, 0x15, 2, 0, 0, 0, 0x18, 0x80, 0x80, 0x80, 0x80, 0x10, 0x22, 0x06,
            0x08, 0x01, 0x12, 0x02, b'h', b'i',
        ];
        oversized_channel.clear();
        oversized_channel.extend([0x12, packet.len() as u8]);
        oversized_channel.extend(packet);
        assert!(matches!(
            decode_recorded_from_radio(&oversized_channel),
            Err(DecodeError::Malformed(_))
        ));
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
