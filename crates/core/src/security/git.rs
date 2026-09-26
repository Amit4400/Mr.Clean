//! Git tampering: global hooks, hook templates, suspicious repo hooks, and
//! commit identities (malware that commits "as you" or as someone else).

use std::path::{Path, PathBuf};
use std::process::Command;

use super::discover::Discovered;
use super::iocs::Iocs;
use super::{patterns, read_small, Area, Finding, Severity};
use crate::env::Env;

/// Hooks written by well-known tools. Still scanned for bad patterns, but not
/// reported just for existing.
const KNOWN_HOOK_TOOLS: &[&str] = &["husky", "lefthook", "pre-commit", "lint-staged", "git-lfs", "commitlint", "overcommit", "talisman"];

fn git_global(env: &Env, key: &str) -> Option<String> {
    let out = Command::new("git")
        .args(["config", "--global", "--get", key])
        .env("HOME", &env.home)
        .env("XDG_CONFIG_HOME", env.home.join(".config"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .ok()?;
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && !v.is_empty()).then_some(v)
}

fn expand_tilde(env: &Env, v: &str) -> PathBuf {
    match v.strip_prefix("~/") {
        Some(rest) => env.home.join(rest),
        None => PathBuf::from(v),
    }
}

/// Tiny reader for `[section] key = value` in a repo's .git/config.
pub fn ini_get(text: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_section = l.trim_matches(['[', ']']).trim().eq_ignore_ascii_case(section);
            continue;
        }
        if in_section {
            if let Some((k, v)) = l.split_once('=') {
                if k.trim().eq_ignore_ascii_case(key) {
                    return Some(v.trim().trim_matches('"').to_string());
                }
            }
        }
    }
    None
}

fn hook_files(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.extension().is_none_or(|e| e != "sample"))
                .collect()
        })
        .unwrap_or_default()
}

/// Report a hook file if its content looks malicious (or unknown & `report_unknown`).
fn check_hook(p: &Path, context: &str, report_unknown: bool, iocs: &Iocs) -> Option<Finding> {
    let text = read_small(p, 1024 * 1024)?;
    let hits = patterns::scan_text(&text);
    if let Some(m) = iocs.has_marker(&text) {
        return Some(
            Finding::new(Severity::High, Area::KnownMalware, "Git hook contains a known malware marker", format!("{context} mentions \"{m}\"."))
                .path(p)
                .quarantinable(),
        );
    }
    if let Some(sev) = patterns::worst(&hits) {
        let reasons: Vec<&str> = hits.iter().map(|h| h.reason).collect();
        let mut f = Finding::new(sev, Area::Git, "Suspicious git hook", format!("{context} {}.", reasons.join("; ")))
            .path(p)
            .advice("Hooks run automatically on commit/push. If you didn't add this, quarantine it.")
            .quarantinable();
        if let Some((n, l)) = patterns::first_matching_line(&text) {
            f = f.line(n).evidence(l);
        }
        return Some(f);
    }
    let known = KNOWN_HOOK_TOOLS.iter().any(|t| text.contains(t));
    (report_unknown && !known).then(|| {
        Finding::new(Severity::Low, Area::Git, "Custom git hook", format!("{context} runs on git actions. It looks harmless, but check you recognise it."))
            .path(p)
            .evidence(patterns::excerpt(text.lines().find(|l| !l.starts_with('#') && !l.trim().is_empty()).unwrap_or("")))
            .quarantinable()
    })
}

pub fn scan(env: &Env, found: &Discovered, iocs: &Iocs) -> Vec<Finding> {
    let mut out = Vec::new();

    let name = git_global(env, "user.name");
    let email = git_global(env, "user.email");
    if name.is_some() || email.is_some() {
        out.push(Finding::new(
            Severity::Info,
            Area::Git,
            "Your git identity",
            format!(
                "New commits are signed as {} <{}>. If this isn't you, something changed your git config.",
                name.as_deref().unwrap_or("?"),
                email.as_deref().unwrap_or("?")
            ),
        ));
    }

    if let Some(hp) = git_global(env, "core.hooksPath") {
        let dir = expand_tilde(env, &hp);
        out.push(
            Finding::new(Severity::Medium, Area::Git, "Global git hooks folder is set",
                format!("Every repository on this machine runs the hooks in {}. Malware uses this to run on every commit or push.", dir.display()))
                .path(&dir)
                .advice("If you didn't set this, run: git config --global --unset core.hooksPath"),
        );
        for h in hook_files(&dir) {
            out.extend(check_hook(&h, "This global hook", false, iocs));
        }
    }
    if let Some(td) = git_global(env, "init.templateDir") {
        let dir = expand_tilde(env, &td);
        out.push(
            Finding::new(Severity::Low, Area::Git, "Git template folder is set",
                format!("New and cloned repos copy hooks from {}.", dir.display()))
                .path(&dir),
        );
        for h in hook_files(&dir.join("hooks")) {
            out.extend(check_hook(&h, "This template hook (copied into every new repo)", true, iocs));
        }
    }
    if let Some(helper) = git_global(env, "credential.helper") {
        if helper.trim() == "store" {
            out.push(
                Finding::new(Severity::Medium, Area::Secrets, "Git saves passwords in plain text",
                    "credential.helper=store keeps your GitHub token unencrypted in ~/.git-credentials.")
                    .path(&env.home.join(".git-credentials"))
                    .advice(if cfg!(target_os = "macos") { "Use the Keychain instead: git config --global credential.helper osxkeychain" } else { "Use an encrypted helper such as libsecret or Git Credential Manager." }),
            );
        }
    }

    for repo in &found.repos {
        let git_dir = repo.join(".git");
        let config = read_small(&git_dir.join("config"), 256 * 1024).unwrap_or_default();
        for h in hook_files(&git_dir.join("hooks")) {
            out.extend(check_hook(&h, "This repository hook", true, iocs));
        }
        if let Some(hp) = ini_get(&config, "core", "hooksPath") {
            let dir = if Path::new(&hp).is_absolute() { PathBuf::from(&hp) } else { repo.join(&hp) };
            for h in hook_files(&dir) {
                out.extend(check_hook(&h, "This repository hook", false, iocs));
            }
        }
        if let (Some(local), Some(global)) = (ini_get(&config, "user", "email"), email.as_ref()) {
            if !local.eq_ignore_ascii_case(global) {
                out.push(
                    Finding::new(Severity::Low, Area::Git, "Repository commits under a different email",
                        format!("Commits in this repo are signed as <{local}>, not your usual <{global}>. Fine if it's a work/personal split; suspicious if you didn't set it."))
                        .path(repo),
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ini() {
        let t = "[core]\n\thooksPath = .husky/_\n[user]\n\temail = \"a@b.c\"\n";
        assert_eq!(ini_get(t, "core", "hookspath").as_deref(), Some(".husky/_"));
        assert_eq!(ini_get(t, "user", "email").as_deref(), Some("a@b.c"));
        assert_eq!(ini_get(t, "user", "name"), None);
    }
}
