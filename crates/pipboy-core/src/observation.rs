//! Versioned local observations with explicit provenance and freshness.
//! Process-relative time cannot be compared across different app sessions.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const OBSERVATION_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Valid,
    Stale,
    Unknown,
    Invalid,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Observation<T> {
    pub schema_version: u16,
    pub source_id: String,
    pub provenance: String,
    pub captured_wall_unix_ms: Option<u64>,
    /// Milliseconds since process-local session start; not a shared clock.
    pub captured_session_ms: u64,
    pub unit: Option<String>,
    pub value: Option<T>,
    pub quality: Quality,
    pub expires_after_ms: Option<u64>,
}

impl<T> Observation<T> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_id: impl Into<String>,
        provenance: impl Into<String>,
        captured_wall_unix_ms: Option<u64>,
        captured_session_ms: u64,
        unit: Option<String>,
        value: Option<T>,
        quality: Quality,
        expires_after_ms: Option<u64>,
    ) -> Result<Self, String> {
        let item = Self {
            schema_version: OBSERVATION_SCHEMA_VERSION,
            source_id: source_id.into(),
            provenance: provenance.into(),
            captured_wall_unix_ms,
            captured_session_ms,
            unit,
            value,
            quality,
            expires_after_ms,
        };
        item.validate()?;
        Ok(item)
    }

    /// Create a measured observation without networking or side effects.
    pub fn measured(
        source_id: &str,
        provenance: &str,
        value: T,
        session_start: Instant,
        expires_after: Duration,
    ) -> Result<Self, String> {
        let wall_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|d| u64::try_from(d.as_millis()).ok());
        let since_start = u64::try_from(session_start.elapsed().as_millis()).unwrap_or(u64::MAX);
        let expires_ms = u64::try_from(expires_after.as_millis()).unwrap_or(u64::MAX);
        Self::new(
            source_id,
            provenance,
            wall_time,
            since_start,
            None,
            Some(value),
            Quality::Valid,
            Some(expires_ms),
        )
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != OBSERVATION_SCHEMA_VERSION {
            return Err("unsupported observation schema".into());
        }
        if self.source_id.trim().is_empty() || self.source_id.len() > 128 {
            return Err("source_id must contain 1-128 bytes".into());
        }
        if self.provenance.trim().is_empty() || self.provenance.len() > 256 {
            return Err("provenance must contain 1-256 bytes".into());
        }
        if self
            .unit
            .as_deref()
            .is_some_and(|unit| unit.trim().is_empty() || unit.len() > 48)
        {
            return Err("unit must contain 1-48 bytes when provided".into());
        }
        if self.expires_after_ms == Some(0) {
            return Err("sample expiry cannot be zero".into());
        }
        if matches!(self.quality, Quality::Valid | Quality::Stale) != self.value.is_some() {
            return Err(
                "valid or stale needs a value; unknown or invalid must not carry a value".into(),
            );
        }
        Ok(())
    }

    /// Only a valid measurement may transition into stale.
    pub fn update_freshness(&mut self, session_now_ms: u64) {
        if self.quality == Quality::Valid
            && self
                .expires_after_ms
                .is_some_and(|ttl| session_now_ms.saturating_sub(self.captured_session_ms) >= ttl)
        {
            self.quality = Quality::Stale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_is_not_zero() {
        let zero = Observation::new(
            "host.cpu",
            "fixture",
            Some(1000),
            50,
            Some("%".into()),
            Some(0_u8),
            Quality::Valid,
            Some(500),
        )
        .unwrap();
        let unknown = Observation::new(
            "host.cpu",
            "fixture",
            None,
            50,
            None,
            None::<u8>,
            Quality::Unknown,
            None,
        )
        .unwrap();
        assert_eq!(zero.value, Some(0));
        assert_eq!(unknown.value, None);
        assert_ne!(zero.quality, unknown.quality);
    }

    #[test]
    fn expiry_is_monotonic_and_one_way() {
        let mut item = Observation::new(
            "rx",
            "fixture",
            None,
            100,
            None,
            Some(1_u8),
            Quality::Valid,
            Some(300),
        )
        .unwrap();
        item.update_freshness(399);
        assert_eq!(item.quality, Quality::Valid);
        item.update_freshness(400);
        assert_eq!(item.quality, Quality::Stale);
        item.update_freshness(50);
        assert_eq!(item.quality, Quality::Stale);
    }

    #[test]
    fn absent_value_and_invalid_provenance_are_rejected() {
        assert!(Observation::new(
            "rx",
            "fixture",
            None,
            0,
            None,
            None::<u8>,
            Quality::Valid,
            None
        )
        .is_err());
        assert!(
            Observation::new("rx", "", None, 0, None, Some(1_u8), Quality::Valid, None).is_err()
        );
        assert!(Observation::new(
            "rx",
            "fixture",
            None,
            0,
            None,
            Some(1_u8),
            Quality::Unknown,
            None
        )
        .is_err());
    }

    #[test]
    fn round_trip_and_schema_validation() {
        let item = Observation::new(
            "cpu",
            "local:sysinfo",
            Some(1234),
            42,
            Some("%".into()),
            Some(22_u8),
            Quality::Valid,
            Some(1000),
        )
        .unwrap();
        let text = serde_json::to_string(&item).unwrap();
        let decoded: Observation<u8> = serde_json::from_str(&text).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded, item);
        assert!(text.contains("\"quality\":\"valid\""));
    }

    #[test]
    fn locally_measured_values_are_attributed() {
        let sample = Observation::measured(
            "host.stats",
            "local:sysinfo",
            1_u8,
            Instant::now(),
            Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(sample.schema_version, OBSERVATION_SCHEMA_VERSION);
        assert_eq!(sample.value, Some(1));
        assert!(sample.captured_wall_unix_ms.is_some());
    }
}
