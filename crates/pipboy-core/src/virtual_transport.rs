//! Bounded, memory-only transport for simulated events. No physical I/O.
use crate::capability::Capability;
use crate::device_simulator::{
    BrokerError, SessionConfirmation, SimulatedBroker, SimulationEvent, SimulationLease,
};
use crate::permissions::PreferenceEntry;
use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

pub const MAX_TRANSPORT_CAPACITY: usize = 64;
pub const MAX_TRANSPORT_PUMP_COUNT: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransportError {
    InvalidCapacity,
    InvalidCount,
    Broker(BrokerError),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCapacity => f.write_str("virtual transport capacity must be 1..=64"),
            Self::InvalidCount => f.write_str("virtual transport count must be 1..=128"),
            Self::Broker(e) => e.fmt(f),
        }
    }
}

impl Error for TransportError {}

impl From<BrokerError> for TransportError {
    fn from(value: BrokerError) -> Self {
        Self::Broker(value)
    }
}

/// An explicit virtual session with a bounded FIFO. Neither a real transport,
/// security sandbox, device broker, nor persistent authorization.
pub struct VirtualTransport {
    broker: SimulatedBroker,
    lease: Option<SimulationLease>,
    queue: VecDeque<SimulationEvent>,
    capacity: usize,
    dropped: u64,
}

