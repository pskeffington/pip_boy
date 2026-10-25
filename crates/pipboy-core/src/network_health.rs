//! Deterministic statistics for explicitly requested network-health measurements.
//! RTT-like TCP connect timings are not ICMP round trips, and failed connects
//! are not IP packet loss. Statistics cannot initiate I/O.
use serde::Serialize;
use std::io;

pub const MIN_SAMPLES: u8 = 3;
pub const MAX_SAMPLES: u8 = 8;

#[derive(Clone, Debug, Serialize)]
pub struct HealthReport {
    pub method: &'static str,
    pub target: String,
    pub attempts: usize,
    pub connected: usize,
    pub failed: usize,
    pub failure_percent: f64,
    pub min_ms: Option<f64>,
    pub mean_ms: Option<f64>,
    pub median_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub max_ms: Option<f64>,
    /// Arithmetic mean of absolute differences of adjacent successful probes.
    /// This is connect-time variation, not RFC 3550 RTP interarrival jitter.
    pub variation_ms: Option<f64>,
    pub samples_ms: Vec<Option<f64>>,
    pub interpretation: &'static str,
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// A strict count is shared by the CLI and local dashboard.
pub fn count(value: &str) -> io::Result<u8> {
    if value.is_empty() || !value.bytes().all(|x| x.is_ascii_digit()) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Sample count must be numeric"));
    }
    let n = value
        .parse::<u8>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid sample count"))?;
    if !(MIN_SAMPLES..=MAX_SAMPLES).contains(&n) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Sample count must be between 3 and 8",
        ));
    }
    Ok(n)
}

/// Summarizes completed connect attempts. Never generates samples itself.
pub fn report(target: &str, samples: &[Option<f64>]) -> io::Result<HealthReport> {
    if target.len() > 90 || target.is_empty()
        || !(MIN_SAMPLES as usize..=MAX_SAMPLES as usize).contains(&samples.len())
        || samples.iter().flatten().any(|value| !value.is_finite() || *value < 0.0 || *value > 10_000.0)
    {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid health sample data"));
    }
    let mut successful: Vec<f64> = samples.iter().flatten().copied().collect();
    successful.sort_by(f64::total_cmp);
    let connected = successful.len();
    let failed = samples.len() - connected;
    let variation: Vec<f64> = samples.windows(2).filter_map(|window| {
        match (window[0], window[1]) {
            (Some(first), Some(second)) => Some((second - first).abs()),
            _ => None,
        }
    }).collect();
    let mean = (!successful.is_empty()).then(|| successful.iter().sum::<f64>() / connected as f64);
    let median = (!successful.is_empty()).then(|| {
        if connected % 2 == 1 {
            successful[connected / 2]
        } else {
            (successful[connected / 2 - 1] + successful[connected / 2]) / 2.0
        }
    });
    let p95 = (!successful.is_empty()).then(|| {
        let rank = ((connected as f64 * 0.95).ceil() as usize).max(1) - 1;
        successful[rank]
    });
    Ok(HealthReport {
        method: "tcp_connect",
        target: target.to_owned(),
        attempts: samples.len(),
        connected,
        failed,
        failure_percent: round2(failed as f64 / samples.len() as f64 * 100.0),
        min_ms: successful.first().map(|v| round2(*v)),
        mean_ms: mean.map(round2),
        median_ms: median.map(round2),
        p95_ms: p95.map(round2),
        max_ms: successful.last().map(|v| round2(*v)),
        variation_ms: (!variation.is_empty())
            .then(|| round2(variation.iter().sum::<f64>() / variation.len() as f64)),
        samples_ms: samples.iter().map(|v| v.map(round2)).collect(),
        interpretation: "TCP handshake time and adjacent-sample variation; failed connects are NOT packet loss. No ICMP or bandwidth test.",
    })
}

impl HealthReport {
    pub fn console(&self) -> String {
        fn number(value: Option<f64>) -> String {
            value.map(|v| format!("{v:.2} ms")).unwrap_or_else(|| "N/A".to_owned())
        }
        format!(
            "NETWORK HEALTH / TCP CONNECT\nTarget: {}\nAttempts: {} | Connected: {} | Failed: {}\n\
             Connect failure rate: {:.2}% (NOT packet loss)\n\
             Min: {} | Mean: {} | Median: {}\n\
             P95: {} | Max: {}\nVariation (mean adjacent |delta|): {}\n\
             Method: TCP handshake, fixed target; no application data or port scanning.\n\
             No ICMP RTT, UDP jitter, or throughput claims.",
            self.target, self.attempts, self.connected, self.failed,
            self.failure_percent, number(self.min_ms), number(self.mean_ms),
            number(self.median_ms), number(self.p95_ms), number(self.max_ms),
            number(self.variation_ms)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_p95_and_adjacent_variation_ignore_failed_gaps() {
        let data = [Some(10.0), Some(20.0), None, Some(30.0), Some(40.0)];
        let r = report("127.0.0.1:443", &data).unwrap();
        assert_eq!(r.attempts, 5);
        assert_eq!(r.connected, 4);
        assert_eq!(r.failed, 1);
        assert_eq!(r.failure_percent, 20.0);
        assert_eq!(r.min_ms, Some(10.0));
        assert_eq!(r.median_ms, Some(25.0));
        assert_eq!(r.p95_ms, Some(40.0));
        assert_eq!(r.variation_ms, Some(10.0));
        assert!(r.console().contains("NOT packet loss"));
    }

    #[test]
    fn all_failed_and_single_success_do_not_invent_jitter() {
        let r = report("x", &[None, None, None]).unwrap();
        assert_eq!(r.failure_percent, 100.0);
        assert_eq!(r.mean_ms, None);
        assert_eq!(r.variation_ms, None);
        let r = report("x", &[None, Some(2.5), None]).unwrap();
        assert_eq!(r.variation_ms, None);
        assert_eq!(r.p95_ms, Some(2.5));
    }

    #[test]
    fn reject_unbounded_and_invalid_samples() {
        for s in ["", "2", "9", "255", "-4", "4x", "4\n"] {
            assert!(count(s).is_err(), "{s}");
        }
        assert_eq!(count("5").unwrap(), 5);
        assert!(report("x", &[Some(-1.0), Some(0.0), Some(1.0)]).is_err());
        assert!(report("x", &[Some(f64::NAN), None, None]).is_err());
    }
}
