//! One walk over the user's code folders collecting everything the security
//! checks need: git repos, node_modules, JS config files, workflows, and files
//! whose names match known malware artifacts.

use std::path::PathBuf;

use super::iocs::Iocs;
use super::{patterns, read_small, Area, Finding, Severity};
use crate::env::Env;
use crate::fsutil::Cancel;

const MAX_DEPTH: usize = 8;
const SKIP_AT_HOME: &[&str] = &["Library", "Applications", "Pictures", "Movies", "Music", "AppData"];
/// Build output and vendored code: huge and not where attackers hide config.
const SKIP_ANYWHERE: &[&str] = &[
    "node_modules",
    "Pods",
    "DerivedData",
    "build",
    "dist",
    ".next",
    ".nuxt",
    "target",
    ".gradle",
    ".dart_tool",
    "vendor",
    ".venv",
    "venv",
    "__pycache__",
    ".cache",
    ".turbo",
];

#[derive(Debug, Default)]
pub struct Discovered {
    pub repos: Vec<PathBuf>,
    pub node_modules: Vec<PathBuf>,
    pub lockfiles: Vec<PathBuf>,
    pub js_configs: Vec<PathBuf>,
    pub workflows: Vec<PathBuf>,
    pub ioc_files: Vec<PathBuf>,
    pub projects: usize,
}

fn is_js_config(name: &str) -> bool {
    let is_js = [".js", ".cjs", ".mjs", ".ts"].iter().any(|e| name.ends_with(e));
    is_js
        && (name.contains(".config.")
            || name.starts_with(".eslintrc")
            || name == "gulpfile.js"
            || name == "Gruntfile.js")
}

pub fn discover(env: &Env, roots: &[PathBuf], iocs: &Iocs, cancel: &Cancel) -> Discovered {
    let mut d = Discovered::default();
    for root in roots {
        let home = env.home.clone();
        let walker = jwalk::WalkDir::new(root)
            .follow_links(false)
            .skip_hidden(false)
            .max_depth(MAX_DEPTH)
            .process_read_dir(move |_, dir, _, children| {
                let at_home = dir == home.as_path();
                for c in children.iter_mut().flatten() {
                    let name = c.file_name.to_string_lossy();
                    let hidden_keep = name == ".github" || name == ".git" || name == ".husky";
                    let prune = SKIP_ANYWHERE.contains(&name.as_ref())
                        || name == ".git"
                        || (name.starts_with('.') && !hidden_keep)
                        || (at_home && SKIP_AT_HOME.contains(&name.as_ref()));
                    if prune && c.file_type.is_dir() {
                        c.read_children = None;
                    }
                }
            });
        for entry in walker {
            if cancel.is_cancelled() {
                break;
            }
            let Ok(e) = entry else { continue };
            let name = e.file_name().to_string_lossy().to_string();
            let path = e.path();
            if e.file_type().is_dir() {
                if name == ".git" {
                    d.repos.push(e.parent_path().to_path_buf());
                } else if name == "node_modules" && e.parent_path().join("package.json").exists() {
                    d.node_modules.push(path);
                }
                continue;
            }
            match name.as_str() {
                "package.json" => d.projects += 1,
                "package-lock.json" => d.lockfiles.push(path.clone()),
                _ => {}
            }
            if is_js_config(&name) {
                d.js_configs.push(path.clone());
            }
            let in_workflows = path.parent().is_some_and(|p| p.ends_with(".github/workflows"));
            if in_workflows && (name.ends_with(".yml") || name.ends_with(".yaml")) {
                d.workflows.push(path.clone());
            }
            if iocs.repo_files.contains(&name) {
                d.ioc_files.push(path);
            }
        }
    }
    d.repos.sort();
    d.repos.dedup();
    d
}

/// Findings from files the walk flagged directly (config injection, workflows, artifacts).
pub fn findings(d: &Discovered, iocs: &Iocs) -> Vec<Finding> {
    let mut out = Vec::new();
    for p in &d.ioc_files {
        out.push(
            Finding::new(
                Severity::High,
                Area::KnownMalware,
                "Stolen-secrets file from the Shai-Hulud worm",
                "The worm writes the secrets it harvests into files with this name before uploading them.",
            )
            .path(p)
            .advice(
                "Rotate every token on this machine (GitHub, npm, cloud) and check GitHub for repos you didn't create.",
            )
            .quarantinable(),
        );
    }
    for p in &d.workflows {
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let text = read_small(p, 512 * 1024).unwrap_or_default();
        let marker = iocs.has_marker(&text);
        let named = iocs.workflow_files.contains(&name);
        let hits = patterns::scan_text(&text);
        let high: Vec<_> = hits.iter().filter(|h| h.severity == Severity::High).collect();
        if named || marker.is_some() {
            out.push(
                Finding::new(
                    Severity::High,
                    Area::KnownMalware,
                    "Shai-Hulud GitHub workflow",
                    "This workflow matches the npm worm that steals secrets through GitHub Actions.",
                )
                .path(p)
                .evidence(marker.unwrap_or(&name).to_string())
                .advice("Delete the workflow from the repo on GitHub too, and rotate repository secrets.")
                .quarantinable(),
            );
        } else if let Some(h) = high.first() {
            out.push(
                Finding::new(
                    Severity::High,
                    Area::ProjectCode,
                    "Suspicious GitHub workflow",
                    format!("This workflow {}.", h.reason),
                )
                .path(p)
                .quarantinable(),
            );
        }
    }
    for p in &d.js_configs {
        let Some(text) = read_small(p, 2 * 1024 * 1024) else {
            continue;
        };
        let hits = patterns::scan_text(&text);
        let Some(sev) = patterns::worst(&hits) else { continue };
        // Config files are never legitimately minified or obfuscated, so even
        // Medium signals matter here; Low alone is noise.
        if sev < Severity::Medium {
            continue;
        }
        let reasons: Vec<&str> = hits.iter().map(|h| h.reason).collect();
        let mut f = Finding::new(sev, Area::ProjectCode, "Suspicious code in a project config file",
            format!("This config file {}. Fake job-interview projects (Contagious Interview / BeaverTail) hide malware in files like this.", reasons.join("; ")))
            .path(p)
            .advice("Don't run npm install / npm start in this project until you've checked the file. Compare it with the original repository.");
        if let Some((n, l)) = patterns::first_matching_line(&text) {
            f = f.line(n).evidence(l);
        }
        out.push(f);
    }
    out
}
