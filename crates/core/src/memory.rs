//! RAM view: processes grouped by app, developer leftovers, and a safe quit.
//!
//! We never "purge" memory — macOS already uses spare RAM as cache and gives
//! it back on demand. Real wins come from closing things you don't need, so
//! that's all this does, and only for your own, non-system processes.

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;
use sysinfo::{Pid, Process, ProcessRefreshKind, ProcessesToUpdate, Signal, System, UpdateKind};

/// Processes that must keep running for the OS or desktop to work.
const PROTECTED_NAMES: &[&str] = &[
    "kernel_task",
    "launchd",
    "WindowServer",
    "loginwindow",
    "Finder",
    "Dock",
    "SystemUIServer",
    "ControlCenter",
    "coreaudiod",
    "mds",
    "mds_stores",
    "mdworker",
    "cfprefsd",
    "distnoted",
    "securityd",
    "trustd",
    "opendirectoryd",
    "systemd",
    "init",
    "Xorg",
    "Xwayland",
    "gnome-shell",
    "plasmashell",
    "kwin_x11",
    "kwin_wayland",
    "dbus-daemon",
    "pipewire",
    "pulseaudio",
    "explorer.exe",
    "csrss.exe",
    "winlogon.exe",
    "dwm.exe",
    "lsass.exe",
    "services.exe",
    "svchost.exe",
    "System",
    "Registry",
    "smss.exe",
    "wininit.exe",
    "mr-clean",
    "Mr.Clean",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DevKind {
    GradleDaemon,
    KotlinDaemon,
    AdbServer,
    AndroidEmulator,
    IosSimulator,
    DevServer,
    OrphanNode,
    Docker,
    LanguageServer,
}

impl DevKind {
    pub fn advice(self) -> &'static str {
        match self {
            DevKind::GradleDaemon => {
                "Idle Gradle daemons keep 0.5–2 GB each. Safe to quit; the next build starts a new one."
            }
            DevKind::KotlinDaemon => "Kotlin compile daemon. Safe to quit when you're not building.",
            DevKind::AdbServer => "Android debug bridge. Safe to quit; it restarts when you run adb or Android Studio.",
            DevKind::AndroidEmulator => "A running Android emulator. Quit it if you're not testing.",
            DevKind::IosSimulator => "iOS Simulator. Quit it if you're not testing.",
            DevKind::DevServer => "A dev server (Metro, Vite, Next, webpack…). Quit it if you forgot it running.",
            DevKind::OrphanNode => {
                "A Node process whose terminal is gone. Often a forgotten script — check what it is before quitting."
            }
            DevKind::Docker => "Docker's virtual machine. Quit Docker Desktop when you don't need containers.",
            DevKind::LanguageServer => "Editor helper. It restarts when you reopen the project.",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcInfo {
    pub pid: u32,
    pub name: String,
    pub app: String,
    pub memory_bytes: u64,
    pub cpu_percent: f32,
    pub command: String,
    pub protected: bool,
    pub dev_kind: Option<DevKind>,
    pub advice: Option<&'static str>,
    /// The `.app` bundle this process belongs to (for its icon).
    pub bundle: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppGroup {
    pub app: String,
    pub memory_bytes: u64,
    pub cpu_percent: f32,
    pub process_count: usize,
    pub pids: Vec<u32>,
    pub protected: bool,
    pub bundle: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemorySnapshot {
    pub apps: Vec<AppGroup>,
    pub dev_leftovers: Vec<ProcInfo>,
    pub top_processes: Vec<ProcInfo>,
}

pub fn refresh(sys: &mut System) {
    sys.refresh_memory();
    sys.refresh_cpu_usage();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_memory()
            .with_cpu()
            .with_user(UpdateKind::OnlyIfNotSet)
            .with_exe(UpdateKind::OnlyIfNotSet)
            .with_cmd(UpdateKind::OnlyIfNotSet),
    );
}

/// "Google Chrome Helper (Renderer)" → "Google Chrome": the outermost .app.
pub fn app_name(name: &str, exe: Option<&Path>) -> String {
    if let Some(exe) = exe {
        for c in exe.components() {
            let s = c.as_os_str().to_string_lossy();
            if let Some(app) = s.strip_suffix(".app") {
                return app.to_string();
            }
        }
    }
    name.to_string()
}

pub fn classify(name: &str, cmd: &str, parent_is_init: bool) -> Option<DevKind> {
    let lname = name.to_lowercase();
    if cmd.contains("GradleDaemon") || cmd.contains("org.gradle.launcher.daemon") {
        return Some(DevKind::GradleDaemon);
    }
    if cmd.contains("KotlinCompileDaemon") {
        return Some(DevKind::KotlinDaemon);
    }
    if lname == "adb" || lname == "adb.exe" {
        return Some(DevKind::AdbServer);
    }
    if lname.starts_with("qemu-system") || lname == "emulator" || lname.starts_with("emulator64") {
        return Some(DevKind::AndroidEmulator);
    }
    if name == "Simulator" || name == "launchd_sim" {
        return Some(DevKind::IosSimulator);
    }
    if lname.starts_with("com.docker") || name == "Docker Desktop" || lname == "vpnkit" {
        return Some(DevKind::Docker);
    }
    let is_node = lname == "node" || lname == "node.exe" || lname == "bun" || lname == "deno";
    if is_node {
        const SERVERS: &[&str] = &[
            "metro",
            "react-native start",
            "expo start",
            "vite",
            "next dev",
            "next-server",
            "webpack serve",
            "webpack-dev-server",
            "nodemon",
            "ng serve",
            "nuxt dev",
            "storybook",
        ];
        if SERVERS.iter().any(|s| cmd.contains(s)) {
            return Some(DevKind::DevServer);
        }
        if cmd.contains("tsserver") || cmd.contains("language-server") || cmd.contains("eslintServer") {
            return Some(DevKind::LanguageServer);
        }
        if parent_is_init {
            return Some(DevKind::OrphanNode);
        }
    }
    None
}

fn is_protected(p: &Process, me: Pid, my_uid: Option<&sysinfo::Uid>) -> bool {
    let name = p.name().to_string_lossy();
    if p.pid() == me || p.pid().as_u32() <= 1 {
        return true;
    }
    if PROTECTED_NAMES.iter().any(|n| name == *n) {
        return true;
    }
    // Other users' and root's processes: not ours to stop.
    match (p.user_id(), my_uid) {
        (Some(u), Some(mine)) => u != mine,
        _ => true,
    }
}

fn to_info(p: &Process, me: Pid, my_uid: Option<&sysinfo::Uid>, sys: &System) -> ProcInfo {
    let name = p.name().to_string_lossy().to_string();
    let cmd: Vec<String> = p.cmd().iter().map(|s| s.to_string_lossy().to_string()).collect();
    let cmd = cmd.join(" ");
    let parent_is_init =
        p.parent().map(|pp| pp.as_u32() <= 1).unwrap_or(false) || p.parent().and_then(|pp| sys.process(pp)).is_none();
    let dev_kind = classify(&name, &cmd, parent_is_init);
    ProcInfo {
        pid: p.pid().as_u32(),
        app: app_name(&name, p.exe()),
        memory_bytes: p.memory(),
        cpu_percent: p.cpu_usage(),
        command: cmd.chars().take(400).collect(),
        protected: is_protected(p, me, my_uid),
        advice: dev_kind.map(DevKind::advice),
        bundle: p
            .exe()
            .and_then(crate::icons::bundle_of)
            .map(|b| b.display().to_string()),
        dev_kind,
        name,
    }
}

pub fn snapshot(sys: &System) -> MemorySnapshot {
    let me = sysinfo::get_current_pid().unwrap_or(Pid::from_u32(0));
    let my_uid = sys.process(me).and_then(|p| p.user_id()).cloned();
    // On Linux, sysinfo also lists threads as tasks; count each process once.
    let procs: Vec<ProcInfo> = sys
        .processes()
        .values()
        .filter(|p| p.thread_kind().is_none())
        .map(|p| to_info(p, me, my_uid.as_ref(), sys))
        .collect();

    let mut groups: HashMap<String, AppGroup> = HashMap::new();
    for p in &procs {
        let g = groups.entry(p.app.clone()).or_insert_with(|| AppGroup {
            app: p.app.clone(),
            memory_bytes: 0,
            cpu_percent: 0.0,
            process_count: 0,
            pids: vec![],
            protected: false,
            bundle: None,
        });
        if g.bundle.is_none() {
            g.bundle = p.bundle.clone();
        }
        g.memory_bytes += p.memory_bytes;
        g.cpu_percent += p.cpu_percent;
        g.process_count += 1;
        g.pids.push(p.pid);
        g.protected |= p.protected;
    }
    let mut apps: Vec<AppGroup> = groups.into_values().collect();
    apps.sort_by_key(|a| std::cmp::Reverse(a.memory_bytes));
    apps.truncate(40);

    let mut dev: Vec<ProcInfo> = procs.iter().filter(|p| p.dev_kind.is_some()).cloned().collect();
    dev.sort_by_key(|a| std::cmp::Reverse(a.memory_bytes));

    let mut top = procs;
    top.sort_by_key(|a| std::cmp::Reverse(a.memory_bytes));
    top.truncate(60);
    MemorySnapshot {
        apps,
        dev_leftovers: dev,
        top_processes: top,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct QuitResult {
    pub pid: u32,
    pub name: String,
    pub ok: bool,
    pub message: String,
}

/// Ask a process to quit (SIGTERM / close request). `force` sends SIGKILL and
/// is only offered by the UI after a normal quit didn't work. Protection is
/// re-checked here, so the UI can't bypass it. `expected_name` guards against
/// the PID having been reused since the list was shown.
pub fn quit(sys: &System, pid: u32, expected_name: &str, force: bool) -> QuitResult {
    let me = sysinfo::get_current_pid().unwrap_or(Pid::from_u32(0));
    let my_uid = sys.process(me).and_then(|p| p.user_id()).cloned();
    let res = |ok: bool, message: &str| QuitResult {
        pid,
        name: expected_name.to_string(),
        ok,
        message: message.to_string(),
    };
    let Some(p) = sys.process(Pid::from_u32(pid)) else {
        return res(true, "Already closed.");
    };
    if p.name().to_string_lossy() != expected_name {
        return res(
            false,
            "The process changed since the list was loaded. Refresh and try again.",
        );
    }
    if is_protected(p, me, my_uid.as_ref()) {
        return res(
            false,
            "This is a system or other-user process, so Mr.Clean won't stop it.",
        );
    }
    let signal = if force { Signal::Kill } else { Signal::Term };
    let sent = p.kill_with(signal).unwrap_or_else(|| p.kill());
    if sent {
        res(true, if force { "Force quit sent." } else { "Asked to quit." })
    } else {
        res(false, "Couldn't send the quit signal.")
    }
}

/// Quit a whole app. On macOS, GUI apps are asked through AppleScript so they
/// can prompt to save documents; everything else gets SIGTERM per process.
pub fn quit_app(sys: &System, app: &str, force: bool) -> Vec<QuitResult> {
    let snap = snapshot(sys);
    let Some(group) = snap.apps.iter().find(|g| g.app == app) else {
        return vec![QuitResult {
            pid: 0,
            name: app.into(),
            ok: false,
            message: "App not found. Refresh and try again.".into(),
        }];
    };
    if group.protected {
        return vec![QuitResult {
            pid: 0,
            name: app.into(),
            ok: false,
            message: "This app includes system or other-user processes, so Mr.Clean won't stop it.".into(),
        }];
    }
    let is_bundle = sys
        .process(Pid::from_u32(group.pids[0]))
        .and_then(|p| p.exe())
        .is_some_and(|e| e.to_string_lossy().contains(".app/"));
    if cfg!(target_os = "macos") && is_bundle && !force {
        let script = format!("quit app \"{}\"", app.replace('\\', "\\\\").replace('"', "\\\""));
        let ok = std::process::Command::new("osascript")
            .args(["-e", &script])
            .status()
            .is_ok_and(|s| s.success());
        return vec![QuitResult {
            pid: group.pids[0],
            name: app.into(),
            ok,
            message: if ok {
                "Asked the app to quit (it may ask to save).".into()
            } else {
                "The app didn't accept the quit request.".into()
            },
        }];
    }
    group
        .pids
        .iter()
        .filter_map(|pid| {
            sys.process(Pid::from_u32(*pid))
                .map(|p| (*pid, p.name().to_string_lossy().to_string()))
        })
        .map(|(pid, name)| quit(sys, pid, &name, force))
        .collect()
}

// --------------------------------------------------------------- breakdown

/// Where RAM goes, in Activity Monitor's terms. `free` includes file cache
/// that macOS hands back instantly when apps need it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Breakdown {
    pub total: u64,
    pub apps: u64,
    pub wired: u64,
    pub compressed: u64,
    pub free: u64,
}

fn finish(total: u64, apps: u64, wired: u64, compressed: u64) -> Breakdown {
    let used = (apps + wired + compressed).min(total);
    Breakdown {
        total,
        apps,
        wired,
        compressed,
        free: total - used,
    }
}

/// Parse macOS `vm_stat` output.
pub fn parse_vm_stat(text: &str, total: u64) -> Option<Breakdown> {
    let page: u64 = text
        .split("page size of ")
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    let get = |key: &str| -> u64 {
        text.lines()
            .find(|l| l.trim_start().trim_start_matches('"').starts_with(key))
            .and_then(|l| l.rsplit(':').next())
            .and_then(|v| v.trim().trim_end_matches('.').parse::<u64>().ok())
            .unwrap_or(0)
            * page
    };
    let anonymous = get("Anonymous pages");
    let purgeable = get("Pages purgeable");
    Some(finish(
        total,
        anonymous.saturating_sub(purgeable),
        get("Pages wired down"),
        get("Pages occupied by compressor"),
    ))
}

/// Parse Linux `/proc/meminfo` (values in kB).
pub fn parse_meminfo(text: &str) -> Option<Breakdown> {
    let get = |key: &str| -> u64 {
        text.lines()
            .find(|l| l.split(':').next() == Some(key))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0)
            * 1024
    };
    let total = get("MemTotal");
    if total == 0 {
        return None;
    }
    let wired = get("Unevictable") + get("Slab") + get("KernelStack") + get("PageTables");
    let compressed = get("Zswap");
    let used = total.saturating_sub(get("MemAvailable"));
    let apps = used.saturating_sub(wired + compressed);
    Some(finish(total, apps, wired, compressed))
}

pub fn breakdown(sys: &System) -> Breakdown {
    let total = sys.total_memory();
    let parsed = if cfg!(target_os = "macos") {
        std::process::Command::new("vm_stat")
            .output()
            .ok()
            .and_then(|o| parse_vm_stat(&String::from_utf8_lossy(&o.stdout), total))
    } else if cfg!(target_os = "linux") {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|t| parse_meminfo(&t))
    } else {
        None
    };
    parsed.unwrap_or_else(|| finish(total, total.saturating_sub(sys.available_memory()), 0, 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm_stat_breakdown() {
        let out = "Mach Virtual Memory Statistics: (page size of 16384 bytes)
Pages free:                                3000.
Pages active:                            100000.
Pages wired down:                         70000.
Pages purgeable:                           5000.
\"Translation faults\":                 123456.
Pages occupied by compressor:             30000.
Anonymous pages:                         250000.
";
        let total = 16 * 1024 * 1024 * 1024;
        let b = parse_vm_stat(out, total).unwrap();
        assert_eq!(b.apps, 245_000 * 16384);
        assert_eq!(b.wired, 70_000 * 16384);
        assert_eq!(b.compressed, 30_000 * 16384);
        assert_eq!(b.apps + b.wired + b.compressed + b.free, total);
        assert!(parse_vm_stat("garbage", total).is_none());
    }

    #[test]
    fn meminfo_breakdown() {
        let t = "MemTotal:       16000000 kB\nMemAvailable:    6000000 kB\nSlab:  500000 kB\nKernelStack: 20000 kB\nPageTables: 80000 kB\nUnevictable: 0 kB\n";
        let b = parse_meminfo(t).unwrap();
        assert_eq!(b.total, 16_000_000 * 1024);
        assert_eq!(b.wired, 600_000 * 1024);
        assert_eq!(b.free, 6_000_000 * 1024);
        assert_eq!(b.apps, 9_400_000 * 1024);
    }

    #[test]
    fn app_names_from_bundles() {
        let exe = Path::new(
            "/Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Helper (Renderer).app/Contents/MacOS/x",
        );
        assert_eq!(app_name("Google Chrome Helper", Some(exe)), "Google Chrome");
        assert_eq!(app_name("node", Some(Path::new("/usr/local/bin/node"))), "node");
    }

    #[test]
    fn dev_classification() {
        assert_eq!(
            classify(
                "java",
                "java -cp gradle-launcher.jar org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.5",
                false
            ),
            Some(DevKind::GradleDaemon)
        );
        assert_eq!(
            classify("adb", "adb -L tcp:5037 fork-server server", false),
            Some(DevKind::AdbServer)
        );
        assert_eq!(
            classify("node", "node node_modules/.bin/react-native start", false),
            Some(DevKind::DevServer)
        );
        assert_eq!(
            classify("node", "node /Users/x/.hidden/run.js", true),
            Some(DevKind::OrphanNode)
        );
        assert_eq!(classify("node", "node script.js", false), None);
        assert_eq!(classify("Safari", "", false), None);
    }

    #[test]
    fn snapshot_protects_self_and_init() {
        let mut sys = System::new();
        refresh(&mut sys);
        let s = snapshot(&sys);
        let me = std::process::id();
        let mine = s
            .top_processes
            .iter()
            .chain(s.dev_leftovers.iter())
            .find(|p| p.pid == me);
        if let Some(p) = mine {
            assert!(p.protected);
        }
        let r = quit(&sys, me, "definitely-not-this-name", false);
        assert!(!r.ok);
        if let Some(init) = sys.process(Pid::from_u32(1)) {
            let r = quit(&sys, 1, &init.name().to_string_lossy(), true);
            assert!(!r.ok, "pid 1 is protected");
        }
    }
}
