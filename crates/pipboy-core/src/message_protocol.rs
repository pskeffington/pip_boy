//! Canonical Messenger v1 wire contract. No transport can confer human read status.
use crate::messenger::{MessageStore, RemoteMessage};
use serde::{Deserialize, Serialize};
use std::io;

pub const MAX_ENVELOPE: usize = 8192;
pub type Envelope = RemoteMessage;

impl Envelope {
    /// Fixed struct-field order, compact UTF-8 JSON; no signing of arbitrary JSON.
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        if !self.valid() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid envelope",
            ));
        }
        Ok(serde_json::to_vec(self)?)
    }

    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() > MAX_ENVELOPE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Envelope too large",
            ));
        }
        let result: Self = serde_json::from_slice(bytes)?;
        result.encode()?;
        Ok(result)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredReceipt {
    pub schema_version: u32,
    pub kind: String,
    pub receiver_device: String,
    pub from_device: String,
    pub origin_id: u64,
    pub remote_id: u64,
}
impl StoredReceipt {
    pub fn validate_for(&self, frame: &Envelope) -> io::Result<()> {
        if !frame.valid()
            || self.schema_version != 1
            || self.kind != "message_stored"
            || self.receiver_device != frame.to_device
            || self.from_device != frame.from_device
            || self.origin_id != frame.origin_id
            || self.remote_id == 0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Mismatched storage receipt",
            ));
        }
        Ok(())
    }
}

/// Implementors MUST authenticate the sender independently of envelope labels,
/// bound IO and deadlines, and return only a durable receiver receipt. No retries
/// or discovery are implicit. Passive radio capture cannot implement this trait.
pub trait MessageTransport {
    fn exchange(&mut self, frame: &Envelope) -> io::Result<StoredReceipt>;
}

pub fn transmit(
    transport: &mut impl MessageTransport,
    journal: &MessageStore,
    frame: &Envelope,
) -> io::Result<StoredReceipt> {
    let pending = journal.outgoing(frame.origin_id, &frame.to_device)?;
    if pending.body != frame.body || pending.created_at != frame.sent_at {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Envelope differs from queued record",
        ));
    }
    frame.encode()?;
    let receipt = transport.exchange(frame)?;
    receipt.validate_for(frame)?;
    journal.mark_stored(frame.origin_id, &frame.to_device, receipt.remote_id)?;
    Ok(receipt)
}

