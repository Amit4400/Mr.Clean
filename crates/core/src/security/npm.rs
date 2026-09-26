//! npm supply chain: compromised package versions, worm payload files,
//! malicious install scripts, and plaintext npm tokens.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::discover::Discovered;
use super::iocs::Iocs;
use super::{patterns, read_small, Area, Finding, Severity};
use crate::env::Env;
use crate::fsutil::Cancel;

const INSTALL_HOOKS: &[&str] = &["preinstall", "install", "postinstall", "prepare"];

/// Global node_modules folders (Homebrew, system, nvm, fnm, volta).
fn global_roots(env: &Env) -> Vec<PathBuf> {
    let mut roots = vec![
        env.root.join("usr/local/lib/node_modules"),
        env.root.join("opt/homebrew/lib/node_modules"),
        env.root.join("usr/lib/node_modules"),
    ];
    for base in [".nvm/versions/node", "Library/Application Support/fnm/node-versions", ".local/share/fnm/node-versions"] {
        if let Ok(rd) = std::fs::read_dir(env.home.join(base)) {
            for v in rd.flatten() {
                roots.push(v.path().join("lib/node_modules"));
                roots.push(v.path().join("installation/lib/node_modules"));
            }
        }
    }
    roots.push(env.home.join(".volta/tools/image/packages"));
    roots.into_iter().filter(|p| p.is_dir()).collect()
}

/// Package folders directly in a node_modules (including @scope/pkg).
fn packages_in(nm: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(nm) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || !p.is_dir() {
            continue;
        }
        if name.starts_with('@') {
            if let Ok(inner) = std::fs::read_dir(&p) {
                out.extend(inner.flatten().map(|x| x.path()).filter(|x| x.is_dir()));
            }
        } else {
            out.push(p);
        }
    }
    out
}

fn check_package(dir: &Path, iocs: &Iocs, out: &mut Vec<Finding>) {
    let Some(text) = read_small(&dir.join("package.json"), 1024 * 1024) else { return };
    let Ok(pkg) = serde_json::from_str::<Value>(&text) else { return };
    let name = pkg["name"].as_str().unwrap_or_default();
    let version = pkg["version"].as_str().unwrap_or_default();

    if iocs.is_compromised(name, version) {
        out.push(
            Finding::new(Severity::High, Area::Npm, format!("Compromised package installed: {name}@{version}"),
                "This exact version was published by attackers who hijacked the maintainer's npm account.")
                .path(dir)
                .advice("Delete node_modules, pin a safe version, reinstall, and rotate any tokens this machine has (npm, GitHub, cloud)."),
        );
    }
    for f in &iocs.package_files {
        let p = dir.join(f);
        if p.is_file() {
            out.push(
                Finding::new(Severity::High, Area::KnownMalware, format!("Shai-Hulud worm payload in {name}"),
                    format!("{f} is the loader of the Shai-Hulud 2.0 npm worm, which steals tokens and publishes infected packages under your account."))
                    .path(&p)
                    .advice("Quarantine it, delete this project's node_modules, rotate GitHub/npm/cloud tokens, and check GitHub for repos named \"Sha1-Hulud\".")
                    .quarantinable(),
            );
        }
    }
    let Some(scripts) = pkg["scripts"].as_object() else { return };
    for hook in INSTALL_HOOKS {
        let Some(cmd) = scripts.get(*hook).and_then(Value::as_str) else { continue };
        if let Some(m) = iocs.install_script_markers.iter().find(|m| cmd.contains(m.as_str())) {
            // `node bundle.js` alone is common; it's only the worm if the file is large & obfuscated.
            let bundle_only = m == "node bundle.js";
            let obfuscated = !bundle_only
                || read_small(&dir.join("bundle.js"), 16 * 1024 * 1024)
                    .is_some_and(|t| patterns::worst(&patterns::scan_text(&t)) >= Some(Severity::Medium));
            if obfuscated {
                out.push(
                    Finding::new(Severity::High, Area::KnownMalware, format!("Worm install script in {name}"),
                        format!("The \"{hook}\" script runs `{cmd}`, which matches the Shai-Hulud npm worm."))
                        .path(&dir.join("package.json"))
                        .evidence(cmd.to_string()),
                );
                continue;
            }
        }
        let hits = patterns::scan_text(cmd);
        if let Some(sev) = patterns::worst(&hits) {
            out.push(
                Finding::new(sev.max(Severity::Medium), Area::Npm, format!("Risky install script in {name}"),
                    format!("Its \"{hook}\" script {} — this runs automatically on npm install.", hits[0].reason))
                    .path(&dir.join("package.json"))
                    .evidence(patterns::excerpt(cmd)),
            );
        }
    }
}

