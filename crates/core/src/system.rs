//! Machine overview for the dashboard.

use std::path::Path;
use std::process::Command;

use serde::Serialize;
use sysinfo::{Disks, System};

/// Folders macOS hides without Full Disk Access. Never probe the TCC database —
/// SIP keeps it unlistable even when FDA is granted, which falsely looks like
/// "not allowed" and makes the UI ask again every launch. Skip folders that stay
/// readable without FDA (e.g. Reminders) — those would look "granted" forever.
/// Also skip other apps' containers: reading one can pop up "would like to access
/// data from other apps", and this check re-runs on every window focus.
const FDA_PROBES: &[&str] = &[
    "Library/Safari",
    "Library/Mail",
    "Library/Cookies",
    "Library/Suggestions",
];

/// True when this process can see macOS TCC-protected folders under `home`.
/// Non-macOS always returns true (there is no Full Disk Access concept).
pub fn has_full_disk_access(home: &Path) -> bool {
    if !cfg!(target_os = "macos") {
        return true;
    }
    fda_from_probes(home, |p| p.exists(), |p| std::fs::read_dir(p).is_ok())
}

/// Granted if any existing sentinel is listable. If none exist, treat as
/// granted so we never nag forever on an empty fake home.
fn fda_from_probes(home: &Path, exists: impl Fn(&Path) -> bool, can_list: impl Fn(&Path) -> bool) -> bool {
    let mut saw = false;
    for rel in FDA_PROBES {
        let p = home.join(rel);
        if !exists(&p) {
            continue;
        }
        saw = true;
        if can_list(&p) {
            return true;
        }
    }
    !saw
}

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
    pub os: crate::env::Os,
}

/// Values firmware vendors leave in unused model fields.
fn junk(s: &str) -> bool {
    let l = s.trim().to_ascii_lowercase();
    l.is_empty()
        || [
            "to be filled",
            "system product name",
            "system manufacturer",
            "default string",
            "not applicable",
            "none",
            "o.e.m",
            "type1",
            "unknown",
        ]
        .iter()
        .any(|j| l.contains(j))
}

/// A part number like "20XW0024US": no lowercase, no spaces, has digits.
fn looks_like_code(s: &str) -> bool {
    !s.contains(' ') && s.chars().any(|c| c.is_ascii_digit()) && !s.chars().any(|c| c.is_ascii_lowercase())
}

fn nice_vendor(v: &str) -> String {
    let v = v
        .trim()
        .trim_end_matches(" Inc.")
        .trim_end_matches(" Inc")
        .trim_end_matches(", Inc.");
    match v.to_ascii_uppercase().as_str() {
        "LENOVO" => "Lenovo".into(),
        "HP" | "HEWLETT-PACKARD" => "HP".into(),
        "DELL" => "Dell".into(),
        "ASUSTEK COMPUTER" | "ASUSTEK COMPUTER INC." => "ASUS".into(),
        "MICRO-STAR INTERNATIONAL CO., LTD." => "MSI".into(),
        "MICROSOFT CORPORATION" => "Microsoft".into(),
        _ => v.to_string(),
    }
}

/// A friendly PC name from firmware fields, e.g. ("LENOVO", ["20XW0024US",
/// "ThinkPad X1 Carbon Gen 9"]) → "Lenovo ThinkPad X1 Carbon Gen 9".
pub fn pc_name(vendor: &str, candidates: &[&str], fallback: &str) -> String {
    let model = candidates
        .iter()
        .map(|c| c.trim())
        .find(|c| !junk(c) && !looks_like_code(c));
    match model {
        Some(m) => {
            let v = if junk(vendor) {
                String::new()
            } else {
                nice_vendor(vendor)
            };
            if v.is_empty() || m.to_ascii_lowercase().starts_with(&v.to_ascii_lowercase()) {
                m.to_string()
            } else {
                format!("{v} {m}")
            }
        }
        None => fallback.to_string(),
    }
}

/// SMBIOS chassis types: 8–10, 14, 31, 32 are portables; 35/36 mini PCs.
pub fn kind_from_chassis(types: &str, has_battery: bool) -> DeviceKind {
    let nums: Vec<u32> = types
        .split([',', ' ', '\n'])
        .filter_map(|t| t.trim().parse().ok())
        .collect();
    if nums.iter().any(|n| matches!(n, 8 | 9 | 10 | 14 | 31 | 32)) || (nums.is_empty() && has_battery) {
        DeviceKind::Laptop
    } else if nums.iter().any(|n| matches!(n, 35 | 36)) {
        DeviceKind::Mini
    } else {
        DeviceKind::Desktop
    }
}

/// `PRETTY_NAME` from /etc/os-release.
pub fn parse_os_release(text: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.strip_prefix("PRETTY_NAME="))
        .map(|v| v.trim().trim_matches('"').to_string())
        .filter(|v| !v.is_empty())
}

