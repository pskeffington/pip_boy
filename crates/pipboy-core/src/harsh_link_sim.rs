//! Deterministic, offline harsh-link fault simulator for public-demo packets.
//! No sockets, clocks, devices, authentication grants, or durable-storage claims.
#![forbid(unsafe_code)]
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    Clear,
    CableCut,
    TunnelBlackspot,
    OneWay,
    SilentRelay,
    Duplicate,
    Reorder,
    PowerLoss,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delivery {
    Queued,
    TransmittedUnconfirmed,
    StoredAtPeer,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Packet {
    pub origin: u16,
    pub sequence: u64,
}
#[derive(Debug)]
pub struct Simulation {
    pending: VecDeque<Packet>,
    seen: BTreeSet<(u16, u64)>,
    pub attempted: usize,
    pub stored: usize,
    pub duplicates: usize,
    pub unconfirmed: usize,
}
impl Simulation {
    pub fn new(packets: impl IntoIterator<Item = Packet>) -> Self {
        Self {
            pending: packets.into_iter().collect(),
            seen: BTreeSet::new(),
            attempted: 0,
            stored: 0,
            duplicates: 0,
            unconfirmed: 0,
        }
    }
    pub fn pending(&self) -> usize { self.pending.len() }
    pub fn deliver(&mut self, fault: Fault, peer_authenticated: bool) -> Delivery {
        let Some(packet) = self.pending.front().copied() else { return Delivery::Queued; };
        if !peer_authenticated || matches!(fault, Fault::CableCut | Fault::TunnelBlackspot | Fault::SilentRelay | Fault::PowerLoss) {
            return Delivery::Queued;
        }
        self.attempted += 1;
        if matches!(fault, Fault::OneWay) {
            self.unconfirmed += 1;
            return Delivery::TransmittedUnconfirmed;
        }
        self.pending.pop_front();
        if !self.seen.insert((packet.origin, packet.sequence)) {
            self.duplicates += 1;
        } else {
            self.stored += 1;
        }
        if matches!(fault, Fault::Duplicate) {
            if !self.seen.insert((packet.origin, packet.sequence)) {
                self.duplicates += 1;
            }
        }
        if matches!(fault, Fault::Reorder) && self.pending.len() > 1 {
            let first = self.pending.pop_front().expect("checked length");
            self.pending.push_back(first);
        }
        Delivery::StoredAtPeer
    }
    /// Reboot restores a hypothetical in-memory pending queue, not disk persistence.
    pub fn restart(&mut self) {
        self.attempted = 0;
        self.unconfirmed = 0;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn packet(seq: u64) -> Packet { Packet { origin: 1, sequence: seq } }
    #[test]
    fn physical_link_failures_hold_queue() {
        for fault in [Fault::CableCut, Fault::TunnelBlackspot, Fault::SilentRelay, Fault::PowerLoss] {
            let mut sim = Simulation::new([packet(1)]);
            assert_eq!(sim.deliver(fault, true), Delivery::Queued);
            assert_eq!((sim.pending(), sim.stored), (1, 0));
        }
    }
    #[test]
    fn unauthenticated_radio_never_delivers() {
        let mut sim = Simulation::new([packet(1)]);
        assert_eq!(sim.deliver(Fault::Clear, false), Delivery::Queued);
        assert_eq!(sim.stored, 0);
    }
    #[test]
    fn one_way_link_is_not_durable_receipt() {
        let mut sim = Simulation::new([packet(1)]);
        assert_eq!(sim.deliver(Fault::OneWay, true), Delivery::TransmittedUnconfirmed);
        assert_eq!(sim.pending(), 1);
        assert_eq!(sim.stored, 0);
    }
    #[test]
    fn duplicate_is_applied_once() {
        let mut sim = Simulation::new([packet(1), packet(1)]);
        sim.deliver(Fault::Duplicate, true);
        sim.deliver(Fault::Clear, true);
        assert_eq!(sim.stored, 1);
        assert_eq!(sim.duplicates, 2);
    }
    #[test]
    fn reordered_and_mass_rejoined_devices_complete() {
        let packets = (0..20).map(|origin| Packet { origin, sequence: 1 });
        let mut sim = Simulation::new(packets);
        assert_eq!(sim.deliver(Fault::Reorder, true), Delivery::StoredAtPeer);
        while sim.pending() > 0 { sim.deliver(Fault::Clear, true); }
        assert_eq!(sim.stored, 20);
        assert_eq!(sim.pending(), 0);
    }
    #[test]
    fn restart_does_not_claim_receiver_has_data() {
        let mut sim = Simulation::new([packet(1)]);
        sim.deliver(Fault::OneWay, true);
        sim.restart();
        assert_eq!(sim.pending(), 1);
        assert_eq!(sim.stored, 0);
    }
}
