//! Mac protection checks: FileVault, Firewall, Gatekeeper, SIP, automatic
//! updates and Remote Login. Read-only: each runs one fixed system command
//! and reads its output. Fixing means opening the matching System Settings
//! page; nothing is changed from here.

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
    // Windows
    WindowsVirus,
    WindowsFirewall,
    WindowsAppBrowser,
    WindowsUpdate,
    DeviceEncryption,
    RemoteDesktop,
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
            Pane::WindowsVirus => "windowsdefender://threat",
            Pane::WindowsFirewall => "windowsdefender://network",
            Pane::WindowsAppBrowser => "windowsdefender://appbrowser",
            Pane::WindowsUpdate => "ms-settings:windowsupdate",
            Pane::DeviceEncryption => "ms-settings:deviceencryption",
            Pane::RemoteDesktop => "ms-settings:remotedesktop",
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
    match crate::fsutil::command(cmd).args(args).output() {
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

/// Run every check for this operating system.
pub fn checks() -> Vec<Check> {
    if cfg!(windows) {
        return windows_checks();
    }
    if cfg!(target_os = "linux") {
        return linux_checks();
    }
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

// ----------------------------------------------------------------- Windows

/// One PowerShell run that prints `key=value` lines (read-only queries).
const WINDOWS_SCRIPT: &str = r#"$ErrorActionPreference='SilentlyContinue'
$s=Get-MpComputerStatus; "defender=$($s.RealTimeProtectionEnabled)|$($s.AMRunningMode)"
"firewall=" + ((Get-NetFirewallProfile | ForEach-Object { $_.Enabled }) -join ',')
"bitlocker=" + (New-Object -ComObject Shell.Application).NameSpace($env:SystemDrive).Self.ExtendedProperty('System.Volume.BitLockerProtection')
"wuauserv=" + (Get-Service wuauserv).StartType
"smartscreen=" + (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer').SmartScreenEnabled
"uac=" + (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System').EnableLUA
"rdp=" + (Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Terminal Server').fDenyTSConnections
"#;

fn kv(out: &str) -> std::collections::HashMap<String, String> {
    out.lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

/// Build the Windows checks from the script's `key=value` output.
pub fn parse_windows(out: &str) -> Vec<Check> {
    let m = kv(out);
    let get = |k: &str| m.get(k).map(String::as_str).unwrap_or("");
    let defender = {
        let (on, mode) = get("defender").split_once('|').unwrap_or((get("defender"), ""));
        if on.eq_ignore_ascii_case("true") || has(mode, "passive") {
            State::Pass // passive = another antivirus is in charge
        } else if on.eq_ignore_ascii_case("false") {
            State::Fail
        } else {
            State::Unknown
        }
    };
    let firewall = match get("firewall") {
        "" => State::Unknown,
        v if v
            .split(',')
            .all(|p| p.trim().eq_ignore_ascii_case("true") || p.trim() == "1") =>
        {
            State::Pass
        }
        _ => State::Fail,
    };
    // System.Volume.BitLockerProtection: 1 = on, 2 = off, 3 = encrypting; others unknown.
    let bitlocker = match get("bitlocker") {
        "1" | "3" | "5" => State::Pass,
        "2" => State::Fail,
        _ => State::Unknown,
    };
    let updates = match get("wuauserv") {
        "" => State::Unknown,
        v if v.eq_ignore_ascii_case("disabled") => State::Fail,
        _ => State::Pass,
    };
    let smartscreen = if get("smartscreen").eq_ignore_ascii_case("off") {
        State::Fail
    } else {
        State::Pass
    };
    let uac = match get("uac") {
        "0" => State::Fail,
        "1" => State::Pass,
        _ => State::Unknown,
    };
    let rdp = if get("rdp") == "0" { State::Fail } else { State::Pass };
    vec![
        Check {
            id: "defender",
            title: "Virus protection",
            about: "Microsoft Defender (or another antivirus) scans files as they open.",
            state: defender,
            pane: Some(Pane::WindowsVirus),
            how: Some("In Windows Security, turn on Real-time protection."),
        },
        Check {
            id: "firewall",
            title: "Firewall",
            about: "Blocks unwanted incoming connections on every network.",
            state: firewall,
            pane: Some(Pane::WindowsFirewall),
            how: Some("Turn the firewall on for Domain, Private and Public networks."),
        },
        Check {
            id: "bitlocker",
            title: "Drive encryption (BitLocker)",
            about: "Keeps your files unreadable if your PC is lost or stolen.",
            state: bitlocker,
            pane: Some(Pane::DeviceEncryption),
            how: Some("Turn on Device encryption, or BitLocker in Control Panel on Pro editions."),
        },
        Check {
            id: "updates",
            title: "Windows Update",
            about: "Installs security fixes as soon as Microsoft ships them.",
            state: updates,
            pane: Some(Pane::WindowsUpdate),
            how: None,
        },
        Check {
            id: "smartscreen",
            title: "SmartScreen",
            about: "Warns before you run unknown or harmful downloads.",
            state: smartscreen,
            pane: Some(Pane::WindowsAppBrowser),
            how: Some("Under Reputation-based protection, turn on \"Check apps and files\"."),
        },
        Check {
            id: "uac",
            title: "User Account Control",
            about: "Asks before apps make changes that need admin rights.",
            state: uac,
            pane: None,
            how: Some("Search Start for \"UAC\", open Change User Account Control settings, and move the slider off the bottom."),
        },
        Check {
            id: "remote_desktop",
            title: "Remote Desktop off",
            about: "When on, anyone with your password can sign in over the network.",
            state: rdp,
            pane: Some(Pane::RemoteDesktop),
            how: Some("Turn off Remote Desktop unless you connect to this PC remotely."),
        },
    ]
}

fn windows_checks() -> Vec<Check> {
    let (out, _) = run(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", WINDOWS_SCRIPT],
    );
    parse_windows(&out)
}

// ------------------------------------------------------------------- Linux

pub fn parse_lsblk_encrypted(out: &str) -> State {
    if out
        .lines()
        .any(|l| l.split_whitespace().any(|w| w == "crypt" || w == "crypto_LUKS"))
    {
        State::Pass
    } else if out.trim().is_empty() {
        State::Unknown
    } else {
        State::Fail
    }
}

/// `/etc/ufw/ufw.conf` → ENABLED=yes|no.
pub fn parse_ufw_conf(text: &str) -> State {
    match text.lines().find_map(|l| l.trim().strip_prefix("ENABLED=")) {
        Some(v) if v.trim_matches('"').eq_ignore_ascii_case("yes") => State::Pass,
        Some(_) => State::Fail,
        None => State::Unknown,
    }
}

/// `/etc/apt/apt.conf.d/20auto-upgrades`.
pub fn parse_auto_upgrades(text: &str) -> State {
    let line = text.lines().find(|l| l.contains("Unattended-Upgrade"));
    match line {
        Some(l) if l.contains("\"1\"") => State::Pass,
        Some(_) => State::Fail,
        None => State::Fail,
    }
}

fn linux_checks() -> Vec<Check> {
    let (lsblk, _) = run("lsblk", &["-rno", "TYPE,FSTYPE"]);
    let firewall = match std::fs::read_to_string("/etc/ufw/ufw.conf") {
        Ok(t) => parse_ufw_conf(&t),
        Err(_) => {
            let (fw, _) = run("systemctl", &["is-active", "firewalld"]);
            if fw.trim() == "active" {
                State::Pass
            } else {
                State::Unknown
            }
        }
    };
    let updates = match std::fs::read_to_string("/etc/apt/apt.conf.d/20auto-upgrades") {
        Ok(t) => parse_auto_upgrades(&t),
        Err(_) if std::path::Path::new("/etc/apt").exists() => State::Fail,
        Err(_) => State::Unknown,
    };
    let (ssh, _) = run("systemctl", &["is-active", "ssh", "sshd"]);
    let ssh = if ssh.lines().any(|l| l.trim() == "active") {
        State::Fail
    } else {
        State::Pass
    };
    let (lock, lock_ok) = run("gsettings", &["get", "org.gnome.desktop.screensaver", "lock-enabled"]);
    let lock = match (lock_ok, lock.trim()) {
        (true, "true") => State::Pass,
        (true, "false") => State::Fail,
        _ => State::Unknown,
    };
    let (sb, _) = run("mokutil", &["--sb-state"]);
    vec![
        Check {
            id: "disk_encryption",
            title: "Disk encryption",
            about: "Keeps your files unreadable if your computer is lost or stolen.",
            state: parse_lsblk_encrypted(&lsblk),
            pane: None,
            how: Some("Most Linux installers can only encrypt the whole disk during installation (\"Encrypt the new installation\"). Keep private files in an encrypted folder until you reinstall."),
        },
        Check {
            id: "firewall",
            title: "Firewall",
            about: "Blocks unwanted incoming connections.",
            state: firewall,
            pane: None,
            how: Some("Run: sudo ufw enable"),
        },
        Check {
            id: "updates",
            title: "Automatic security updates",
            about: "Installs security fixes without waiting for you.",
            state: updates,
            pane: None,
            how: Some("Run: sudo apt install unattended-upgrades && sudo dpkg-reconfigure -plow unattended-upgrades"),
        },
        Check {
            id: "ssh",
            title: "SSH server off",
            about: "When on, anyone with your password can log in over the network.",
            state: ssh,
            pane: None,
            how: Some("If you don't need it, run: sudo systemctl disable --now ssh"),
        },
        Check {
            id: "screen_lock",
            title: "Screen lock",
            about: "Locks your screen when you step away.",
            state: lock,
            pane: None,
            how: Some("Open Settings → Privacy → Screen Lock and turn on Automatic Screen Lock."),
        },
        Check {
            id: "secure_boot",
            title: "Secure Boot",
            about: "Stops tampered boot software from starting before Linux.",
            state: pass_fail(&sb, "SecureBoot enabled", "SecureBoot disabled"),
            pane: None,
            how: Some("Turn on Secure Boot in your computer's firmware (UEFI) settings."),
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
        for p in [
            Pane::WindowsVirus,
            Pane::WindowsFirewall,
            Pane::WindowsAppBrowser,
            Pane::WindowsUpdate,
            Pane::DeviceEncryption,
            Pane::RemoteDesktop,
        ] {
            assert!(p.url().starts_with("ms-settings:") || p.url().starts_with("windowsdefender://"));
        }
    }

    #[test]
    fn parses_windows_output() {
        let out = "defender=True|Normal\r\nfirewall=True,True,False\r\nbitlocker=2\r\nwuauserv=Manual\r\nsmartscreen=\r\nuac=1\r\nrdp=1\r\n";
        let c = parse_windows(out);
        let st = |id: &str| c.iter().find(|c| c.id == id).unwrap().state;
        assert_eq!(st("defender"), State::Pass);
        assert_eq!(st("firewall"), State::Fail, "one profile is off");
        assert_eq!(st("bitlocker"), State::Fail);
        assert_eq!(st("updates"), State::Pass);
        assert_eq!(st("smartscreen"), State::Pass);
        assert_eq!(st("uac"), State::Pass);
        assert_eq!(st("remote_desktop"), State::Pass);
        let c = parse_windows("defender=False|Passive Mode\nwuauserv=Disabled\nuac=0\nrdp=0\nsmartscreen=Off\n");
        let st = |id: &str| c.iter().find(|c| c.id == id).unwrap().state;
        assert_eq!(st("defender"), State::Pass, "another antivirus is active");
        assert_eq!(st("updates"), State::Fail);
        assert_eq!(st("uac"), State::Fail);
        assert_eq!(st("remote_desktop"), State::Fail);
        assert_eq!(st("smartscreen"), State::Fail);
        assert_eq!(st("bitlocker"), State::Unknown);
        assert_eq!(st("firewall"), State::Unknown);
    }

    #[test]
    fn parses_linux_output() {
        assert_eq!(
            parse_lsblk_encrypted("disk \npart crypto_LUKS\ncrypt ext4\n"),
            State::Pass
        );
        assert_eq!(parse_lsblk_encrypted("disk \npart ext4\npart vfat\n"), State::Fail);
        assert_eq!(parse_ufw_conf("# comment\nENABLED=yes\nLOGLEVEL=low\n"), State::Pass);
        assert_eq!(parse_ufw_conf("ENABLED=no\n"), State::Fail);
        assert_eq!(
            parse_auto_upgrades(
                "APT::Periodic::Update-Package-Lists \"1\";\nAPT::Periodic::Unattended-Upgrade \"1\";\n"
            ),
            State::Pass
        );
        assert_eq!(
            parse_auto_upgrades("APT::Periodic::Unattended-Upgrade \"0\";\n"),
            State::Fail
        );
    }
}
