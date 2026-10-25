//! Field-level interpretation of local host snapshots. No collection or I/O.
//!
//! The outer observation records capture/freshness; each field has an explicit
//! unit, quality and bounded reason code. Unknown/invalid never carries a value.

use crate::metrics::Snapshot;
use crate::observation::{Observation, Quality};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Unit {
    #[serde(rename = "percent")]
    Percent,
    #[serde(rename = "MiB")]
    Mebibytes,
    #[serde(rename = "s")]
    Seconds,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "text")]
    Text,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    WarmingUpOrUnavailable,
    Unavailable,
    NonFinite,
    OutOfRange,
    InconsistentMemory,
}

impl Reason {
    fn label(self) -> &'static str {
        match self {
            Self::WarmingUpOrUnavailable => "warming up or unavailable",
            Self::Unavailable => "unavailable",
            Self::NonFinite => "non-finite value",
            Self::OutOfRange => "out of range",
            Self::InconsistentMemory => "used memory exceeds total",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Reading<T> {
    pub value: Option<T>,
    pub unit: Unit,
    pub quality: Quality,
    pub reason: Option<Reason>,
}

impl<T> Reading<T> {
    fn valid(value: T, unit: Unit) -> Self {
        Self {
            value: Some(value),
            unit,
            quality: Quality::Valid,
            reason: None,
        }
    }

    fn absent(unit: Unit, quality: Quality, reason: Reason) -> Self {
        Self {
            value: None,
            unit,
            quality,
            reason: Some(reason),
        }
    }

    fn mark_stale(&mut self) {
        if self.quality == Quality::Valid {
            self.quality = Quality::Stale;
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct HostReadings {
    /// Version of this opt-in payload; independent of the observation envelope.
    pub payload_schema_version: u16,
    /// Interval between poll starts, not a measured CPU averaging window.
    pub poll_interval_ms: Option<u64>,
    pub os: Reading<String>,
    pub arch: Reading<String>,
    pub cpu_percent: Reading<f32>,
    pub logical_cores: Reading<usize>,
    pub memory_used_mib: Reading<u64>,
    pub memory_total_mib: Reading<u64>,
    pub uptime_s: Reading<u64>,
}

impl HostReadings {
    pub fn from_snapshot(snapshot: &Snapshot, poll_interval_ms: Option<u64>) -> Self {
        let cpu_percent = match snapshot.cpu_percent {
            None => Reading::absent(
                Unit::Percent,
                Quality::Unknown,
                Reason::WarmingUpOrUnavailable,
            ),
            Some(value) if !value.is_finite() => {
                Reading::absent(Unit::Percent, Quality::Invalid, Reason::NonFinite)
            }
            Some(value) if !(0.0..=100.0).contains(&value) => {
                Reading::absent(Unit::Percent, Quality::Invalid, Reason::OutOfRange)
            }
            Some(_) if snapshot.logical_cores == 0 => {
                Reading::absent(Unit::Percent, Quality::Unknown, Reason::Unavailable)
            }
            Some(value) => Reading::valid(value, Unit::Percent),
        };
        let (memory_used_mib, memory_total_mib) = if snapshot.memory_total_mib == 0 {
            (
                Reading::absent(Unit::Mebibytes, Quality::Unknown, Reason::Unavailable),
                Reading::absent(Unit::Mebibytes, Quality::Unknown, Reason::Unavailable),
            )
        } else {
            let used = if snapshot.memory_used_mib > snapshot.memory_total_mib {
                Reading::absent(
                    Unit::Mebibytes,
                    Quality::Invalid,
                    Reason::InconsistentMemory,
                )
            } else {
                Reading::valid(snapshot.memory_used_mib, Unit::Mebibytes)
            };
            (
                used,
                Reading::valid(snapshot.memory_total_mib, Unit::Mebibytes),
            )
        };
        Self {
            payload_schema_version: 1,
            poll_interval_ms,
            os: text_reading(&snapshot.os),
            arch: text_reading(&snapshot.arch),
            cpu_percent,
            logical_cores: if snapshot.logical_cores == 0 {
                Reading::absent(Unit::Count, Quality::Unknown, Reason::Unavailable)
            } else {
                Reading::valid(snapshot.logical_cores, Unit::Count)
            },
            memory_used_mib,
            memory_total_mib,
            uptime_s: Reading::valid(snapshot.uptime_s, Unit::Seconds),
        }
    }

    fn mark_stale(&mut self) {
        self.os.mark_stale();
        self.arch.mark_stale();
        self.cpu_percent.mark_stale();
        self.logical_cores.mark_stale();
        self.memory_used_mib.mark_stale();
        self.memory_total_mib.mark_stale();
        self.uptime_s.mark_stale();
    }
}

fn text_reading(value: &str) -> Reading<String> {
    if value.trim().is_empty() || value.eq_ignore_ascii_case("unknown") {
        Reading::absent(Unit::Text, Quality::Unknown, Reason::Unavailable)
    } else {
        Reading::valid(value.to_owned(), Unit::Text)
    }
}

/// Preserve the original source, session and expiry; never read the host again.
pub fn describe(
    observation: &Observation<Snapshot>,
    poll_interval_ms: Option<u64>,
) -> Observation<HostReadings> {
    let mut report = Observation {
        schema_version: observation.schema_version,
        source_id: observation.source_id.clone(),
        provenance: observation.provenance.clone(),
        captured_wall_unix_ms: observation.captured_wall_unix_ms,
        captured_session_ms: observation.captured_session_ms,
        unit: observation.unit.clone(),
        value: observation
            .value
            .as_ref()
            .map(|snapshot| HostReadings::from_snapshot(snapshot, poll_interval_ms)),
        quality: observation.quality,
        expires_after_ms: observation.expires_after_ms,
    };
    refresh(&mut report, observation.captured_session_ms);
    report
}

/// Freshness changes are one-way, including per-field qualities.
pub fn refresh(report: &mut Observation<HostReadings>, now_ms: u64) {
    report.update_freshness(now_ms);
    if report.quality == Quality::Stale {
        if let Some(readings) = &mut report.value {
            readings.mark_stale();
        }
    }
}

/// Shared human-readable field values. Quality labels remain readable without color.
pub fn display<T: std::fmt::Display>(reading: &Reading<T>) -> String {
    match &reading.value {
        Some(value) => format!("{value} [{:?}]", reading.quality),
        None => format!(
            "{:?} ({})",
            reading.quality,
            reading.reason.unwrap_or(Reason::Unavailable).label()
        ),
    }
}

pub fn display_fields(readings: &HostReadings) -> String {
    let interval = readings
        .poll_interval_ms
        .map(|value| format!("{value} ms between poll starts"))
        .unwrap_or_else(|| "first poll; interval unknown".into());
    format!(
        "HOST: {} / {}\nCPU: {} percent\nCORES: {} logical\nRAM: {} / {} MiB\nUPTIME: {} s\nPOLL: {}",
        display(&readings.os), display(&readings.arch), display(&readings.cpu_percent),
        display(&readings.logical_cores), display(&readings.memory_used_mib),
        display(&readings.memory_total_mib), display(&readings.uptime_s), interval,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        Snapshot {
            observed_unix_s: 1234,
            os: "fixture".into(),
            arch: "test".into(),
            cpu_percent: Some(0.0),
            logical_cores: 2,
            memory_used_mib: 0,
            memory_total_mib: 64,
            uptime_s: 0,
        }
    }

    #[test]
    fn zero_is_measured_but_warmup_and_missing_memory_are_unknown() {
        let mut sample = snapshot();
        let valid = HostReadings::from_snapshot(&sample, None);
        assert_eq!(valid.cpu_percent.value, Some(0.0));
        assert_eq!(valid.memory_used_mib.value, Some(0));
        assert_eq!(valid.uptime_s.quality, Quality::Valid);
        assert_eq!(valid.cpu_percent.unit, Unit::Percent);
        assert_eq!(valid.memory_used_mib.unit, Unit::Mebibytes);
        assert_eq!(valid.uptime_s.unit, Unit::Seconds);
        sample.cpu_percent = None;
        sample.memory_total_mib = 0;
        let absent = HostReadings::from_snapshot(&sample, Some(2000));
        assert_eq!(absent.cpu_percent.quality, Quality::Unknown);
        assert_eq!(absent.cpu_percent.value, None);
        assert_eq!(absent.memory_used_mib.value, None);
        assert_eq!(absent.memory_total_mib.quality, Quality::Unknown);
        assert_eq!(absent.poll_interval_ms, Some(2000));
    }

    #[test]
    fn rejects_nonfinite_out_of_range_and_inconsistent_values_before_json() {
        for (value, reason) in [
            (f32::NAN, Reason::NonFinite),
            (f32::INFINITY, Reason::NonFinite),
            (f32::NEG_INFINITY, Reason::NonFinite),
            (-0.1, Reason::OutOfRange),
            (100.1, Reason::OutOfRange),
        ] {
            let mut sample = snapshot();
            sample.cpu_percent = Some(value);
            sample.memory_used_mib = 65;
            let report = HostReadings::from_snapshot(&sample, None);
            assert_eq!(report.cpu_percent.quality, Quality::Invalid);
            assert_eq!(report.cpu_percent.reason, Some(reason));
            assert_eq!(report.cpu_percent.value, None);
            assert_eq!(report.memory_used_mib.quality, Quality::Invalid);
            assert_eq!(report.memory_used_mib.value, None);
            assert_eq!(report.memory_total_mib.value, Some(64));
            let json = serde_json::to_value(&report).unwrap();
            assert!(json["cpu_percent"]["value"].is_null());
            assert_eq!(json["cpu_percent"]["quality"], "invalid");
        }
    }

    #[test]
    fn missing_os_and_zero_cores_do_not_become_valid_measurements() {
        let mut sample = snapshot();
        sample.os = "unknown".into();
        sample.arch = " ".into();
        sample.logical_cores = 0;
        let report = HostReadings::from_snapshot(&sample, None);
        assert_eq!(report.os.quality, Quality::Unknown);
        assert_eq!(report.arch.value, None);
        assert_eq!(report.cpu_percent.value, None);
        assert_eq!(report.logical_cores.quality, Quality::Unknown);
    }

    #[test]
    fn expiry_preserves_unknowns_provenance_and_never_revives_stale_fields() {
        let mut sample = snapshot();
        sample.cpu_percent = None;
        let observation = Observation::new(
            "host.stats",
            "local:sysinfo",
            None,
            100,
            None,
            Some(sample),
            Quality::Valid,
            Some(500),
        )
        .unwrap();
        let mut report = describe(&observation, Some(2000));
        refresh(&mut report, 599);
        assert_eq!(
            report.value.as_ref().unwrap().memory_total_mib.quality,
            Quality::Valid
        );
        refresh(&mut report, 600);
        refresh(&mut report, 0);
        report.validate().unwrap();
        let readings = report.value.as_ref().unwrap();
        assert_eq!(report.quality, Quality::Stale);
        assert_eq!(readings.memory_total_mib.quality, Quality::Stale);
        assert_eq!(readings.cpu_percent.quality, Quality::Unknown);
        assert_eq!(report.captured_session_ms, 100);
        assert_eq!(report.captured_wall_unix_ms, None);
        assert_eq!(report.provenance, "local:sysinfo");
        let json = serde_json::to_string(&report).unwrap();
        let decoded: Observation<HostReadings> = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, report);
    }
}
