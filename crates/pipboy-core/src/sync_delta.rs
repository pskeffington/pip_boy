//! Offline-only bounded delta planning for prospective RB sync v2.
//!
//! This is NOT a transport, authorization, or trusted acknowledgement service.
//! Callers must separately verify organization, operator, device, dataset and
//! receipt authenticity before using any cursor in a live protocol.
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_BATCH: usize = 64;
pub const MAX_ORIGINS: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventHeader {
    pub origin: String,
    pub sequence: u64,
    pub event_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanError {
    InvalidLimit,
    InvalidIdentifier,
    InvalidSequence,
    TooManyOrigins,
    DuplicateSequence,
    SequenceGap,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CursorSet {
    /// Contiguous, verified, durable sequence per origin; zero means none.
    pub positions: BTreeMap<String, u64>,
}

fn valid_id(id: &str) -> bool {
    (2..=64).contains(&id.len())
        && id.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        && id.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && !id.ends_with('-')
}

/// Select a deterministic, bounded batch of contiguous event headers.
/// Rejects malformed input rather than silently skipping a gap.
/// The returned headers are planning hints, not proof of delivery.
pub fn plan_batch(
    events: &[EventHeader],
    receiver: &CursorSet,
    limit: usize,
) -> Result<Vec<EventHeader>, PlanError> {
    if limit == 0 || limit > MAX_BATCH {
        return Err(PlanError::InvalidLimit);
    }
    if receiver.positions.len() > MAX_ORIGINS {
        return Err(PlanError::TooManyOrigins);
    }
    for (origin, _) in &receiver.positions {
        if !valid_id(origin) {
            return Err(PlanError::InvalidIdentifier);
        }
    }
    let mut ordered = events.to_vec();
    for event in &ordered {
        if !valid_id(&event.origin) || !valid_id(&event.event_id) {
            return Err(PlanError::InvalidIdentifier);
        }
        if event.sequence == 0 {
            return Err(PlanError::InvalidSequence);
        }
    }
    ordered.sort_by(|a, b| (&a.origin, a.sequence, &a.event_id).cmp(&(&b.origin, b.sequence, &b.event_id)));
    let mut seen = BTreeMap::<String, u64>::new();
    let mut seen_sequences = BTreeSet::new();
    let mut selected = Vec::new();
    for event in ordered {
        if !seen.contains_key(&event.origin) && seen.len() >= MAX_ORIGINS {
            return Err(PlanError::TooManyOrigins);
        }
        let current = seen.entry(event.origin.clone()).or_insert_with(|| {
            receiver.positions.get(&event.origin).copied().unwrap_or(0)
        });
        if event.sequence > receiver.positions.get(&event.origin).copied().unwrap_or(0)
            && !seen_sequences.insert((event.origin.clone(), event.sequence)) {
            return Err(PlanError::DuplicateSequence);
        }
        if event.sequence <= *current {
            // Historical events at or below the durable cursor are not sent.
            continue;
        }
        if event.sequence != current.saturating_add(1) {
            return Err(PlanError::SequenceGap);
        }
        *current = event.sequence;
        if selected.len() < limit {
            selected.push(event);
        }
    }
    Ok(selected)
}

/// Advance only a contiguous cursor, *after* the caller has verified a
/// durable, authenticated receipt and committed its evidence locally.
pub fn advance_verified(cursor: &mut CursorSet, origin: &str, sequence: u64) -> Result<(), PlanError> {
    if !valid_id(origin) {
        return Err(PlanError::InvalidIdentifier);
    }
    if !cursor.positions.contains_key(origin) && cursor.positions.len() >= MAX_ORIGINS {
        return Err(PlanError::TooManyOrigins);
    }
    let previous = cursor.positions.get(origin).copied().unwrap_or(0);
    if sequence != previous.saturating_add(1) || previous == u64::MAX {
        return Err(PlanError::SequenceGap);
    }
    cursor.positions.insert(origin.to_owned(), sequence);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(origin: &str, sequence: u64) -> EventHeader {
        EventHeader { origin: origin.into(), sequence, event_id: format!("event-{sequence}") }
    }
    #[test]
    fn selects_contiguous_delta_deterministically() {
        let input = vec![event("unit-b", 1), event("unit-a", 2), event("unit-a", 1)];
        let result = plan_batch(&input, &CursorSet::default(), 2).unwrap();
        assert_eq!(result, vec![event("unit-a", 1), event("unit-a", 2)]);
    }
    #[test]
    fn rejects_gaps_and_invalid_limits() {
        assert_eq!(plan_batch(&[event("unit-a", 2)], &CursorSet::default(), 1), Err(PlanError::SequenceGap));
        assert_eq!(plan_batch(&[], &CursorSet::default(), 0), Err(PlanError::InvalidLimit));
    }
    #[test]
    fn duplicate_future_sequences_are_rejected() {
        assert_eq!(plan_batch(&[event("unit-a", 1), event("unit-a", 1)], &CursorSet::default(), 2), Err(PlanError::DuplicateSequence));
    }
    #[test]
    fn receipt_cursor_cannot_skip_or_replay() {
        let mut cursor = CursorSet::default();
        assert_eq!(advance_verified(&mut cursor, "unit-a", 2), Err(PlanError::SequenceGap));
        advance_verified(&mut cursor, "unit-a", 1).unwrap();
        assert_eq!(advance_verified(&mut cursor, "unit-a", 1), Err(PlanError::SequenceGap));
        assert_eq!(cursor.positions["unit-a"], 1);
    }
    #[test]
    fn existing_cursor_filters_prior_history() {
        let mut c = CursorSet::default();
        c.positions.insert("unit-a".into(), 2);
        assert_eq!(plan_batch(&[event("unit-a", 1), event("unit-a", 3)], &c, 5).unwrap(), vec![event("unit-a", 3)]);
    }
}
