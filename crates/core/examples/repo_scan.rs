//! Scan a source tree for malicious code, for CI on pull requests.
//!
//!     cargo run -p mrclean-core --example repo_scan -- <path>
//!
//! Prints GitHub Actions annotations and exits 1 if anything is High severity.

use std::path::PathBuf;

use mrclean_core::security::{self, Severity};
use mrclean_core::Cancel;

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    let root = root.canonicalize().unwrap_or(root);
    let report = security::scan_tree(&root, &Cancel::new());
    let in_actions = std::env::var_os("GITHUB_ACTIONS").is_some();

    for f in &report.findings {
        let level = match f.severity {
            Severity::High => "error",
            Severity::Medium => "warning",
            _ => "notice",
        };
        let rel = f
            .path
            .as_deref()
            .map(|p| p.strip_prefix(&format!("{}/", root.display())).unwrap_or(p).to_string());
        if in_actions {
            let loc = match (&rel, f.line) {
                (Some(p), Some(l)) => format!(" file={p},line={l}"),
                (Some(p), None) => format!(" file={p}"),
                _ => String::new(),
            };
            println!("::{level}{loc}::{} — {}", f.title, f.detail);
        } else {
            println!("[{level}] {} — {} {}", f.title, f.detail, rel.unwrap_or_default());
        }
    }
    println!(
        "Scanned {} projects, {} packages, {} repos: {} high, {} medium, {} low.",
        report.scanned_projects,
        report.scanned_packages,
        report.scanned_repos,
        report.counts.high,
        report.counts.medium,
        report.counts.low
    );
    if report.counts.high > 0 {
        std::process::exit(1);
    }
}
