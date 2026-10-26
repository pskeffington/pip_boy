//! Receive-only Meshtastic USB stream framing. No Write bound or ToRadio API.
//! Payloads are UNTRUSTED opaque FromRadio protobuf bytes, never chat receipts.
use std::io::{self, Read};

pub const MAX_FROM_RADIO: usize = 512;
#[derive(Default)]
pub struct PassiveDecoder {
    buffer: Vec<u8>,
}
impl PassiveDecoder {
    /// Accept fragmented/noisy USB input with bounded state. No port auto-open.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        let mut frames = Vec::new();
        for byte in bytes {
            self.buffer.push(*byte);
            loop {
                if self.buffer.first() != Some(&0x94) {
                    self.buffer.clear();
                    break;
                }
                if self.buffer.len() == 1 {
                    break;
                }
                if self.buffer[1] != 0xc3 {
                    self.buffer.remove(0);
                    continue;
                }
                if self.buffer.len() < 4 {
                    break;
                }
                let len = u16::from_be_bytes([self.buffer[2], self.buffer[3]]) as usize;
                if len == 0 || len > MAX_FROM_RADIO {
                    self.buffer.remove(0);
                    continue;
                }
                if self.buffer.len() < len + 4 {
                    break;
                }
                frames.push(self.buffer[4..].to_vec());
                self.buffer.clear();
                break;
            }
        }
        frames
    }
}

/// Caller owns explicit USB setup, read timeout and device selection. A passive
/// capture may stay silent until another authorized client requests streaming.
pub fn read_once(reader: &mut impl Read, decoder: &mut PassiveDecoder) -> io::Result<Vec<Vec<u8>>> {
    let mut bytes = [0; MAX_FROM_RADIO + 4];
    let n = reader.read(&mut bytes)?;
    Ok(decoder.feed(&bytes[..n]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fragmented_noisy_coalesced_and_oversized_capture() {
        let mut decoder = PassiveDecoder::default();
        assert!(decoder.feed(&[b'\n', 0x94, 0x94, 0xc3, 0]).is_empty());
        assert_eq!(
            decoder.feed(&[2, 8, 1, 0x94, 0xc3, 0, 1, 7]),
            vec![vec![8, 1], vec![7]]
        );
        assert!(decoder.feed(&[0x94, 0xc3, 2, 1]).is_empty());
        assert_eq!(decoder.feed(&[0x94, 0xc3, 0, 1, 9]), vec![vec![9]]);
        assert!(decoder.buffer.len() <= MAX_FROM_RADIO + 4);
        let mut capture = io::Cursor::new([0x94, 0xc3, 0, 1, 42]);
        assert_eq!(
            read_once(&mut capture, &mut decoder).unwrap(),
            vec![vec![42]]
        );
    }
}