impl VirtualTransport {
    pub fn new(preferences: &[PreferenceEntry], capacity: usize) -> Result<Self, TransportError> {
        if !(1..=MAX_TRANSPORT_CAPACITY).contains(&capacity) {
            return Err(TransportError::InvalidCapacity);
        }
        Ok(Self {
            broker: SimulatedBroker::new(preferences)?,
            lease: None,
            queue: VecDeque::with_capacity(capacity),
            capacity,
            dropped: 0,
        })
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn retained(&self) -> usize {
        self.queue.len()
    }

    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    fn retire(&mut self) {
        self.lease = None;
        self.queue.clear();
    }

    fn retire_if_inactive(&mut self) {
        if self.lease.is_some() && self.broker.active_device().is_none() {
            self.retire();
        }
    }

    fn ensure_live(&mut self) -> Result<(), TransportError> {
        if self.lease.is_none() || self.broker.active_device().is_none() {
            self.retire();
            return Err(BrokerError::InvalidLease.into());
        }
        Ok(())
    }

    pub fn open(&mut self, id: &str, confirmation: SessionConfirmation) -> Result<(), TransportError> {
        let lease = self.broker.open(id, confirmation)?;
        self.queue.clear();
        self.lease = Some(lease);
        Ok(())
    }

    pub fn open_with_timeout(
        &mut self,
        id: &str,
        confirmation: SessionConfirmation,
        ttl_ms: u64,
    ) -> Result<(), TransportError> {
        let lease = self.broker.open_with_timeout(id, confirmation, ttl_ms)?;
        self.queue.clear();
        self.lease = Some(lease);
        Ok(())
    }

    /// Produces no more than 128 fixed, clearly simulated events at a time.
    /// The oldest event is dropped if the 1..=64 element FIFO is full.
    pub fn pump(&mut self, count: usize) -> Result<(), TransportError> {
        if !(1..=MAX_TRANSPORT_PUMP_COUNT).contains(&count) {
            return Err(TransportError::InvalidCount);
        }
        self.ensure_live()?;
        for _ in 0..count {
            let lease = self.lease.as_ref().ok_or(BrokerError::InvalidLease)?;
            let event = self.broker.sample(lease)?;
            if self.queue.len() == self.capacity {
                self.queue.pop_front();
                self.dropped = self.dropped.saturating_add(1);
            }
            self.queue.push_back(event);
        }
        Ok(())
    }

    /// An invalidated lease can never drain queued events.
    pub fn receive(&mut self) -> Result<Option<SimulationEvent>, TransportError> {
        self.ensure_live()?;
        Ok(self.queue.pop_front())
    }

    pub fn close(&mut self) -> Result<(), TransportError> {
        self.ensure_live()?;
        let result = self
            .broker
            .close(self.lease.as_ref().ok_or(BrokerError::InvalidLease)?);
        self.retire();
        result.map_err(Into::into)
    }

    pub fn cancel(&mut self) -> Result<(), TransportError> {
        self.ensure_live()?;
        let result = self
            .broker
            .cancel(self.lease.as_ref().ok_or(BrokerError::InvalidLease)?);
        self.retire();
        result.map_err(Into::into)
    }

    pub fn revoke(&mut self, capability: Capability) -> bool {
        let result = self.broker.revoke(capability);
        self.retire_if_inactive();
        result
    }

    pub fn disconnect(&mut self, id: &str) -> Result<bool, TransportError> {
        let result = self.broker.disconnect(id);
        self.retire_if_inactive();
        result.map_err(Into::into)
    }

    pub fn reconnect(&mut self, id: &str) -> Result<bool, TransportError> {
        self.broker.reconnect(id).map_err(Into::into)
    }

    pub fn advance_to(&mut self, logical_ms: u64) -> Result<bool, TransportError> {
        let result = self.broker.advance_to(logical_ms);
        self.retire_if_inactive();
        result.map_err(Into::into)
    }

    pub fn replace_preferences(
        &mut self,
        entries: &[PreferenceEntry],
    ) -> Result<(), TransportError> {
        let result = self.broker.replace_preferences(entries);
        self.retire_if_inactive();
        result.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::Preference;

    fn preference(key: &str, value: Preference) -> PreferenceEntry {
        PreferenceEntry {
            capability: key.to_string(),
            preference: value,
        }
    }

    #[test]
    fn buffer_and_pump_limits_reject_without_collecting() {
        assert!(matches!(
            VirtualTransport::new(&[], 0),
            Err(TransportError::InvalidCapacity)
        ));
        assert!(matches!(
            VirtualTransport::new(&[], 65),
            Err(TransportError::InvalidCapacity)
        ));
        let mut t = VirtualTransport::new(&[], 2).unwrap();
        t.open("sim:sdr", SessionConfirmation::Confirmed).unwrap();
        assert_eq!(t.pump(0), Err(TransportError::InvalidCount));
        assert_eq!(t.pump(129), Err(TransportError::InvalidCount));
        assert_eq!((t.retained(), t.dropped()), (0, 0));
    }

    #[test]
    fn overflow_retains_newest_mock_events_and_reports_drops() {
        let mut t = VirtualTransport::new(&[], 2).unwrap();
        t.open("sim:sdr", SessionConfirmation::Confirmed).unwrap();
        t.pump(5).unwrap();
        assert_eq!((t.capacity(), t.retained(), t.dropped()), (2, 2, 3));
        let first = t.receive().unwrap().unwrap();
        let second = t.receive().unwrap().unwrap();
        assert_eq!((first.sequence, second.sequence), (4, 5));
        assert!(first.simulated);
        assert_eq!(second.event, "fixture_only_no_hardware_io");
        assert!(t.receive().unwrap().is_none());
        t.close().unwrap();
    }

    #[test]
    fn missing_confirmation_and_denial_prevent_collection() {
        let mut t = VirtualTransport::new(
            &[preference("mesh.transmit", Preference::Requested)],
            2,
        ).unwrap();
        assert_eq!(
            t.open("sim:mesh", SessionConfirmation::Absent),
            Err(TransportError::Broker(BrokerError::ConfirmationRequired))
        );
        assert_eq!(
            t.pump(1),
            Err(TransportError::Broker(BrokerError::InvalidLease))
        );
        t.replace_preferences(&[preference("mesh.transmit", Preference::Denied)])
            .unwrap();
        assert_eq!(
            t.open("sim:mesh", SessionConfirmation::Confirmed),
            Err(TransportError::Broker(BrokerError::DeniedPreference))
        );
        assert_eq!(t.retained(), 0);
    }

    #[test]
    fn revoke_purges_queued_events_and_requires_fresh_confirmation() {
        let mut t = VirtualTransport::new(&[], 4).unwrap();
        t.open("sim:ble", SessionConfirmation::Confirmed).unwrap();
        t.pump(3).unwrap();
        assert!(t.revoke(Capability::BleDiscover));
        assert_eq!(t.retained(), 0);
        assert_eq!(
            t.receive(),
            Err(TransportError::Broker(BrokerError::InvalidLease))
        );
        assert_eq!(
            t.open("sim:ble", SessionConfirmation::Absent),
            Err(TransportError::Broker(BrokerError::ConfirmationRequired))
        );
        t.open("sim:ble", SessionConfirmation::Confirmed).unwrap();
        t.pump(1).unwrap();
        assert_eq!(t.receive().unwrap().unwrap().sequence, 4);
    }

    #[test]
    fn disconnect_and_reconnect_do_not_restore_buffered_data() {
        let mut t = VirtualTransport::new(&[], 2).unwrap();
        t.open("sim:serial", SessionConfirmation::Confirmed).unwrap();
        t.pump(2).unwrap();
        assert!(t.disconnect("sim:serial").unwrap());
        assert_eq!(t.retained(), 0);
        assert_eq!(
            t.open("sim:serial", SessionConfirmation::Confirmed),
            Err(TransportError::Broker(BrokerError::Unavailable))
        );
        assert!(t.reconnect("sim:serial").unwrap());
        assert_eq!(
            t.open("sim:serial", SessionConfirmation::Absent),
            Err(TransportError::Broker(BrokerError::ConfirmationRequired))
        );
        t.open("sim:serial", SessionConfirmation::Confirmed).unwrap();
        assert_eq!(t.retained(), 0);
    }

    #[test]
    fn cancel_and_logical_deadline_purge_data() {
        let mut t = VirtualTransport::new(&[], 3).unwrap();
        t.open_with_timeout("sim:gpio", SessionConfirmation::Confirmed, 200)
            .unwrap();
        t.pump(2).unwrap();
        assert!(!t.advance_to(199).unwrap());
        assert!(t.advance_to(200).unwrap());
        assert_eq!(t.retained(), 0);
        assert!(t.receive().is_err());
        t.open("sim:gpio", SessionConfirmation::Confirmed).unwrap();
        t.pump(1).unwrap();
        t.cancel().unwrap();
        assert_eq!(t.retained(), 0);
        assert!(t.receive().is_err());
    }

    #[test]
    fn failed_preference_reload_clears_queue_and_blocks_reopen() {
        let mut t = VirtualTransport::new(&[], 2).unwrap();
        t.open("sim:network", SessionConfirmation::Confirmed).unwrap();
        t.pump(2).unwrap();
        let bad = [
            preference("network.fetch", Preference::Denied),
            preference("network.fetch", Preference::Requested),
        ];
        assert_eq!(
            t.replace_preferences(&bad),
            Err(TransportError::Broker(BrokerError::InvalidPreference))
        );
        assert_eq!(t.retained(), 0);
        assert_eq!(
            t.open("sim:network", SessionConfirmation::Confirmed),
            Err(TransportError::Broker(BrokerError::InvalidPreference))
        );
        t.replace_preferences(&[]).unwrap();
        assert_eq!(
            t.open("sim:network", SessionConfirmation::Absent),
            Err(TransportError::Broker(BrokerError::ConfirmationRequired))
        );
    }

    #[test]
    fn backtracking_logical_clock_discards_pending_events() {
        let mut t = VirtualTransport::new(&[], 2).unwrap();
        t.open("sim:gpio", SessionConfirmation::Confirmed).unwrap();
        t.pump(2).unwrap();
        assert!(!t.advance_to(100).unwrap());
        assert_eq!(
            t.advance_to(99),
            Err(TransportError::Broker(BrokerError::ClockReversed))
        );
        assert_eq!(t.retained(), 0);
        assert!(t.receive().is_err());
    }
}