/// Windows 11 still reports itself as "Windows 10" in some places; builds
/// from 22000 on are Windows 11.
pub fn windows_label(long: &str, build: Option<u32>) -> String {
    match build {
        Some(b) if b >= 22000 && long.contains("Windows 10") => long.replace("Windows 10", "Windows 11"),
        _ => long.to_string(),
    }
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
    let out = crate::fsutil::command(cmd).args(args).output().ok()?;
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
            os: crate::env::Os::Mac,
        };
    }
    if cfg!(windows) {
        let out = run(
            "powershell",
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$c=Get-CimInstance Win32_ComputerSystem; $e=Get-CimInstance Win32_SystemEnclosure; $b=@(Get-CimInstance Win32_Battery).Count; \"$($c.Manufacturer)|$($c.Model)|$($c.SystemFamily)|$($e.ChassisTypes -join ',')|$b\"",
            ],
        )
        .unwrap_or_default();
        let f: Vec<&str> = out.trim().split('|').collect();
        let get = |i: usize| f.get(i).copied().unwrap_or("");
        let kind = kind_from_chassis(get(3), get(4).trim().parse::<u32>().unwrap_or(0) > 0);
        let fallback = if kind == DeviceKind::Laptop {
            "Windows laptop"
        } else {
            "Windows PC"
        };
        let long = System::long_os_version().unwrap_or_else(|| "Windows".into());
        let build = System::kernel_version().and_then(|k| k.split('.').next_back().and_then(|b| b.parse().ok()));
        return DeviceInfo {
            name: pc_name(get(0), &[get(1), get(2)], fallback),
            chip: cpu,
            memory_bytes: sys.total_memory(),
            os_label: windows_label(&long, build),
            kind,
            os: crate::env::Os::Windows,
        };
    }
    let dmi = |f: &str| std::fs::read_to_string(format!("/sys/class/dmi/id/{f}")).unwrap_or_default();
    let has_battery = std::fs::read_dir("/sys/class/power_supply")
        .map(|rd| rd.flatten().any(|e| e.file_name().to_string_lossy().starts_with("BAT")))
        .unwrap_or(false);
    let kind = kind_from_chassis(&dmi("chassis_type"), has_battery);
    let fallback = if kind == DeviceKind::Laptop {
        "Linux laptop"
    } else {
        "Linux PC"
    };
    let (name_f, family, version) = (dmi("product_name"), dmi("product_family"), dmi("product_version"));
    DeviceInfo {
        name: pc_name(&dmi("sys_vendor"), &[&name_f, &family, &version], fallback),
        chip: cpu,
        memory_bytes: sys.total_memory(),
        os_label: std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|t| parse_os_release(&t))
            .or_else(System::long_os_version)
            .unwrap_or_else(|| "Linux".into()),
        kind,
        os: crate::env::Os::Linux,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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

    #[test]
    fn pc_names_and_kinds() {
        assert_eq!(
            pc_name("LENOVO", &["20XW0024US", "ThinkPad X1 Carbon Gen 9"], "PC"),
            "Lenovo ThinkPad X1 Carbon Gen 9"
        );
        assert_eq!(pc_name("Dell Inc.", &["XPS 13 9310", "XPS"], "PC"), "Dell XPS 13 9310");
        assert_eq!(
            pc_name("HP", &["HP EliteBook 840 G8 Notebook PC"], "PC"),
            "HP EliteBook 840 G8 Notebook PC"
        );
        assert_eq!(
            pc_name(
                "System manufacturer",
                &["System Product Name", "To be filled by O.E.M."],
                "Windows PC"
            ),
            "Windows PC"
        );
        assert_eq!(
            pc_name("QEMU", &["Standard PC (Q35 + ICH9, 2009)"], "Linux PC"),
            "QEMU Standard PC (Q35 + ICH9, 2009)"
        );
        assert_eq!(kind_from_chassis("10", false), DeviceKind::Laptop);
        assert_eq!(kind_from_chassis("3", true), DeviceKind::Desktop);
        assert_eq!(kind_from_chassis("", true), DeviceKind::Laptop);
        assert_eq!(kind_from_chassis("35", false), DeviceKind::Mini);
        assert_eq!(
            parse_os_release("NAME=\"Ubuntu\"\nPRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\n").as_deref(),
            Some("Ubuntu 24.04.1 LTS")
        );
        assert_eq!(windows_label("Windows 10 Pro", Some(22631)), "Windows 11 Pro");
        assert_eq!(windows_label("Windows 10 Pro", Some(19045)), "Windows 10 Pro");
    }

    #[test]
    fn fda_granted_when_any_sentinel_lists() {
        let home = Path::new("/fake/home");
        let exists = |p: &Path| p.ends_with("Library/Safari") || p.ends_with("Library/Mail");
        // Only Mail is listable — still counts as granted (don't require every probe).
        assert!(fda_from_probes(home, exists, |p| p.ends_with("Library/Mail")));
    }

    #[test]
    fn fda_denied_when_every_existing_sentinel_fails() {
        let home = Path::new("/fake/home");
        let exists = |p: &Path| p.ends_with("Library/Safari") || p.ends_with("Library/Mail");
        assert!(!fda_from_probes(home, exists, |_| false));
    }

    #[test]
    fn fda_granted_when_no_sentinels_exist() {
        let home = Path::new("/fake/home");
        assert!(fda_from_probes(home, |_| false, |_| false));
    }

    #[test]
    fn fda_ignores_missing_sentinels_and_uses_readable_one() {
        let home = Path::new("/fake/home");
        // Safari missing, Mail present and readable.
        assert!(fda_from_probes(
            home,
            |p| p.ends_with("Library/Mail"),
            |p| p.ends_with("Library/Mail"),
        ));
    }
}
