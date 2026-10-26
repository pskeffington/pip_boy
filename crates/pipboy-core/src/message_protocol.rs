//! Canonical Messenger v1 wire contract. No transport can confer human read status.
use crate::messenger::{AttemptOutcome, AttemptRoute, Delivery, MessageStore, RemoteMessage};
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
    if pending.delivery != Delivery::Queued
        || pending.body != frame.body
        || pending.created_at != frame.sent_at
    {
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

/// Persist evidence before contacting the peer. A crash or failed final write
/// leaves Started (unknown outcome), never a fabricated delivery or safe-to-retry
/// claim. Retrying is always explicit and uses the original immutable envelope.
pub fn transmit_on_route(
    transport: &mut impl MessageTransport,
    journal: &MessageStore,
    frame: &Envelope,
    route: AttemptRoute,
) -> io::Result<StoredReceipt> {
    let _dispatch = journal.dispatch_lock()?;
    let pending = journal.outgoing(frame.origin_id, &frame.to_device)?;
    if pending.delivery != Delivery::Queued
        || pending.body != frame.body
        || pending.created_at != frame.sent_at
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Envelope differs from pending record",
        ));
    }
    frame.encode()?;
    let started = journal.start_attempt(frame.origin_id, &frame.to_device, route)?;
    let attempts = started
        .progress
        .as_ref()
        .expect("attempt progress initialized")
        .attempts;
    let receipt = match transport.exchange(frame) {
        Ok(receipt) => receipt,
        Err(error) => {
            let outcome = if error.kind() == io::ErrorKind::InvalidData {
                AttemptOutcome::ReceiptRejected
            } else {
                AttemptOutcome::ExchangeUnconfirmed
            };
            journal.fail_attempt(frame.origin_id, &frame.to_device, attempts, outcome)?;
            return Err(error);
        }
    };
    if let Err(error) = receipt.validate_for(frame) {
        journal.fail_attempt(
            frame.origin_id,
            &frame.to_device,
            attempts,
            AttemptOutcome::ReceiptRejected,
        )?;
        return Err(error);
    }
    // Receipt and successful attempt outcome commit together in one atomic save.
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
    use crate::messenger::AttemptOutcome;
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

    #[test]
    fn tracked_lost_receipt_then_alternate_retry_preserves_identity_and_remote_record() {
        struct Relay<'a> {
            receiver: &'a MessageStore,
            lose_receipt: bool,
        }
        impl MessageTransport for Relay<'_> {
            fn exchange(&mut self, frame: &Envelope) -> io::Result<StoredReceipt> {
                let (stored, _) = self.receiver.receive(frame, "scout")?;
                if self.lose_receipt {
                    return Err(io::Error::new(io::ErrorKind::TimedOut, "lost receipt"));
                }
                Ok(StoredReceipt {
                    schema_version: 1,
                    kind: "message_stored".into(),
                    receiver_device: "scout".into(),
                    from_device: "overseer".into(),
                    origin_id: frame.origin_id,
                    remote_id: stored.id,
                })
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let key = age::x25519::Identity::generate();
        let sender = MessageStore::with_key(dir.path().join("sender"), key.clone());
        let receiver = MessageStore::new(dir.path().join("receiver"));
        let pending = sender.queue("scout", "immutable report").unwrap();
        let frame = Envelope {
            schema_version: 1,
            from_device: "overseer".into(),
            to_device: "scout".into(),
            origin_id: pending.id,
            body: pending.body.clone(),
            sent_at: pending.created_at,
        };
        let wire = frame.encode().unwrap();
        let mut relay = Relay {
            receiver: &receiver,
            lose_receipt: true,
        };
        assert!(transmit_on_route(&mut relay, &sender, &frame, AttemptRoute::Primary).is_err());
        assert_eq!(receiver.load().unwrap().messages.len(), 1);
        let reopened = MessageStore::with_key(dir.path().join("sender"), key);
        let failed = reopened.outgoing(pending.id, "scout").unwrap();
        let progress = failed.progress.unwrap();
        assert_eq!(progress.attempts, 1);
        assert_eq!(
            progress.last_attempt.unwrap().outcome,
            AttemptOutcome::ExchangeUnconfirmed
        );
        let held = reopened
            .hold(pending.id, "scout")
            .unwrap()
            .progress
            .unwrap();
        assert!(held.held_locally);
        assert_eq!(held.attempts, 1);
        relay.lose_receipt = false;
        let route = AttemptRoute::Alternate {
            ipv4: "192.168.1.20".parse().unwrap(),
        };
        let receipt = transmit_on_route(&mut relay, &reopened, &frame, route).unwrap();
        assert_eq!(receipt.remote_id, 1);
        assert_eq!(receiver.load().unwrap().messages.len(), 1);
        let stored = reopened.outgoing(pending.id, "scout").unwrap();
        assert_eq!(
            (stored.id, stored.body, stored.created_at),
            (pending.id, pending.body, pending.created_at)
        );
        let progress = stored.progress.unwrap();
        assert_eq!(progress.attempts, 2);
        assert!(!progress.held_locally);
        let last = progress.last_attempt.unwrap();
        assert_eq!(last.route, route);
        assert_eq!(last.outcome, AttemptOutcome::Stored);
        assert_eq!(frame.encode().unwrap(), wire);
        assert!(reopened.hold(pending.id, "scout").is_err());
        assert!(transmit_on_route(&mut relay, &reopened, &frame, route).is_err());
    }

    #[test]
    fn dispatch_lock_and_interrupted_attempt_do_not_trigger_automatic_transport() {
        struct Reject {
            calls: u32,
        }
        impl MessageTransport for Reject {
            fn exchange(&mut self, _: &Envelope) -> io::Result<StoredReceipt> {
                self.calls += 1;
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "malformed receipt",
                ))
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let sender = MessageStore::new(dir.path().into());
        let pending = sender.queue("scout", "report").unwrap();
        let frame = Envelope {
            schema_version: 1,
            from_device: "overseer".into(),
            to_device: "scout".into(),
            origin_id: pending.id,
            body: pending.body,
            sent_at: pending.created_at,
        };
        let mut transport = Reject { calls: 0 };
        let guard = sender.dispatch_lock().unwrap();
        assert!(transmit_on_route(&mut transport, &sender, &frame, AttemptRoute::Primary).is_err());
        assert!(sender.hold(pending.id, "scout").is_err());
        assert_eq!(transport.calls, 0);
        assert_eq!(
            sender
                .outgoing(pending.id, "scout")
                .unwrap()
                .progress
                .unwrap()
                .attempts,
            0
        );
        // Model interruption after the durable start record, before any receipt.
        sender
            .start_attempt(pending.id, "scout", AttemptRoute::Primary)
            .unwrap();
        drop(guard);
        let reopened = MessageStore::new(dir.path().into());
        assert_eq!(
            reopened.load().unwrap().messages[0]
                .progress
                .as_ref()
                .unwrap()
                .last_attempt
                .as_ref()
                .unwrap()
                .outcome,
            AttemptOutcome::Started
        );
        assert_eq!(transport.calls, 0);
        assert!(
            transmit_on_route(&mut transport, &reopened, &frame, AttemptRoute::Primary).is_err()
        );
        let progress = reopened
            .outgoing(pending.id, "scout")
            .unwrap()
            .progress
            .unwrap();
        assert_eq!(progress.attempts, 2);
        assert_eq!(
            progress.last_attempt.unwrap().outcome,
            AttemptOutcome::ReceiptRejected
        );
        assert_eq!(transport.calls, 1);
        assert!(reopened
            .fail_attempt(pending.id, "scout", 1, AttemptOutcome::ExchangeUnconfirmed)
            .is_err());
    }

    #[test]
    fn tracked_mismatched_receipt_is_recorded_without_trusting_remote_status() {
        struct Forged;
        impl MessageTransport for Forged {
            fn exchange(&mut self, frame: &Envelope) -> io::Result<StoredReceipt> {
                Ok(StoredReceipt {
                    schema_version: 1,
                    kind: "message_stored".into(),
                    receiver_device: "wrong-device".into(),
                    from_device: frame.from_device.clone(),
                    origin_id: frame.origin_id,
                    remote_id: 1,
                })
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let sender = MessageStore::new(dir.path().into());
        let pending = sender.queue("scout", "report").unwrap();
        let frame = Envelope {
            schema_version: 1,
            from_device: "overseer".into(),
            to_device: "scout".into(),
            origin_id: pending.id,
            body: pending.body,
            sent_at: pending.created_at,
        };
        assert!(transmit_on_route(&mut Forged, &sender, &frame, AttemptRoute::Primary).is_err());
        let message = sender.outgoing(pending.id, "scout").unwrap();
        assert_eq!(message.delivery, Delivery::Queued);
        assert_eq!(message.remote_stored_id, None);
        assert_eq!(
            message.progress.unwrap().last_attempt.unwrap().outcome,
            AttemptOutcome::ReceiptRejected
        );
    }
}
