//! Real host measurements via sysinfo. Unknown hardware adapters are not faked.

use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::{System, MINIMUM_CPU_UPDATE_INTERVAL};

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub observed_unix_s: u64,
    pub os: String,
    pub arch: String,
    pub cpu_percent: f32,
    pub logical_cores: usize,
    pub memory_used_mib: u64,
    pub memory_total_mib: u64,
    pub uptime_s: u64,
}

pub struct Collector {
    system: System,
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }

    /// CPU usage requires two samples separated by a minimum interval.
    pub fn prime(&mut self) {
        self.system.refresh_cpu_usage();
        std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
    }

    pub fn sample(&mut self) -> Snapshot {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        Snapshot {
            observed_unix_s: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            os: System::name().unwrap_or_else(|| "unknown".to_string()),
            arch: std::env::consts::ARCH.to_string(),
            cpu_percent: self.system.global_cpu_usage(),
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
    }
}
