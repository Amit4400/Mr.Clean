//! Security scanner for developer machines. Read-only: it reports findings
//! and can move a file to quarantine (restorable) only when the user asks.
//!
//! Focus: supply-chain malware that targets developers (Shai-Hulud npm worm,
//! Contagious Interview / BeaverTail), persistence (LaunchAgents, cron, shell
//! profiles), git tampering (hooks, identity) and exposed tokens.

mod discover;
mod git;
pub mod iocs;
mod npm;
pub mod patterns;
mod persistence;
mod shell;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::env::Env;
use crate::fsutil::{now_secs, Cancel};
pub use discover::Discovered;
use iocs::Iocs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Area {
    KnownMalware,
    Npm,
    Git,
    Startup,
    ShellProfile,
    Secrets,
    ProjectCode,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub area: Area,
    pub title: String,
    pub detail: String,
    pub path: Option<String>,
    pub line: Option<usize>,
    pub evidence: Option<String>,
    pub advice: Option<String>,
    pub can_quarantine: bool,
}

impl Finding {
    pub fn new(severity: Severity, area: Area, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Finding {
            id: String::new(),
            severity,
            area,
            title: title.into(),
            detail: detail.into(),
            path: None,
            line: None,
            evidence: None,
            advice: None,
            can_quarantine: false,
        }
    }
    pub fn path(mut self, p: &Path) -> Self {
        self.path = Some(p.display().to_string());
        self
    }
    pub fn line(mut self, n: usize) -> Self {
        self.line = Some(n);
        self
    }
    pub fn evidence(mut self, e: impl Into<String>) -> Self {
        self.evidence = Some(e.into());
        self
    }
    pub fn advice(mut self, a: impl Into<String>) -> Self {
        self.advice = Some(a.into());
        self
    }
    pub fn quarantinable(mut self) -> Self {
        self.can_quarantine = true;
        self
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SecurityReport {
    pub findings: Vec<Finding>,
    pub scanned_repos: usize,
    pub scanned_projects: usize,
    pub scanned_packages: usize,
    pub duration_ms: u64,
    pub counts: Counts,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Counts {
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

/// Read a text file if it's reasonably small.
pub(crate) fn read_small(p: &Path, max: u64) -> Option<String> {
    let m = std::fs::metadata(p).ok()?;
    if !m.is_file() || m.len() > max {
        return None;
    }
    std::fs::read(p).ok().map(|b| String::from_utf8_lossy(&b).into_owned())
}

pub fn scan(env: &Env, roots: &[PathBuf], cancel: &Cancel) -> SecurityReport {
    let started = std::time::Instant::now();
    let iocs = Iocs::load(&env.home);
    let mut f: Vec<Finding> = Vec::new();

    // Known malware leftovers in the home folder.
    for hp in &iocs.home_paths {
        let p = env.home.join(&hp.path);
        if std::fs::symlink_metadata(&p).is_ok() {
            f.push(
                Finding::new(Severity::High, Area::KnownMalware, format!("Known malware folder: ~/{}", hp.path),
                    format!("This location is used by {}.", hp.malware))
                    .path(&p)
                    .advice("Quarantine it, rotate your GitHub/npm/cloud tokens, and check GitHub for repos or commits you didn't make.")
                    .quarantinable(),
            );
        }
    }

    f.extend(persistence::scan(env));
    f.extend(shell::scan(env));
    if cancel.is_cancelled() {
        return finish(f, 0, 0, 0, started);
    }
    let found = discover::discover(env, roots, &iocs, cancel);
    f.extend(git::scan(env, &found, &iocs));
    let (npm_findings, packages) = npm::scan(env, &found, &iocs, cancel);
    f.extend(npm_findings);
    f.extend(discover::findings(&found, &iocs));
    finish(f, found.repos.len(), found.projects, packages, started)
}

fn finish(mut f: Vec<Finding>, repos: usize, projects: usize, packages: usize, started: std::time::Instant) -> SecurityReport {
    for x in f.iter_mut() {
        x.id = format!(
            "{:?}|{}|{}|{}",
            x.area,
            x.path.as_deref().unwrap_or(""),
            x.line.unwrap_or(0),
            x.title
        );
    }
    f.sort_by(|a, b| b.severity.cmp(&a.severity).then(a.title.cmp(&b.title)));
    f.dedup_by(|a, b| a.id == b.id);
    let mut counts = Counts::default();
    for x in &f {
        match x.severity {
            Severity::High => counts.high += 1,
            Severity::Medium => counts.medium += 1,
            Severity::Low => counts.low += 1,
            Severity::Info => counts.info += 1,
        }
    }
    SecurityReport {
        findings: f,
        scanned_repos: repos,
        scanned_projects: projects,
        scanned_packages: packages,
        duration_ms: started.elapsed().as_millis() as u64,
        counts,
    }
}

// ------------------------------------------------------------- quarantine

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineEntry {
    pub id: String,
    pub original: String,
    pub stored: String,
    pub reason: String,
    pub at: i64,
    /// Unix permission bits before quarantine, restored on the way back.
    #[serde(default)]
    pub mode: Option<u32>,
}

fn quarantine_dir(env: &Env) -> PathBuf {
    env.home.join(".mrclean/quarantine")
}

fn manifest_path(env: &Env) -> PathBuf {
    quarantine_dir(env).join("manifest.json")
}

pub fn quarantined(env: &Env) -> Vec<QuarantineEntry> {
    std::fs::read_to_string(manifest_path(env))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_manifest(env: &Env, entries: &[QuarantineEntry]) -> Result<(), String> {
    std::fs::create_dir_all(quarantine_dir(env)).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
    std::fs::write(manifest_path(env), json).map_err(|e| e.to_string())
}

/// Move a flagged file or folder into ~/.mrclean/quarantine. Only paths in the
/// home folder, and never anything inside the quarantine itself.
pub fn quarantine(env: &Env, path: &Path, reason: &str) -> Result<QuarantineEntry, String> {
    let resolved = crate::safety::Guard::resolve(path).map_err(|e| e.to_string())?;
    let home = env.home.canonicalize().unwrap_or_else(|_| env.home.clone());
    let qdir = quarantine_dir(env);
    if !resolved.starts_with(&home) || resolved == home {
        return Err("Only items inside your home folder can be quarantined. Remove system items with an admin account.".into());
    }
    if resolved.starts_with(qdir.canonicalize().unwrap_or(qdir.clone())) || resolved.starts_with(&qdir) {
        return Err("Already in quarantine.".into());
    }
    let at = now_secs();
    let id = format!("{at}-{}", quarantined(env).len());
    let slot = qdir.join(&id);
    std::fs::create_dir_all(&slot).map_err(|e| e.to_string())?;
    let name = resolved.file_name().ok_or("bad path")?;
    let stored = slot.join(name);
    #[allow(unused_mut)]
    let mut mode = None;
    std::fs::rename(&resolved, &stored).map_err(|e| format!("Couldn't move it: {e}"))?;
    // Strip execute permission so nothing runs it from quarantine.
    #[cfg(unix)]
    if stored.is_file() {
        use std::os::unix::fs::PermissionsExt;
        mode = std::fs::metadata(&stored).ok().map(|m| m.permissions().mode());
        let _ = std::fs::set_permissions(&stored, std::fs::Permissions::from_mode(0o600));
    }
    let entry = QuarantineEntry {
        id,
        original: resolved.display().to_string(),
        stored: stored.display().to_string(),
        reason: reason.to_string(),
        at,
        mode,
    };
    let mut all = quarantined(env);
    all.push(entry.clone());
    save_manifest(env, &all)?;
    Ok(entry)
}

pub fn restore(env: &Env, id: &str) -> Result<(), String> {
    let mut all = quarantined(env);
    let idx = all.iter().position(|e| e.id == id).ok_or("Not found in quarantine.")?;
    let e = &all[idx];
    let original = PathBuf::from(&e.original);
    if original.exists() {
        return Err("Something already exists at the original location.".into());
    }
    if let Some(parent) = original.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&e.stored, &original).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    if let Some(mode) = e.mode {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&original, std::fs::Permissions::from_mode(mode));
    }
    let _ = std::fs::remove_dir(Path::new(&e.stored).parent().unwrap_or(Path::new("")));
    all.remove(idx);
    save_manifest(env, &all)
}

#[cfg(test)]
mod tests;
