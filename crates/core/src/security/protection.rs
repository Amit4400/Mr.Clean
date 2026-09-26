//! Mac protection checks: FileVault, Firewall, Gatekeeper, SIP, automatic
//! updates and Remote Login. Read-only: each runs one fixed system command
//! and reads its output. Fixing means opening the matching System Settings
//! page; nothing is changed from here.

use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Pass,
    Fail,
    Unknown,
}

/// System Settings pages the app may open. An allowlist: the UI can only
/// name one of these, never pass a URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pane {
    FullDiskAccess,
    FileVault,
    Firewall,
    PrivacySecurity,
    SoftwareUpdate,
    Sharing,
}

impl Pane {
    pub fn url(self) -> &'static str {
        match self {
            Pane::FullDiskAccess => "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles",
            Pane::FileVault => "x-apple.systempreferences:com.apple.preference.security?FileVault",
            Pane::Firewall => "x-apple.systempreferences:com.apple.preference.security?Firewall",
            Pane::PrivacySecurity => "x-apple.systempreferences:com.apple.preference.security?General",
            Pane::SoftwareUpdate => "x-apple.systempreferences:com.apple.Software-Update-Settings.extension",
            Pane::Sharing => "x-apple.systempreferences:com.apple.Sharing-Settings.extension",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub id: &'static str,
    pub title: &'static str,
    /// What it protects, in plain words.
    pub about: &'static str,
    pub state: State,
    /// Where to turn it on. `None` when it can't be changed from Settings (SIP).
    pub pane: Option<Pane>,
    /// Extra help shown instead of (or next to) the Fix button.
    pub how: Option<&'static str>,
}

fn has(out: &str, s: &str) -> bool {
    out.to_ascii_lowercase().contains(&s.to_ascii_lowercase())
}

fn pass_fail(out: &str, on: &str, off: &str) -> State {
    if has(out, off) {
        State::Fail
    } else if has(out, on) {
        State::Pass
    } else {
        State::Unknown
    }
}

pub fn parse_filevault(out: &str) -> State {
    pass_fail(out, "FileVault is On", "FileVault is Off")
}

pub fn parse_firewall(out: &str) -> State {
    // "Firewall is enabled. (State = 1)" / "Firewall is disabled. (State = 0)"
    pass_fail(out, "enabled", "disabled")
}

pub fn parse_gatekeeper(out: &str) -> State {
    pass_fail(out, "assessments enabled", "assessments disabled")
}

pub fn parse_sip(out: &str) -> State {
    pass_fail(out, "status: enabled", "status: disabled")
}

/// `defaults read … AutomaticallyInstallMacOSUpdates`: "1", "0", or an error
/// when the key was never set (then it follows the macOS default).
pub fn parse_auto_updates(out: &str, ok: bool) -> State {
    match (ok, out.trim()) {
        (true, "1") => State::Pass,
        (true, "0") => State::Fail,
        _ => State::Unknown,
    }
}

/// `launchctl print-disabled system`: Remote Login is off unless sshd is
/// listed as enabled (older macOS: `=> false`).
pub fn parse_remote_login(out: &str) -> State {
    let line = out.lines().find(|l| l.contains("\"com.openssh.sshd\""));
    match line {
        Some(l) if l.contains("enabled") || l.contains("false") => State::Fail,
        _ => State::Pass,
    }
}

fn run(cmd: &str, args: &[&str]) -> (String, bool) {
    match Command::new(cmd).args(args).output() {
        Ok(o) => (
            format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            ),
            o.status.success(),
        ),
        Err(_) => (String::new(), false),
    }
}

