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

/* P1.2 virtual-time congestion and seeded loss harness.
 * Not connected to the production Messenger and not evidence of fsync.
 */
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkConfig {
    pub seed: u64,
    pub loss_percent: u8,
    pub delay_ticks: u64,
    pub max_inflight: usize,
    pub retry_base_ticks: u64,
    pub retry_cap_ticks: u64,
}
impl LinkConfig {
    pub fn valid(self) -> bool {
        self.loss_percent <= 100
            && self.max_inflight > 0
            && self.retry_base_ticks > 0
            && self.retry_cap_ticks >= self.retry_base_ticks
    }
}
#[derive(Clone, Copy, Debug)]
struct Inflight {
    packet: Packet,
    ready_at: u64,
}
#[derive(Debug)]
pub struct TimedLink {
    config: LinkConfig,
    rng: u64,
    tick: u64,
    waiting: VecDeque<Packet>,
    flight: VecDeque<Inflight>,
    seen: BTreeSet<(u16, u64)>,
    next_retry: u64,
    failures: u32,
    pub lost: usize,
    pub accepted: usize,
    pub duplicates: usize,
    pub attempts: usize,
}
impl TimedLink {
    pub fn new(config: LinkConfig, packets: impl IntoIterator<Item = Packet>) -> Option<Self> {
        if !config.valid() { return None; }
        Some(Self {
            rng: config.seed,
            config,
            tick: 0,
            waiting: packets.into_iter().collect(),
            flight: VecDeque::new(),
            seen: BTreeSet::new(),
            next_retry: 0,
            failures: 0,
            lost: 0,
            accepted: 0,
            duplicates: 0,
            attempts: 0,
        })
    }
    pub fn pending(&self) -> usize { self.waiting.len() + self.flight.len() }
    pub fn in_flight(&self) -> usize { self.flight.len() }
    pub fn tick(&self) -> u64 { self.tick }
    fn random_percent(&mut self) -> u8 {
        // Reproducible simulation PRNG, explicitly not cryptographic.
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.rng >> 32).wrapping_rem(100) as u8
    }
    pub fn advance(&mut self, authenticated: bool, link_available: bool) {
        self.tick = self.tick.saturating_add(1);
        // Acknowledged storage is modeled only at successful receiver arrival.
        // A failed endpoint check leaves all work intact and in flight unconfirmed.
        if !authenticated || !link_available { return; }
        while self.flight.front().is_some_and(|p| p.ready_at <= self.tick) {
            let p = self.flight.pop_front().expect("front checked");
            if self.seen.insert((p.packet.origin, p.packet.sequence)) {
                self.accepted += 1;
            } else {
                self.duplicates += 1;
            }
        }
        if self.tick < self.next_retry { return; }
        while self.flight.len() < self.config.max_inflight {
            let Some(packet) = self.waiting.pop_front() else { break; };
            self.attempts += 1;
            if self.random_percent() < self.config.loss_percent {
                self.lost += 1;
                self.waiting.push_front(packet);
                self.failures = self.failures.saturating_add(1);
                let shift = self.failures.saturating_sub(1).min(20);
                let delay = self.config.retry_base_ticks.saturating_mul(1_u64 << shift)
                    .min(self.config.retry_cap_ticks);
                self.next_retry = self.tick.saturating_add(delay);
                break;
            }
            self.failures = 0;
            self.flight.push_back(Inflight {
                packet,
                ready_at: self.tick.saturating_add(self.config.delay_ticks).saturating_add(1),
            });
        }
    }
}
#[cfg(test)]
mod timed_tests {
    use super::*;
    fn cfg(seed: u64) -> LinkConfig {
        LinkConfig { seed, loss_percent: 30, delay_ticks: 2, max_inflight: 3,
            retry_base_ticks: 1, retry_cap_ticks: 16 }
    }
    #[test]
    fn seeded_runs_match_exactly() {
        let make = || (0..24).map(|n| Packet { origin: n, sequence: 1 });
        let mut a = TimedLink::new(cfg(42), make()).unwrap();
        let mut b = TimedLink::new(cfg(42), make()).unwrap();
        for _ in 0..500 {
            a.advance(true, true); b.advance(true, true);
            assert_eq!((a.pending(), a.accepted, a.lost, a.attempts),
                       (b.pending(), b.accepted, b.lost, b.attempts));
        }
        assert_eq!(a.accepted, 24);
    }
    #[test]
    fn congestion_is_bounded_and_delay_is_real() {
        let mut a = TimedLink::new(LinkConfig { loss_percent: 0, ..cfg(1) },
            (0..20).map(|n| Packet { origin: n, sequence: 1 })).unwrap();
        a.advance(true, true);
        assert_eq!(a.accepted, 0);
        assert_eq!(a.in_flight(), 3);
        for _ in 0..200 {
            a.advance(true, true);
            assert!(a.in_flight() <= 3);
        }
        assert_eq!(a.accepted, 20);
    }
    #[test]
    fn isolated_or_unauthenticated_cannot_accept() {
        let mut a = TimedLink::new(cfg(4), [Packet { origin: 2, sequence: 9 }]).unwrap();
        for _ in 0..100 {
            a.advance(false, true); a.advance(true, false);
        }
        assert_eq!((a.accepted, a.pending()), (0, 1));
    }
    #[test]
    fn loss_holds_queue_without_false_receipt() {
        let mut a = TimedLink::new(LinkConfig { loss_percent: 100, ..cfg(12) },
            [Packet { origin: 1, sequence: 1 }]).unwrap();
        for _ in 0..100 { a.advance(true, true); }
        assert_eq!(a.accepted, 0);
        assert_eq!(a.pending(), 1);
        assert!(a.attempts < 100);
        assert!(a.lost > 0);
    }
    #[test]
    fn invalid_configuration_is_rejected() {
        assert!(TimedLink::new(LinkConfig { max_inflight: 0, ..cfg(1) }, []).is_none());
        assert!(TimedLink::new(LinkConfig { loss_percent: 101, ..cfg(1) }, []).is_none());
    }
}
