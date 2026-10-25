//! Bounded, offline, read-only diagnostic evidence ingestion.
//!
//! A supplied record is an assertion from its source, not proof of a real fault
//! or proof that the operator is authorized to contact the named asset.
//! This module never opens devices, spawns programs, or connects to a network.

use crate::metrics::Snapshot;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const DIAG_SCHEMA_VERSION: u16 = 1;
pub const MAX_DOCUMENT_BYTES: usize = 64 * 1024;
pub const MAX_LINE_BYTES: usize = 2048;
pub const MAX_RECORDS: usize = 128;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    Hardware,
    Software,
    Network,
    Physical,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Signal {
    pub schema_version: u16,
    pub asset_id: String,
    pub source_id: String,
    pub domain: Domain,
    pub metric: String,
    pub value: Value,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub captured_unix_ms: Option<u64>,
}

fn slug(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

impl Signal {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != DIAG_SCHEMA_VERSION {
            return Err("unsupported DIAG schema version".into());
        }
        if !slug(&self.asset_id, 64) || !slug(&self.source_id, 64) {
            return Err("asset_id/source_id must be 1-64 ASCII identifier bytes".into());
        }
        if !slug(&self.metric, 64) {
            return Err("metric must be a 1-64 byte ASCII identifier".into());
        }
        if self.unit.as_ref().is_some_and(|u| {
            u.is_empty()
                || u.len() > 24
                || !u.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"/%._:-".contains(&b))
        }) {
            return Err("invalid signal unit".into());
        }
        match &self.value {
            Value::Bool(_) | Value::Number(_) => {}
            Value::String(s)
                if !s.is_empty()
                    && s.len() <= 128
                    && s.chars().all(|ch| !ch.is_control()) => {}
            _ => return Err("value must be a finite number, bool or short printable string".into()),
        }
        if self.value.is_number()
            && self.value.as_f64().is_none_or(|number| !number.is_finite())
        {
            return Err("value must be finite".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Watch,
    Alert,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Finding {
    pub evidence_line: usize,
    pub source_id: String,
    pub metric: String,
    pub observed_value: Value,
    pub severity: Severity,
    pub code: String,
    pub interpretation: String,
    pub recommended_check: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct DiagnosticReport {
    pub schema_version: u16,
    pub asset_id: String,
    pub intake: String,
    pub record_count: usize,
    pub assessed_count: usize,
    pub unclassified_count: usize,
    pub unavailable_count: usize,
    pub findings: Vec<Finding>,
    /// Neither a finding nor an absence of findings certifies a root cause.
    pub limitation: String,
}

impl DiagnosticReport {
    pub fn render_text(&self) -> String {
        use std::fmt::Write;
        let mut output = format!(
            "DIAG / OFFLINE EVIDENCE\nASSET: {}\nINTAKE: {}\nRECORDS: {} / ASSESSED: {} / UNCLASSIFIED: {} / UNAVAILABLE: {}\nFINDINGS: {} (heuristics, not root-cause proof)\n",
            self.asset_id,
            self.intake,
            self.record_count,
            self.assessed_count,
            self.unclassified_count,
            self.unavailable_count,
            self.findings.len()
        );
        for f in &self.findings {
            let _ = writeln!(
                output,
                "[{:?}] {} @ line {} / {} / {} = {}",
                f.severity,
                f.code,
                f.evidence_line,
                f.source_id,
                f.metric,
                f.observed_value
            );
            let _ = writeln!(output, "  {}. NEXT: {}", f.interpretation, f.recommended_check);
        }
        if self.findings.is_empty() {
            output.push_str("NO MATCHED ALERTS in the assessed signals; this is NOT a clean bill of health.\n");
        }
        output.push_str(&self.limitation);
        output
    }
}

enum Evaluation {
    Assessed(Option<Finding>),
    Unclassified,
}

fn number(signal: &Signal, unit: &str, min: f64, max: f64) -> Result<f64, String> {
    if signal.unit.as_deref() != Some(unit) {
        return Err(format!("{} needs unit {}", signal.metric, unit));
    }
    signal
        .value
        .as_f64()
        .filter(|x| x.is_finite() && *x >= min && *x <= max)
        .ok_or_else(|| format!("{} needs a numeric value within {min}..{max}", signal.metric))
}

fn boolean(signal: &Signal) -> Result<bool, String> {
    if signal.unit.is_some() {
        return Err(format!("{} must omit unit for a boolean value", signal.metric));
    }
    signal
        .value
        .as_bool()
        .ok_or_else(|| format!("{} needs a boolean value", signal.metric))
}

fn finding(
    signal: &Signal,
    line: usize,
    severity: Severity,
    code: &str,
    interpretation: &str,
    recommended_check: &str,
) -> Evaluation {
    Evaluation::Assessed(Some(Finding {
        evidence_line: line,
        source_id: signal.source_id.clone(),
        metric: signal.metric.clone(),
        observed_value: signal.value.clone(),
        severity,
        code: code.into(),
        interpretation: interpretation.into(),
        recommended_check: recommended_check.into(),
    }))
}

fn classify(signal: &Signal, line: usize) -> Result<Evaluation, String> {
    use Domain::*;
    let no_alert = || Evaluation::Assessed(None);
    let result = match (signal.domain, signal.metric.as_str()) {
        (Hardware, "cpu_usage_percent") => {
            let value = number(signal, "percent", 0.0, 100.0)?;
            if value >= 95.0 {
                finding(signal, line, Severity::Watch, "CPU_HIGH",
                    "One high reading is not evidence of sustained saturation",
                    "Repeat after a normal sampling interval and inspect the workload")
            } else {
                no_alert()
            }
        }
        (Hardware, "memory_used_percent") => {
            let value = number(signal, "percent", 0.0, 100.0)?;
            if value >= 95.0 {
                finding(signal, line, Severity::Watch, "MEMORY_PRESSURE",
                    "The reported memory fraction is high; paging is not established",
                    "Compare available memory, swap and application trends")
            } else {
                no_alert()
            }
        }
        (Hardware, "disk_smart_passed") => {
            if !boolean(signal)? {
                finding(signal, line, Severity::Alert, "SMART_FAIL",
                    "The supplied SMART overall-health flag reports a failure",
                    "Back up critical data and verify the drive-specific SMART attributes")
            } else {
                no_alert()
            }
        }
        (Hardware, "device_error_count") => {
            let value = number(signal, "count", 0.0, 1e12)?;
            if value > 0.0 {
                finding(signal, line, Severity::Watch, "DEVICE_ERRORS",
                    "This may be a cumulative counter, not a new failure",
                    "Compare consecutive counter readings and examine device logs")
            } else {
                no_alert()
            }
        }
        (Software, "process_exit_code") => {
            let value = number(signal, "code", -1e9, 1e9)?;
            if value != 0.0 {
                finding(signal, line, Severity::Watch, "NONZERO_EXIT",
                    "A nonzero exit may be expected for some applications",
                    "Inspect the application's exit-code specification and adjacent logs")
            } else {
                no_alert()
            }
        }
        (Software, "service_running") => {
            if !boolean(signal)? {
                finding(signal, line, Severity::Watch, "SERVICE_STOPPED",
                    "Service reported stopped; it may be intentionally inactive",
                    "Check expected service state and local startup logs")
            } else {
                no_alert()
            }
        }
        (Software, "error_count") => {
            let value = number(signal, "count", 0.0, 1e12)?;
            if value > 0.0 {
                finding(signal, line, Severity::Watch, "APP_ERRORS",
                    "The supplied error count is nonzero",
                    "Inspect timestamps and repeated stack traces for a common cause")
            } else {
                no_alert()
            }
        }
        (Network, "packet_loss_percent") => {
            let value = number(signal, "percent", 0.0, 100.0)?;
            if value >= 5.0 {
                finding(signal, line, if value >= 25.0 { Severity::Alert } else { Severity::Watch },
                    "PACKET_LOSS", "Loss is elevated for many interactive workloads",
                    "Check measurement interval, radio quality and route under authorized scope")
            } else {
                no_alert()
            }
        }
        (Network, "latency_ms") => {
            let value = number(signal, "ms", 0.0, 1e9)?;
            if value > 250.0 {
                finding(signal, line, Severity::Watch, "LATENCY_HIGH",
                    "Single-point latency is only meaningful against an expected baseline",
                    "Compare multiple samples from the same authorized destination")
            } else {
                no_alert()
            }
        }
        (Network, "jitter_ms") => {
            let value = number(signal, "ms", 0.0, 1e9)?;
            if value > 50.0 {
                finding(signal, line, Severity::Watch, "JITTER_HIGH",
                    "Observed variability may affect real-time traffic",
                    "Check sample method and compare across time with the same path")
            } else {
                no_alert()
            }
        }
        (Network, "interface_error_count") => {
            let value = number(signal, "count", 0.0, 1e12)?;
            if value > 0.0 {
                finding(signal, line, Severity::Watch, "LINK_ERRORS",
                    "A nonzero interface counter alone cannot date the errors",
                    "Measure counter deltas and inspect cabling or physical link status")
            } else {
                no_alert()
            }
        }
        (Physical, "sensor_fault") => {
            if boolean(signal)? {
                finding(signal, line, Severity::Alert, "SENSOR_FAULT",
                    "The supplied sensor fault flag is asserted",
                    "Consult the device manual and verify the fault locally")
            } else {
                no_alert()
            }
        }
        _ => Evaluation::Unclassified,
    };
    Ok(result)
}

fn evaluate(
    asset: &str,
    intake: &str,
    signals: Vec<(usize, Signal)>,
    unavailable_count: usize,
) -> Result<DiagnosticReport, String> {
    let mut report = DiagnosticReport {
        schema_version: DIAG_SCHEMA_VERSION,
        asset_id: asset.into(),
        intake: intake.into(),
        record_count: signals.len(),
        assessed_count: 0,
        unclassified_count: 0,
        unavailable_count,
        findings: Vec::new(),
        limitation: "EVIDENCE ONLY: source identity and legal scope are operator assertions; no device was contacted, repaired or certified.".into(),
    };
    for (line, signal) in signals {
        signal.validate().map_err(|error| format!("line {line}: {error}"))?;
        if signal.asset_id != asset {
            return Err(format!("line {line}: asset_id is outside the selected scope"));
        }
        match classify(&signal, line).map_err(|error| format!("line {line}: {error}"))? {
            Evaluation::Assessed(hit) => {
                report.assessed_count += 1;
                if let Some(hit) = hit {
                    report.findings.push(hit);
                }
            }
            Evaluation::Unclassified => report.unclassified_count += 1,
        }
    }
    Ok(report)
}

/// Analyze only the exact named asset after an explicit operator attestation.
/// This is *not* an authorization-verification system and never contacts it.
pub fn analyze_ndjson(
    text: &str,
    asset: &str,
    scope_acknowledged: bool,
) -> Result<DiagnosticReport, String> {
    if !scope_acknowledged {
        return Err("explicit operator scope acknowledgment required".into());
    }
    if !slug(asset, 64) {
        return Err("invalid scoped asset ID".into());
    }
    if text.len() > MAX_DOCUMENT_BYTES {
        return Err("diagnostic input exceeds 64 KiB".into());
    }
    let mut signals = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.len() > MAX_LINE_BYTES {
            return Err(format!("line {} exceeds 2048 bytes", index + 1));
        }
        if signals.len() == MAX_RECORDS {
            return Err("diagnostic input exceeds 128 records".into());
        }
        let record: Signal = serde_json::from_str(line)
            .map_err(|_| format!("line {} is not a valid diagnostic v1 record", index + 1))?;
        signals.push((index + 1, record));
    }
    if signals.is_empty() {
        return Err("diagnostic input contains no records".into());
    }
    evaluate(asset, "operator_attested_offline_import", signals, 0)
}

/// A real, one-shot local sysinfo read, separate from unknown-device intake.
/// Missing or invalid CPU/RAM values are unknown, never silently treated as 0.
pub fn analyze_host(snapshot: &Snapshot) -> Result<DiagnosticReport, String> {
    let mut signals = Vec::new();
    let mut unavailable = 0;
    let at = Some(snapshot.observed_unix_s.saturating_mul(1000));
    if let Some(cpu) = snapshot.cpu_percent.filter(|x| x.is_finite() && (0.0..=100.0).contains(x)) {
        signals.push((1, Signal {
            schema_version: DIAG_SCHEMA_VERSION,
            asset_id: "local-host".into(),
            source_id: "local:sysinfo".into(),
            domain: Domain::Hardware,
            metric: "cpu_usage_percent".into(),
            value: json!(cpu),
            unit: Some("percent".into()),
            captured_unix_ms: at,
        }));
    } else {
        unavailable += 1;
    }
    if snapshot.memory_total_mib > 0 && snapshot.memory_used_mib <= snapshot.memory_total_mib {
        let memory_percent = snapshot.memory_used_mib as f64 * 100.0 / snapshot.memory_total_mib as f64;
        signals.push((2, Signal {
            schema_version: DIAG_SCHEMA_VERSION,
            asset_id: "local-host".into(),
            source_id: "local:sysinfo".into(),
            domain: Domain::Hardware,
            metric: "memory_used_percent".into(),
            value: json!(memory_percent),
            unit: Some("percent".into()),
            captured_unix_ms: at,
        }));
    } else {
        unavailable += 1;
    }
    let mut report = evaluate("local-host", "local_sysinfo_one_shot", signals, unavailable)?;
    report.limitation = "LOCAL HOST ONLY: CPU/RAM are sampled once; other hardware, processes, disks and network health were not assessed. No remote devices were contacted.".into();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(metric: &str, value: Value, unit: Option<&str>, domain: Domain) -> String {
        serde_json::to_string(&Signal {
            schema_version: 1,
            asset_id: "lab-1".into(),
            source_id: "user:fixture".into(),
            domain,
            metric: metric.into(),
            value,
            unit: unit.map(str::to_owned),
            captured_unix_ms: Some(1234567),
        }).unwrap()
    }

    #[test]
    fn read_only_import_reports_signal_not_root_cause() {
        let body = format!("{}\n{}\n", record("disk_smart_passed", json!(false), None, Domain::Hardware),
            record("latency_ms", json!(340.0), Some("ms"), Domain::Network));
        let report = analyze_ndjson(&body, "lab-1", true).unwrap();
        assert_eq!(report.assessed_count, 2);
        assert_eq!(report.findings.len(), 2);
        assert_eq!(report.findings[0].code, "SMART_FAIL");
        assert_eq!(report.findings[1].severity, Severity::Watch);
        assert_eq!(report.findings[1].evidence_line, 2);
        assert!(report.render_text().contains("not root-cause proof"));
        assert_eq!(serde_json::from_str::<DiagnosticReport>(&serde_json::to_string(&report).unwrap()).unwrap(), report);
    }

    #[test]
    fn scope_is_explicit_and_cannot_be_changed_by_payload() {
        let sample = record("cpu_usage_percent", json!(0), Some("percent"), Domain::Hardware);
        assert!(analyze_ndjson(&sample, "lab-1", false).is_err());
        assert!(analyze_ndjson(&sample, "other-host", true).is_err());
        assert!(analyze_ndjson(&sample, "../escape", true).is_err());
        assert!(analyze_ndjson("", "lab-1", true).is_err());
        assert!(analyze_ndjson(&"x".repeat(MAX_DOCUMENT_BYTES + 1), "lab-1", true).is_err());
        assert!(analyze_ndjson(&format!("{sample}\n{}", "x".repeat(MAX_LINE_BYTES + 1)), "lab-1", true).is_err());
    }

    #[test]
    fn invalid_and_unknown_signals_are_distinct() {
        let unknown = record("temperature_c", json!(90), Some("celsius"), Domain::Hardware);
        let report = analyze_ndjson(&unknown, "lab-1", true).unwrap();
        assert_eq!(report.assessed_count, 0);
        assert_eq!(report.unclassified_count, 1);
        assert!(report.findings.is_empty());
        assert!(report.render_text().contains("NOT a clean bill of health"));
        let bad = record("packet_loss_percent", json!(150), Some("percent"), Domain::Network);
        assert!(analyze_ndjson(&bad, "lab-1", true).is_err());
        let mut broken: Value = serde_json::from_str(&unknown).unwrap();
        broken["value"] = json!({"unexpected": "object"});
        assert!(analyze_ndjson(&broken.to_string(), "lab-1", true).is_err());
    }

    #[test]
    fn host_missing_cpu_does_not_invent_a_zero() {
        let snapshot = Snapshot {
            observed_unix_s: 2_000_000_000,
            os: "fixture".into(),
            arch: "test".into(),
            cpu_percent: None,
            logical_cores: 1,
            memory_used_mib: 0,
            memory_total_mib: 64,
            uptime_s: 10,
        };
        let report = analyze_host(&snapshot).unwrap();
        assert_eq!(report.unavailable_count, 1);
        assert_eq!(report.assessed_count, 1);
        assert!(report.findings.is_empty());
        assert_eq!(report.intake, "local_sysinfo_one_shot");
    }
}
