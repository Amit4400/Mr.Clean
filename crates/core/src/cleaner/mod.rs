//! Developer-junk cleaner: measure every rule in the catalog, then remove only
//! what the user confirms.

pub mod node_modules;
pub mod rules;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::env::{Env, Os};
use crate::fsutil::{modified_secs, path_size, Cancel, Progress};
use crate::safety::{remove_checked, DeleteMode, DeleteReport, Guard};
use crate::simruntime;
pub use rules::{Category, Rule, Safety, Target};

#[derive(Debug, Clone, Serialize)]
pub struct RuleInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub category: Category,
    pub description: &'static str,
    pub safety: Safety,
    pub command: Option<&'static str>,
    pub always_permanent: bool,
}

impl From<&Rule> for RuleInfo {
    fn from(r: &Rule) -> Self {
        RuleInfo {
            id: r.id,
            name: r.name,
            category: r.category,
            description: r.description,
            safety: r.safety,
            command: r.command,
            always_permanent: r.always_permanent,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub path: String,
    pub name: String,
    pub bytes: u64,
    pub modified: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleScan {
    pub rule: RuleInfo,
    pub items: Vec<Item>,
    pub total_bytes: u64,
    /// Extra context, e.g. "Xcode isn't installed — these are leftovers".
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanerScan {
    pub rules: Vec<RuleScan>,
    pub total_bytes: u64,
    /// Bytes in `Safe` rules: what "Clean recommended" would free.
    pub safe_bytes: u64,
    /// macOS only: whether an Xcode.app exists in /Applications.
    pub xcode_installed: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CleanRequest {
    pub rule_id: String,
    /// Specific items to remove. `None` = every item of the rule.
    pub paths: Option<Vec<String>>,
}

pub fn xcode_installed(env: &Env) -> bool {
    let apps = env.root.join("Applications");
    std::fs::read_dir(apps)
        .map(|rd| {
            rd.flatten().any(|e| {
                let n = e.file_name().to_string_lossy().to_string();
                n.starts_with("Xcode") && n.ends_with(".app")
            })
        })
        .unwrap_or(false)
}

/// Entries of `dir` whose name starts with `prefix`.
fn prefixed_entries(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    rd.flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(prefix))
        .map(|e| e.path())
        .collect()
}

/// All expanded target paths of every rule, used to avoid double counting
/// (e.g. ~/Library/Caches/Yarn belongs to "Yarn", not "App caches").
fn all_targets(env: &Env, rules: &[Rule]) -> Vec<(&'static str, PathBuf)> {
    let mut out = Vec::new();
    for r in rules {
        for t in r.targets {
            let Some(base) = env.expand(t.raw()) else { continue };
            match t {
                Target::Prefixed(_, prefix) => {
                    out.extend(prefixed_entries(&base, prefix).into_iter().map(|p| (r.id, p)))
                }
                _ => out.push((r.id, base)),
            }
        }
    }
    out
}

/// Add `p` (found under this rule's `base`) unless a more specific rule owns
/// it. If another rule owns something deeper inside `p`, add `p`'s other
/// children instead, so e.g. App caches still lists `Caches/Google/Chrome`
/// while Android Studio keeps `Caches/Google/AndroidStudio*`.
fn push_unclaimed(p: PathBuf, base: &Path, rule_id: &str, owned: &[(&'static str, PathBuf)], out: &mut Vec<PathBuf>) {
    let others = || owned.iter().filter(|(id, _)| *id != rule_id);
    if others().any(|(_, o)| p.starts_with(o) && o.starts_with(base) && o != base) {
        return;
    }
    if others().any(|(_, o)| o.starts_with(&p)) {
        if let Ok(rd) = std::fs::read_dir(&p) {
            for e in rd.flatten() {
                push_unclaimed(e.path(), base, rule_id, owned, out);
            }
        }
        return;
    }
    out.push(p);
}

/// Paths a rule would remove right now: the folder itself for `Dir`, its
/// children for `Contents`, skipping anything owned by another rule.
fn rule_items(env: &Env, rule: &Rule, owned: &[(&'static str, PathBuf)]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for t in rule.targets {
        let Some(base) = env.expand(t.raw()) else { continue };
        if std::fs::symlink_metadata(&base).is_err() {
            continue;
        }
        match t {
            Target::Dir(_) => out.push(base),
            Target::Contents(_) => {
                let Ok(rd) = std::fs::read_dir(&base) else { continue };
                for e in rd.flatten() {
                    push_unclaimed(e.path(), &base, rule.id, owned, &mut out);
                }
            }
            Target::Prefixed(_, prefix) => {
                for p in prefixed_entries(&base, prefix) {
                    push_unclaimed(p, &base, rule.id, owned, &mut out);
                }
            }
        }
    }
    out
}

fn rule_roots(env: &Env, rule: &Rule) -> Vec<PathBuf> {
    rule.targets.iter().filter_map(|t| env.expand(t.raw())).collect()
}

pub fn scan(env: &Env, cancel: &Cancel, progress: &Progress) -> CleanerScan {
    let rules = rules::rules_for(env.os);
    let owned = all_targets(env, &rules);
    let xcode = (env.os == Os::Mac).then(|| xcode_installed(env));
    let mounts = simruntime::mounts();
    let mut result = CleanerScan {
        rules: vec![],
        total_bytes: 0,
        safe_bytes: 0,
        xcode_installed: xcode,
    };

    for rule in &rules {
        if cancel.is_cancelled() {
            break;
        }
        let runtimes = rule.safety == Safety::NeedsPassword;
        let mut items: Vec<Item> = rule_items(env, rule, &owned)
            .into_iter()
            .filter(|p| !runtimes || simruntime::is_runtime_item(p))
            .map(|p| Item {
                name: if runtimes {
                    simruntime::label(&p, &mounts)
                } else {
                    p.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                },
                bytes: path_size(&p, Some(cancel), Some(progress)),
                modified: modified_secs(&p),
                path: p.display().to_string(),
            })
            .filter(|i| i.bytes > 0)
            .collect();
        if items.is_empty() {
            continue;
        }
        items.sort_by_key(|a| std::cmp::Reverse(a.bytes));
        let total: u64 = items.iter().map(|i| i.bytes).sum();
        let note = match (rule.category, xcode) {
            (Category::Xcode, Some(false)) => {
                Some("Xcode isn't installed, so everything here is leftover and safe to remove.".into())
            }
            _ => None,
        };
        result.total_bytes += total;
        if rule.safety == Safety::Safe {
            result.safe_bytes += total;
        }
        result.rules.push(RuleScan {
            rule: rule.into(),
            items,
            total_bytes: total,
            note,
        });
    }
    result.rules.sort_by_key(|a| std::cmp::Reverse(a.total_bytes));
    result
}

/// Remove what the user confirmed. Every path is re-derived from the catalog
/// and checked by the safety guard; a path the UI sends that isn't a current
/// item of that rule is refused.
pub fn clean(env: &Env, requests: &[CleanRequest], mode: DeleteMode) -> DeleteReport {
    let rules = rules::rules_for(env.os);
    let owned = all_targets(env, &rules);
    let guard = Guard::new(env);
    let mut report = DeleteReport::default();

    for req in requests {
        let Some(rule) = rules.iter().find(|r| r.id == req.rule_id) else {
            report.fail(Path::new(&req.rule_id), "unknown rule");
            continue;
        };
        if rule.safety == Safety::ReportOnly {
            report.fail(Path::new(rule.name), "this item can only be freed with its command");
            continue;
        }
        let items = rule_items(env, rule, &owned);
        let chosen: Vec<PathBuf> = match &req.paths {
            None => items,
            Some(paths) => paths
                .iter()
                .map(PathBuf::from)
                .filter(|p| {
                    let ok = items.contains(p);
                    if !ok {
                        report.fail(p, "not part of this rule");
                    }
                    ok
                })
                .collect(),
        };
        if rule.safety == Safety::NeedsPassword {
            let xcode = xcode_installed(env);
            crate::safety::remove_simulator_runtimes(env, &chosen, xcode, &mut report);
            continue;
        }
        let roots = rule_roots(env, rule);
        let mode = if rule.always_permanent {
            DeleteMode::Permanent
        } else {
            mode
        };
        for p in chosen {
            match guard.check_cache(&p, &roots) {
                Ok(checked) => remove_checked(&checked, mode, &mut report),
                Err(e) => report.fail(&p, e),
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(p: &Path, n: usize) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, vec![1u8; n]).unwrap();
    }

    fn fake_mac() -> (tempfile::TempDir, Env) {
        let d = tempfile::tempdir().unwrap();
        let env = Env::sandboxed(d.path(), Os::Mac);
        let h = &env.home;
        write(&h.join("Library/Developer/Xcode/DerivedData/App-1/Build/x.o"), 50_000);
        write(&h.join("Library/Developer/Xcode/DerivedData/App-2/Index/y"), 20_000);
        write(
            &h.join("Library/Developer/CoreSimulator/Devices/UUID-1/data/app"),
            80_000,
        );
        write(&h.join("Library/Caches/Yarn/v6/pkg.tgz"), 30_000);
        write(&h.join("Library/Caches/com.someapp/cache.db"), 10_000);
        write(&h.join(".npm/_cacache/content/abc"), 40_000);
        write(&h.join("Documents/keep.txt"), 1_000);
        fs::create_dir_all(env.root.join("Applications")).unwrap();
        (d, env)
    }

    #[test]
    fn scan_finds_rules_and_does_not_double_count() {
        let (_d, env) = fake_mac();
        let s = scan(&env, &Cancel::new(), &Progress::default());
        let ids: Vec<_> = s.rules.iter().map(|r| r.rule.id).collect();
        assert!(ids.contains(&"xcode-derived-data"));
        assert!(ids.contains(&"simulator-devices"));
        assert!(ids.contains(&"yarn-cache"));
        assert!(ids.contains(&"npm-cache"));
        let caches = s.rules.iter().find(|r| r.rule.id == "user-caches").unwrap();
        assert_eq!(caches.items.len(), 1, "Yarn must be counted under yarn-cache only");
        assert_eq!(s.xcode_installed, Some(false));
        let sim = s.rules.iter().find(|r| r.rule.id == "simulator-devices").unwrap();
        assert!(sim.note.is_some());
        assert!(s.safe_bytes < s.total_bytes, "simulators are Review, not Safe");
    }

    #[test]
    fn clean_removes_only_selected_rule_items() {
        let (_d, env) = fake_mac();
        let h = env.home.clone();
        let req = [CleanRequest {
            rule_id: "xcode-derived-data".into(),
            paths: None,
        }];
        let r = clean(&env, &req, DeleteMode::Permanent);
        assert_eq!(r.removed.len(), 2, "{:?}", r.failed);
        assert!(
            h.join("Library/Developer/Xcode/DerivedData").exists(),
            "folder itself is kept"
        );
        assert!(!h.join("Library/Developer/Xcode/DerivedData/App-1").exists());
        assert!(h.join("Library/Developer/CoreSimulator/Devices/UUID-1").exists());
        assert!(h.join("Documents/keep.txt").exists());
    }

    #[test]
    fn clean_refuses_paths_smuggled_into_a_rule() {
        let (_d, env) = fake_mac();
        let h = env.home.clone();
        let req = [CleanRequest {
            rule_id: "user-caches".into(),
            paths: Some(vec![h.join("Documents/keep.txt").display().to_string()]),
        }];
        let r = clean(&env, &req, DeleteMode::Permanent);
        assert!(r.removed.is_empty());
        assert_eq!(r.failed.len(), 1);
        assert!(h.join("Documents/keep.txt").exists());
    }

    #[test]
    fn android_studio_rule_leaves_chrome_to_app_caches() {
        let (_d, env) = fake_mac();
        let h = env.home.clone();
        write(&h.join("Library/Caches/Google/Chrome/Default/cache"), 30_000);
        write(&h.join("Library/Caches/Google/AndroidStudio2024.1/index"), 20_000);
        let s = scan(&env, &Cancel::new(), &Progress::default());
        let names = |id: &str| -> Vec<String> {
            s.rules
                .iter()
                .find(|r| r.rule.id == id)
                .map(|r| r.items.iter().map(|i| i.path.clone()).collect())
                .unwrap_or_default()
        };
        let jb = names("jetbrains");
        assert_eq!(jb.len(), 1);
        assert!(jb[0].ends_with("AndroidStudio2024.1"));
        let caches = names("user-caches");
        assert!(caches.iter().any(|p| p.ends_with("Google/Chrome")), "{caches:?}");
        assert!(!caches.iter().any(|p| p.contains("AndroidStudio")));
    }

    #[test]
    fn simulator_runtimes_are_counted_once() {
        let (_d, env) = fake_mac();
        let base = env.root.join("Library/Developer/CoreSimulator");
        write(&base.join("Images/7D832126-03E7-45A9-86CF-E1E9A1B2C3D4.dmg"), 70_000);
        write(&base.join("Images/images.plist"), 500);
        // The mounted copy of that same image must not be added again.
        write(&base.join("Volumes/iOS_21F79/Library/big"), 70_000);
        let s = scan(&env, &Cancel::new(), &Progress::default());
        let rt = s.rules.iter().find(|r| r.rule.id == "simulator-runtimes").unwrap();
        assert_eq!(rt.items.len(), 1, "{:?}", rt.items);
        assert!(rt.items[0].name.starts_with("Simulator runtime 7D832126"));
        assert!(rt.total_bytes < 100_000);
    }

    #[test]
    fn windows_jetbrains_keeps_toolbox_apps() {
        let d = tempfile::tempdir().unwrap();
        let env = Env::sandboxed(d.path(), Os::Windows);
        let local = env.local_app_data.clone().unwrap();
        write(&local.join("JetBrains/IntelliJIdea2024.1/caches/x"), 10_000);
        write(&local.join("JetBrains/Toolbox/apps/IDEA-U/idea.exe"), 50_000);
        write(&local.join("Google/AndroidStudio2024.1/caches/y"), 10_000);
        write(&local.join("Temp/setup.tmp"), 5_000);
        write(&local.join("NuGet/v3-cache/pkg"), 5_000);
        let s = scan(&env, &Cancel::new(), &Progress::default());
        let jb = s.rules.iter().find(|r| r.rule.id == "jetbrains").unwrap();
        assert_eq!(jb.items.len(), 2, "{:?}", jb.items);
        assert!(jb.items.iter().all(|i| !i.path.contains("Toolbox")));
        let ids: Vec<_> = s.rules.iter().map(|r| r.rule.id).collect();
        assert!(ids.contains(&"win-temp") && ids.contains(&"nuget"), "{ids:?}");
    }

    #[test]
    fn linux_editor_caches_are_found_once() {
        let d = tempfile::tempdir().unwrap();
        let env = Env::sandboxed(d.path(), Os::Linux);
        let h = env.home.clone();
        write(&h.join(".config/Code/CachedData/abc/x"), 10_000);
        write(&h.join(".cache/JetBrains/PyCharm2024.1/index"), 10_000);
        write(&h.join(".cache/pip/http/x"), 10_000);
        write(&h.join(".cache/fontconfig/x"), 10_000);
        let s = scan(&env, &Cancel::new(), &Progress::default());
        let items = |id: &str| {
            s.rules
                .iter()
                .find(|r| r.rule.id == id)
                .map(|r| r.items.len())
                .unwrap_or(0)
        };
        assert_eq!(items("vscode"), 1);
        assert_eq!(items("jetbrains"), 1);
        assert_eq!(items("pip"), 1);
        let cache = s.rules.iter().find(|r| r.rule.id == "linux-cache").unwrap();
        assert_eq!(
            cache.items.len(),
            1,
            "only fontconfig is left for the generic rule: {:?}",
            cache.items
        );
    }

    #[test]
    fn report_only_rules_are_never_deleted() {
        let (_d, env) = fake_mac();
        write(&env.home.join("go/pkg/mod/x/y"), 1000);
        let req = [CleanRequest {
            rule_id: "go-mod".into(),
            paths: None,
        }];
        let r = clean(&env, &req, DeleteMode::Permanent);
        assert!(r.removed.is_empty());
        assert!(env.home.join("go/pkg/mod/x/y").exists());
    }
}