/// package-lock.json v2/v3 ("packages") and v1 ("dependencies") entries.
fn lock_entries(lock: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(pkgs) = lock["packages"].as_object() {
        for (k, v) in pkgs {
            let Some(name) = k.rsplit("node_modules/").next().filter(|n| !n.is_empty()) else { continue };
            if let Some(ver) = v["version"].as_str() {
                out.push((name.to_string(), ver.to_string()));
            }
        }
    } else if let Some(deps) = lock["dependencies"].as_object() {
        fn walk(deps: &serde_json::Map<String, Value>, out: &mut Vec<(String, String)>) {
            for (name, v) in deps {
                if let Some(ver) = v["version"].as_str() {
                    out.push((name.clone(), ver.to_string()));
                }
                if let Some(inner) = v["dependencies"].as_object() {
                    walk(inner, out);
                }
            }
        }
        walk(deps, &mut out);
    }
    out
}

fn check_npmrc(env: &Env, out: &mut Vec<Finding>) {
    let p = env.home.join(".npmrc");
    let Some(text) = read_small(&p, 256 * 1024) else { return };
    for (i, line) in text.lines().enumerate() {
        let Some((_, value)) = line.split_once("_authToken=") else { continue };
        let value = value.trim();
        if value.starts_with("${") {
            continue;
        }
        out.push(
            Finding::new(Severity::Medium, Area::Secrets, "npm token saved in plain text",
                "~/.npmrc holds a publish token. npm worms read this file to publish malware under your name.")
                .path(&p)
                .line(i + 1)
                .evidence(patterns::mask(value))
                .advice("Use a short-lived or read-only token, enable 2FA for publishing on npmjs.com, or keep the token in an environment variable."),
        );
    }
}

pub fn scan(env: &Env, found: &Discovered, iocs: &Iocs, cancel: &Cancel) -> (Vec<Finding>, usize) {
    let mut out = Vec::new();
    check_npmrc(env, &mut out);
    let mut count = 0;
    let mut dirs: Vec<PathBuf> = found.node_modules.clone();
    dirs.extend(global_roots(env));
    for nm in &dirs {
        if cancel.is_cancelled() {
            break;
        }
        for pkg in packages_in(nm) {
            count += 1;
            check_package(&pkg, iocs, &mut out);
        }
    }
    for lock in &found.lockfiles {
        let Some(text) = read_small(lock, 64 * 1024 * 1024) else { continue };
        let Ok(v) = serde_json::from_str::<Value>(&text) else { continue };
        for (name, ver) in lock_entries(&v) {
            if iocs.is_compromised(&name, &ver) {
                out.push(
                    Finding::new(Severity::High, Area::Npm, format!("Lockfile pins a compromised package: {name}@{ver}"),
                        "Installing this project will pull in a version published by attackers.")
                        .path(lock)
                        .advice("Update the dependency to a safe version and regenerate the lockfile before running npm install."),
                );
            }
        }
    }
    (out, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockfile_versions() {
        let v2: Value = serde_json::from_str(r#"{"packages":{"":{},"node_modules/chalk":{"version":"5.6.1"},"node_modules/a/node_modules/@ctrl/tinycolor":{"version":"4.1.1"}}}"#).unwrap();
        let e = lock_entries(&v2);
        assert!(e.contains(&("chalk".into(), "5.6.1".into())));
        assert!(e.contains(&("@ctrl/tinycolor".into(), "4.1.1".into())));
        let v1: Value = serde_json::from_str(r#"{"dependencies":{"debug":{"version":"4.4.2","dependencies":{"ms":{"version":"2.1.3"}}}}}"#).unwrap();
        assert_eq!(lock_entries(&v1).len(), 2);
    }
}
