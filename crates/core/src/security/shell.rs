//! Shell startup files (code that runs in every terminal) and tokens left in
//! plain text in profiles, history and dotfiles.

use std::sync::OnceLock;

use regex::Regex;

use super::{patterns, read_small, Area, Finding, Severity};
use crate::env::Env;

const PROFILES: &[&str] = &[
    ".zshrc", ".zprofile", ".zshenv", ".zlogin", ".bashrc", ".bash_profile", ".profile",
    ".config/fish/config.fish",
];
const HISTORIES: &[&str] = &[".zsh_history", ".bash_history", ".local/share/fish/fish_history"];

fn token_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(concat!(
            r"(ghp_[A-Za-z0-9]{36}|gho_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{40,}",
            r"|npm_[A-Za-z0-9]{36}|AKIA[0-9A-Z]{16}|xox[baprs]-[A-Za-z0-9-]{10,}",
            r"|sk-ant-[A-Za-z0-9_-]{20,}|sk-(proj-)?[A-Za-z0-9_-]{32,}|glpat-[A-Za-z0-9_-]{20})"
        ))
        .unwrap()
    })
}

pub fn scan(env: &Env) -> Vec<Finding> {
    let mut out = Vec::new();
    for rel in PROFILES {
        let p = env.home.join(rel);
        let Some(text) = read_small(&p, 4 * 1024 * 1024) else { continue };
        for (i, line) in text.lines().enumerate() {
            if line.trim_start().starts_with('#') {
                continue;
            }
            let hits = patterns::scan_text(line);
            if let Some(sev) = patterns::worst(&hits) {
                out.push(
                    Finding::new(sev, Area::ShellProfile, format!("Suspicious line in ~/{rel}"),
                        format!("This runs every time you open a terminal and {}.", hits[0].reason))
                        .path(&p)
                        .line(i + 1)
                        .evidence(patterns::excerpt(line))
                        .advice(format!("If you didn't add it, delete line {} from ~/{rel}.", i + 1)),
                );
            }
            if let Some(m) = token_re().find(line) {
                out.push(
                    Finding::new(Severity::Low, Area::Secrets, format!("Token stored in ~/{rel}"),
                        "An access token is written in plain text in your shell profile. Any script you run can read it.")
                        .path(&p)
                        .line(i + 1)
                        .evidence(patterns::mask(m.as_str()))
                        .advice("Move it to your password manager or macOS Keychain, and rotate it if you've run untrusted code."),
                );
            }
        }
    }
    for rel in HISTORIES {
        let p = env.home.join(rel);
        let Some(text) = read_small(&p, 64 * 1024 * 1024) else { continue };
        let found: Vec<&str> = token_re().find_iter(&text).map(|m| m.as_str()).collect();
        if let Some(first) = found.first() {
            out.push(
                Finding::new(Severity::Medium, Area::Secrets, format!("{} token(s) in your shell history", found.len()),
                    format!("~/{rel} contains access tokens you typed or pasted. Malware (like the Shai-Hulud worm) collects these."))
                    .path(&p)
                    .evidence(patterns::mask(first))
                    .advice("Rotate these tokens, then clear them from the history file."),
            );
        }
    }
    let creds = env.home.join(".git-credentials");
    if creds.is_file() {
        out.push(
            Finding::new(Severity::Medium, Area::Secrets, "Plain-text git credentials",
                "~/.git-credentials stores your git host passwords/tokens unencrypted.")
                .path(&creds)
                .advice("Switch to the macOS Keychain helper and delete this file after rotating the tokens."),
        );
    }
    if let Ok(rd) = std::fs::read_dir(&env.home) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name == ".env" || name.starts_with(".env.") {
                out.push(
                    Finding::new(Severity::Low, Area::Secrets, format!("~/{name} in your home folder"),
                        "Environment files usually hold API keys. Keeping one in your home folder exposes it to every tool you run.")
                        .path(&e.path()),
                );
            }
        }
    }
    out
}