/// Explicit virtual-clock fault schedule; no random seed, sockets or sleeps.
/// A list of offsets emits copies in time order; [] drops the whole attempt.
#[derive(Default)]
pub struct FaultSimulator {
    now: u64,
    sequence: u64,
    events: Vec<(u64, u64, Envelope, bool)>,
}
impl FaultSimulator {
    pub fn schedule(
        &mut self,
        frame: &Envelope,
        delays: &[u64],
        lose_receipt: bool,
    ) -> io::Result<()> {
        frame.encode()?;
        if delays.len() > 16 || self.events.len() + delays.len() > 256 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Simulation capacity exceeded",
            ));
        }
        let times = delays
            .iter()
            .map(|d| {
                self.now.checked_add(*d).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Virtual clock overflow")
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        for time in times {
            self.events
                .push((time, self.sequence, frame.clone(), lose_receipt));
            self.sequence += 1;
        }
        Ok(())
    }

    pub fn advance(
        &mut self,
        until: u64,
        receiver: &MessageStore,
        local: &str,
        authenticated_sender: &str,
    ) -> io::Result<Vec<StoredReceipt>> {
        if until < self.now {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Clock cannot reverse",
            ));
        }
        self.now = until;
        self.events
            .sort_by_key(|(time, order, _, _)| (*time, *order));
        let mut receipts = Vec::new();
        while self.events.first().is_some_and(|e| e.0 <= until) {
            let (_, _, frame, lose) = self.events.remove(0);
            if frame.from_device != authenticated_sender {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Unauthenticated simulator sender",
                ));
            }
            let (message, _) = receiver.receive(&frame, local)?;
            if !lose {
                receipts.push(StoredReceipt {
                    schema_version: 1,
                    kind: "message_stored".into(),
                    receiver_device: local.into(),
                    from_device: frame.from_device,
                    origin_id: frame.origin_id,
                    remote_id: message.id,
                });
            }
        }
        Ok(receipts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(id: u64) -> Envelope {
        Envelope {
            schema_version: 1,
            from_device: "overseer".into(),
            to_device: "scout".into(),
            origin_id: id,
            body: format!("message {id}"),
            sent_at: 42,
        }
    }
    #[test]
    fn golden_envelope_and_strict_decode() {
        let bytes = frame(1).encode().unwrap();
        assert_eq!(
            String::from_utf8(bytes.clone()).unwrap(),
            r#"{"schema_version":1,"from_device":"overseer","to_device":"scout","origin_id":1,"body":"message 1","sent_at":42}"#
        );
        assert_eq!(Envelope::decode(&bytes).unwrap().origin_id, 1);
        for bad in [
            b"{}".as_slice(),
            b"{\"schema_version\":1,\"schema_version\":1}".as_slice(),
        ] {
            assert!(Envelope::decode(bad).is_err());
        }
        assert!(Envelope::decode(&vec![0; MAX_ENVELOPE + 1]).is_err());
    }
    #[test]
    fn faults_are_repeatable_and_lost_receipts_do_not_duplicate_storage() {
        let run = || {
            let dir = tempfile::tempdir().unwrap();
            let receiver = MessageStore::new(dir.path().into());
            let mut sim = FaultSimulator::default();
            sim.schedule(&frame(1), &[], false).unwrap(); // dropped
            sim.schedule(&frame(2), &[20, 21], false).unwrap(); // delayed + duplicated
            sim.schedule(&frame(3), &[2], true).unwrap(); // reordered, receipt lost
            assert!(sim
                .advance(1, &receiver, "scout", "overseer")
                .unwrap()
                .is_empty());
            assert!(receiver.load().unwrap().messages.is_empty());
            assert!(sim
                .advance(2, &receiver, "scout", "overseer")
                .unwrap()
                .is_empty());
            sim.schedule(&frame(3), &[1], false).unwrap(); // retry same immutable envelope
            let mut receipts = sim.advance(3, &receiver, "scout", "overseer").unwrap();
            receipts.extend(sim.advance(21, &receiver, "scout", "overseer").unwrap());
            assert_eq!(receiver.load().unwrap().messages.len(), 2);
            assert_eq!(receipts[1].remote_id, receipts[2].remote_id);
            assert!(sim.advance(0, &receiver, "scout", "overseer").is_err());
            receipts
        };
        assert_eq!(run(), run());
    }
    #[test]
    fn failed_transport_and_forged_receipt_leave_sender_queued() {
        struct Stub {
            forge: bool,
        }
        impl MessageTransport for Stub {
            fn exchange(&mut self, frame: &Envelope) -> io::Result<StoredReceipt> {
                if !self.forge {
                    return Err(io::Error::new(io::ErrorKind::TimedOut, "lost receipt"));
                }
                Ok(StoredReceipt {
                    schema_version: 1,
                    kind: "message_stored".into(),
                    receiver_device: "wrong-peer".into(),
                    from_device: frame.from_device.clone(),
                    origin_id: frame.origin_id,
                    remote_id: 1,
                })
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let store = MessageStore::new(dir.path().into());
        let queued = store.queue("scout", "awaiting receipt").unwrap();
        let frame = Envelope {
            schema_version: 1,
            from_device: "overseer".into(),
            to_device: "scout".into(),
            origin_id: queued.id,
            body: queued.body,
            sent_at: queued.created_at,
        };
        for forge in [false, true] {
            assert!(transmit(&mut Stub { forge }, &store, &frame).is_err());
            assert_eq!(
                store.outgoing(queued.id, "scout").unwrap().delivery,
                crate::messenger::Delivery::Queued
            );
        }
    }

    #[test]
    fn authentication_and_receipt_binding_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let store = MessageStore::new(dir.path().into());
        let mut sim = FaultSimulator::default();
        sim.schedule(&frame(1), &[0], false).unwrap();
        assert!(sim.advance(0, &store, "scout", "impostor").is_err());
        assert!(store.load().unwrap().messages.is_empty());
        let receipt = StoredReceipt {
            schema_version: 1,
            kind: "message_stored".into(),
            receiver_device: "scout".into(),
            from_device: "overseer".into(),
            origin_id: 2,
            remote_id: 1,
        };
        assert!(receipt.validate_for(&frame(1)).is_err());
    }
}
