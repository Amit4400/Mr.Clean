//! Machine overview for the dashboard.

use std::process::Command;

use serde::Serialize;
use sysinfo::{Disks, System};

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub model: Option<String>,
    pub cpu_brand: String,
    pub cpu_cores: usize,
    pub uptime_secs: u64,
    pub disk: Option<DiskInfo>,
    pub memory: MemoryInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    /// macOS: percentage of memory the kernel considers free (kern.memorystatus_level).
    pub free_percent: Option<u8>,
    pub pressure: Pressure,
    pub cpu_percent: f32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Pressure {
    Normal,
    Warning,
    Critical,
}

fn sysctl(name: &str) -> Option<String> {
    let out = Command::new("sysctl").args(["-n", name]).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The disk the user's files live on. On macOS that's the Data volume, whose
/// free space is what Finder shows.
pub fn main_disk() -> Option<DiskInfo> {
    let disks = Disks::new_with_refreshed_list();
    let pick = |mount: &str| disks.iter().find(|d| d.mount_point().to_string_lossy() == mount);
    let d = pick("/System/Volumes/Data").or_else(|| pick("/")).or_else(|| {
        if cfg!(windows) {
            disks
                .iter()
                .find(|d| d.mount_point().to_string_lossy().starts_with("C:"))
        } else {
            None
        }
    })?;
    Some(DiskInfo {
        name: d.name().to_string_lossy().to_string(),
        mount: d.mount_point().to_string_lossy().to_string(),
        total_bytes: d.total_space(),
        free_bytes: d.available_space(),
    })
}

/// Classify pressure from what's available (and swap in use).
pub fn pressure_from(available: u64, total: u64, free_percent: Option<u8>) -> Pressure {
    let pct = free_percent
        .map(u64::from)
        .unwrap_or_else(|| (available * 100).checked_div(total).unwrap_or(100));
    match pct {
        0..=10 => Pressure::Critical,
        11..=25 => Pressure::Warning,
        _ => Pressure::Normal,
    }
}

pub fn memory(sys: &System) -> MemoryInfo {
    let free_percent = if cfg!(target_os = "macos") {
        sysctl("kern.memorystatus_level").and_then(|s| s.parse().ok())
    } else {
        None
    };
    let total = sys.total_memory();
    let available = sys.available_memory();
    MemoryInfo {
        total_bytes: total,
        used_bytes: total.saturating_sub(available),
        available_bytes: available,
        swap_total_bytes: sys.total_swap(),
        swap_used_bytes: sys.used_swap(),
        free_percent,
        pressure: pressure_from(available, total, free_percent),
        cpu_percent: sys.global_cpu_usage(),
    }
}

pub fn info(sys: &System) -> SystemInfo {
    let model = if cfg!(target_os = "macos") {
        sysctl("hw.model")
    } else {
        None
    };
    SystemInfo {
        hostname: System::host_name().unwrap_or_default(),
        os_name: System::name().unwrap_or_else(|| std::env::consts::OS.into()),
        os_version: System::long_os_version()
            .or_else(System::os_version)
            .unwrap_or_default(),
        model,
        cpu_brand: sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_default(),
        cpu_cores: sys.cpus().len(),
        uptime_secs: System::uptime(),
        disk: main_disk(),
        memory: memory(sys),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_levels() {
        assert_eq!(pressure_from(50, 100, None), Pressure::Normal);
        assert_eq!(pressure_from(20, 100, None), Pressure::Warning);
        assert_eq!(pressure_from(5, 100, None), Pressure::Critical);
        assert_eq!(pressure_from(90, 100, Some(8)), Pressure::Critical);
    }

    #[test]
    fn info_on_this_machine() {
        let mut sys = System::new_all();
        sys.refresh_all();
        let i = info(&sys);
        assert!(i.memory.total_bytes > 0);
        assert!(i.cpu_cores > 0);
    }
}
