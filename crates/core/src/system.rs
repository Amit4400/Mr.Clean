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

// ------------------------------------------------------------------ device

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Laptop,
    Imac,
    Mini,
    Studio,
    Desktop,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    /// "MacBook Pro", "Mac mini", "Linux PC"…
    pub name: String,
    /// "Apple M2 Pro", "Intel Core i7"…
    pub chip: String,
    pub memory_bytes: u64,
    /// "macOS Sonoma 14.5", "Ubuntu 24.04"…
    pub os_label: String,
    pub kind: DeviceKind,
}

/// macOS marketing name for a major version.
pub fn macos_name(version: &str) -> &'static str {
    match version.split('.').next().and_then(|m| m.parse::<u32>().ok()) {
        Some(11) => "Big Sur",
        Some(12) => "Monterey",
        Some(13) => "Ventura",
        Some(14) => "Sonoma",
        Some(15) => "Sequoia",
        Some(26) => "Tahoe",
        _ => "",
    }
}

pub fn kind_from_name(name: &str) -> DeviceKind {
    let n = name.to_lowercase();
    if n.contains("macbook") || n.contains("laptop") || n.contains("notebook") {
        DeviceKind::Laptop
    } else if n.contains("imac") {
        DeviceKind::Imac
    } else if n.contains("mac mini") {
        DeviceKind::Mini
    } else if n.contains("mac studio") || n.contains("mac pro") {
        DeviceKind::Studio
    } else {
        DeviceKind::Desktop
    }
}

/// Parse `system_profiler SPHardwareDataType -json` → (name, chip).
pub fn parse_hardware_json(json: &str) -> Option<(String, String)> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let hw = v.get("SPHardwareDataType")?.get(0)?;
    let name = hw.get("machine_name")?.as_str()?.to_string();
    let chip = hw
        .get("chip_type")
        .or_else(|| hw.get("cpu_type"))
        .and_then(|c| c.as_str())
        .unwrap_or_default()
        .to_string();
    Some((name, chip))
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn device(sys: &System) -> DeviceInfo {
    let cpu = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_default();
    if cfg!(target_os = "macos") {
        let (name, chip) = run("system_profiler", &["SPHardwareDataType", "-json"])
            .and_then(|j| parse_hardware_json(&j))
            .unwrap_or_else(|| ("Mac".into(), cpu.clone()));
        let version = run("sw_vers", &["-productVersion"]).unwrap_or_default();
        let os_label = format!("macOS {} {}", macos_name(&version), version)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        return DeviceInfo {
            kind: kind_from_name(&name),
            name,
            chip: if chip.is_empty() { cpu } else { chip },
            memory_bytes: sys.total_memory(),
            os_label,
        };
    }
    let laptop = std::path::Path::new("/sys/class/power_supply/BAT0").exists()
        || std::path::Path::new("/sys/class/power_supply/BAT1").exists();
    let family = if cfg!(windows) { "Windows" } else { "Linux" };
    DeviceInfo {
        name: format!("{family} {}", if laptop { "laptop" } else { "PC" }),
        chip: cpu,
        memory_bytes: sys.total_memory(),
        os_label: System::long_os_version()
            .or_else(System::name)
            .unwrap_or_else(|| family.into()),
        kind: if laptop {
            DeviceKind::Laptop
        } else {
            DeviceKind::Desktop
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_json_and_names() {
        let apple = r#"{"SPHardwareDataType":[{"machine_name":"MacBook Pro","chip_type":"Apple M2 Pro","physical_memory":"16 GB"}]}"#;
        assert_eq!(
            parse_hardware_json(apple),
            Some(("MacBook Pro".into(), "Apple M2 Pro".into()))
        );
        let intel = r#"{"SPHardwareDataType":[{"machine_name":"iMac","cpu_type":"Quad-Core Intel Core i5"}]}"#;
        assert_eq!(
            parse_hardware_json(intel),
            Some(("iMac".into(), "Quad-Core Intel Core i5".into()))
        );
        assert_eq!(parse_hardware_json("{}"), None);
        assert_eq!(macos_name("14.5"), "Sonoma");
        assert_eq!(macos_name("26.0.1"), "Tahoe");
        assert_eq!(kind_from_name("MacBook Air"), DeviceKind::Laptop);
        assert_eq!(kind_from_name("Mac mini"), DeviceKind::Mini);
        assert_eq!(kind_from_name("iMac"), DeviceKind::Imac);
    }

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
