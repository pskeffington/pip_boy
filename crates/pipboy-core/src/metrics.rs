//! Real host measurements via sysinfo. Unknown hardware adapters are not faked.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{
    CpuRefreshKind, MemoryRefreshKind, RefreshKind, System, MINIMUM_CPU_UPDATE_INTERVAL,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Snapshot {
    pub observed_unix_s: u64,
    pub os: String,
    pub arch: String,
    /// None while warming up or when the OS cannot provide a usable CPU delta.
    pub cpu_percent: Option<f32>,
    pub logical_cores: usize,
    pub memory_used_mib: u64,
    pub memory_total_mib: u64,
    pub uptime_s: u64,
}

pub struct Collector {
    system: System,
    started: Instant,
    cpu_cadence: CpuCadence,
}

/// Gate CPU refreshes without sleeping or presenting cached usage as a new delta.
struct CpuCadence {
    minimum: Duration,
    last_refresh: Duration,
}

impl CpuCadence {
    fn refresh_due(&mut self, now: Duration) -> bool {
        if now.saturating_sub(self.last_refresh) < self.minimum {
            return false;
        }
        self.last_refresh = now;
        true
    }
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector {
    pub fn new() -> Self {
        // Collect only what STAT uses; do not load process inventories or swap.
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        Self {
            system,
            started: Instant::now(),
            cpu_cadence: CpuCadence {
                minimum: MINIMUM_CPU_UPDATE_INTERVAL,
                last_refresh: Duration::ZERO,
            },
        }
    }

    /// OS-dependent minimum interval for reliable CPU usage deltas.
    pub fn minimum_cpu_interval() -> std::time::Duration {
        MINIMUM_CPU_UPDATE_INTERVAL
    }

    /// CPU usage requires two samples separated by a minimum interval.
    pub fn prime(&mut self) {
        let elapsed = self
            .started
            .elapsed()
            .saturating_sub(self.cpu_cadence.last_refresh);
        std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL.saturating_sub(elapsed));
    }

    pub fn sample(&mut self) -> Snapshot {
        let cpu_percent = if self.cpu_cadence.refresh_due(self.started.elapsed()) {
            self.system.refresh_cpu_usage();
            self.cpu_cadence.last_refresh = self.started.elapsed();
            let usage = self.system.global_cpu_usage();
            (usage.is_finite() && (0.0..=100.0).contains(&usage) && !self.system.cpus().is_empty())
                .then_some(usage)
        } else {
            None
        };
        self.system
            .refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
        Snapshot {
            observed_unix_s: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            os: System::name().unwrap_or_else(|| "unknown".to_string()),
            arch: std::env::consts::ARCH.to_string(),
            cpu_percent,
            logical_cores: self.system.cpus().len(),
            memory_used_mib: self.system.used_memory() / (1024 * 1024),
            memory_total_mib: self.system.total_memory() / (1024 * 1024),
            uptime_s: System::uptime(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_snapshot_has_a_valid_timestamp() {
        let mut collector = Collector::new();
        let snapshot = collector.sample();
        assert!(snapshot.observed_unix_s > 1_600_000_000);
        assert!(!snapshot.arch.is_empty());
        assert!(collector.system.processes().is_empty());
    }

    #[test]
    fn cpu_refresh_requires_a_full_window_and_does_not_reset_on_early_polls() {
        let mut cadence = CpuCadence {
            minimum: Duration::from_millis(200),
            last_refresh: Duration::ZERO,
        };
        for tick in [0, 1, 199] {
            assert!(!cadence.refresh_due(Duration::from_millis(tick)));
        }
        assert!(cadence.refresh_due(Duration::from_millis(200)));
        assert!(!cadence.refresh_due(Duration::from_millis(200)));
        assert!(!cadence.refresh_due(Duration::from_millis(399)));
        assert!(cadence.refresh_due(Duration::from_millis(400)));
        assert!(!cadence.refresh_due(Duration::from_millis(100))); // rollback
    }
}