/// Run every check. macOS only; other systems get an empty list.
pub fn checks() -> Vec<Check> {
    if !cfg!(target_os = "macos") {
        return vec![];
    }
    let (fv, _) = run("/usr/bin/fdesetup", &["status"]);
    let (fw, _) = run("/usr/libexec/ApplicationFirewall/socketfilterfw", &["--getglobalstate"]);
    let (gk, _) = run("/usr/sbin/spctl", &["--status"]);
    let (sip, _) = run("/usr/bin/csrutil", &["status"]);
    let (au, au_ok) = run(
        "/usr/bin/defaults",
        &[
            "read",
            "/Library/Preferences/com.apple.SoftwareUpdate",
            "AutomaticallyInstallMacOSUpdates",
        ],
    );
    let (ssh, _) = run("/bin/launchctl", &["print-disabled", "system"]);
    vec![
        Check {
            id: "filevault",
            title: "FileVault disk encryption",
            about: "Keeps your files unreadable if your Mac is lost or stolen.",
            state: parse_filevault(&fv),
            pane: Some(Pane::FileVault),
            how: None,
        },
        Check {
            id: "firewall",
            title: "Firewall",
            about: "Blocks unwanted incoming connections.",
            state: parse_firewall(&fw),
            pane: Some(Pane::Firewall),
            how: None,
        },
        Check {
            id: "gatekeeper",
            title: "Gatekeeper",
            about: "Only lets apps from identified developers open.",
            state: parse_gatekeeper(&gk),
            pane: Some(Pane::PrivacySecurity),
            how: Some("In Privacy & Security, set \"Allow applications from\" to App Store and identified developers."),
        },
        Check {
            id: "sip",
            title: "System Integrity Protection",
            about: "Stops anything, even with your password, from changing macOS system files.",
            state: parse_sip(&sip),
            pane: None,
            how: Some("Restart into Recovery (hold the power button), open Terminal from Utilities, run `csrutil enable`, then restart."),
        },
        Check {
            id: "updates",
            title: "Automatic macOS updates",
            about: "Installs security fixes as soon as Apple ships them.",
            state: parse_auto_updates(&au, au_ok),
            pane: Some(Pane::SoftwareUpdate),
            how: Some("Click the ⓘ next to Automatic updates and turn on \"Install macOS updates\" and \"Install Security Responses\"."),
        },
        Check {
            id: "remote_login",
            title: "Remote Login (SSH) off",
            about: "When on, anyone with your password can log in over the network.",
            state: parse_remote_login(&ssh),
            pane: Some(Pane::Sharing),
            how: Some("Turn off Remote Login unless you connect to this Mac over SSH."),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_command_outputs() {
        assert_eq!(parse_filevault("FileVault is On.\n"), State::Pass);
        assert_eq!(parse_filevault("FileVault is Off.\n"), State::Fail);
        assert_eq!(parse_filevault(""), State::Unknown);
        assert_eq!(parse_firewall("Firewall is enabled. (State = 1)"), State::Pass);
        assert_eq!(parse_firewall("Firewall is disabled. (State = 0)"), State::Fail);
        assert_eq!(parse_gatekeeper("assessments enabled\n"), State::Pass);
        assert_eq!(parse_gatekeeper("assessments disabled\n"), State::Fail);
        assert_eq!(parse_sip("System Integrity Protection status: enabled.\n"), State::Pass);
        assert_eq!(
            parse_sip("System Integrity Protection status: disabled.\n"),
            State::Fail
        );
        assert_eq!(parse_auto_updates("1\n", true), State::Pass);
        assert_eq!(parse_auto_updates("0\n", true), State::Fail);
        assert_eq!(parse_auto_updates("does not exist", false), State::Unknown);
        let ssh_on = "disabled services = {\n\t\"com.apple.ftpd\" => disabled\n\t\"com.openssh.sshd\" => enabled\n}";
        let ssh_off = "disabled services = {\n\t\"com.openssh.sshd\" => disabled\n}";
        assert_eq!(parse_remote_login(ssh_on), State::Fail);
        assert_eq!(parse_remote_login(ssh_off), State::Pass);
        assert_eq!(parse_remote_login("disabled services = {\n}"), State::Pass);
    }

    #[test]
    fn panes_are_system_settings_links() {
        for p in [
            Pane::FullDiskAccess,
            Pane::FileVault,
            Pane::Firewall,
            Pane::PrivacySecurity,
            Pane::SoftwareUpdate,
            Pane::Sharing,
        ] {
            assert!(p.url().starts_with("x-apple.systempreferences:"));
        }
    }
}
