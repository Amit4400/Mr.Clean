//! The one gate every deletion passes through.
//!
//! Two policies:
//! - **cache**: the path must sit inside a root that a cleaner rule declared.
//!   User-data folders (Documents, Desktop, iCloud…) are refused even if a
//!   rule points there by mistake.
//! - **user file**: a file the user picked in the large-file explorer. It must
//!   be inside home, not a top-level home folder, and not in a credential or
//!   app-data location.
//!
//! Paths are resolved (parent canonicalized) before checking, so `..` and
//! symlinked parent folders cannot escape. A symlink as the final component is
//! removed as a link; its target is never touched.

use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use thiserror::Error;

use crate::env::Env;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SafetyError {
    #[error("path must be absolute")]
    NotAbsolute,
    #[error("path does not exist")]
    Missing,
    #[error("path is outside the folders this action may touch")]
    OutsideAllowedRoots,
    #[error("path is a protected location: {0}")]
    Protected(String),
}

/// Home-relative folders the cleaner must never reach into.
const USER_DATA: &[&str] = &[
    "Documents",
    "Desktop",
    "Downloads",
    "Pictures",
    "Movies",
    "Music",
    "Public",
    "Videos",
    "Library/Mobile Documents",
    "Library/CloudStorage",
    "Library/Mail",
    "Library/Messages",
    "Library/Photos",
    "OneDrive",
    "Dropbox",
    "iCloudDrive",
    "Favorites",
    "Contacts",
    "Saved Games",
    "Links",
];

/// Home-relative locations nothing may delete, whatever the policy.
const CRITICAL: &[&str] = &[
    ".ssh",
    ".gnupg",
    ".aws",
    ".kube",
    ".config/gh",
    ".mrclean",
    "Library/Keychains",
    "Library/Preferences",
    // Windows: saved passwords, certificates and the keys that protect them.
    "AppData/Roaming/Microsoft/Credentials",
    "AppData/Local/Microsoft/Credentials",
    "AppData/Roaming/Microsoft/Protect",
    "AppData/Roaming/Microsoft/SystemCertificates",
    "AppData/Local/Microsoft/Vault",
];

/// App-data folders (macOS ~/Library, Windows ~/AppData): a user-picked file
/// may only be deleted from the listed caches and logs inside them.
const APP_DATA_USER_ALLOWED: &[(&str, &[&str])] = &[
    ("Library", &["Library/Caches", "Library/Logs", "Library/Developer"]),
    ("AppData", &["AppData/Local/Temp", "AppData/Local/CrashDumps"]),
];

/// Absolute system locations nothing may delete.
const SYSTEM: &[&str] = &[
    "/System",
    "/usr",
    "/bin",
    "/sbin",
    "/etc",
    "/private",
    "/Library/Keychains",
    "/Applications",
    "/boot",
    "/dev",
    "/proc",
    "/sys",
    "/var",
    "/lib",
    "/lib64",
    "/opt",
    // Windows (relative to the system drive).
    "/Windows",
    "/Program Files",
    "/Program Files (x86)",
    "/ProgramData",
    "/Users/Public",
    "/Users/Default",
    "/Recovery",
    "/$Recycle.Bin",
];

pub struct Guard<'a> {
    env: &'a Env,
}

impl<'a> Guard<'a> {
    pub fn new(env: &'a Env) -> Self {
        Guard { env }
    }

    /// Resolve a path without following its final component.
    pub fn resolve(path: &Path) -> Result<PathBuf, SafetyError> {
        if !path.is_absolute() {
            return Err(SafetyError::NotAbsolute);
        }
        if path.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(SafetyError::OutsideAllowedRoots);
        }
        std::fs::symlink_metadata(path).map_err(|_| SafetyError::Missing)?;
        let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
            return Err(SafetyError::Protected("filesystem root".into()));
        };
        let parent = parent.canonicalize().map_err(|_| SafetyError::Missing)?;
        Ok(parent.join(name))
    }

    fn canon(p: &Path) -> PathBuf {
        p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
    }

    fn home(&self) -> PathBuf {
        Self::canon(&self.env.home)
    }

    fn under_home(&self, rel: &str) -> PathBuf {
        let mut p = self.home();
        for part in rel.split('/') {
            p.push(part);
        }
        p
    }

    /// Checks shared by every policy.
    fn common(&self, p: &Path) -> Result<(), SafetyError> {
        let home = self.home();
        if home.starts_with(p) {
            return Err(SafetyError::Protected("home folder or one of its parents".into()));
        }
        for rel in CRITICAL {
            if p.starts_with(self.under_home(rel)) {
                return Err(SafetyError::Protected(format!("~/{rel}")));
            }
        }
        let root = Self::canon(&self.env.root);
        for sys in SYSTEM {
            let sys_path = root.join(sys.trim_start_matches('/'));
            if p.starts_with(&sys_path) && !p.starts_with(&home) {
                return Err(SafetyError::Protected((*sys).into()));
            }
        }
        Ok(())
    }

    /// For cleaner rules: `path` must equal or sit inside one of `roots`.
    pub fn check_cache(&self, path: &Path, roots: &[PathBuf]) -> Result<PathBuf, SafetyError> {
        let p = Self::resolve(path)?;
        self.common(&p)?;
        for rel in USER_DATA {
            if p.starts_with(self.under_home(rel)) {
                return Err(SafetyError::Protected(format!("~/{rel}")));
            }
        }
        let inside = roots.iter().any(|r| p.starts_with(Self::canon(r)));
        if !inside {
            return Err(SafetyError::OutsideAllowedRoots);
        }
        Ok(p)
    }

    /// For Xcode simulator runtimes, which are root-owned: only a `<UUID>.dmg`
    /// file directly in CoreSimulator's Images folder, or a `*.simruntime`
    /// folder directly in its Profiles/Runtimes folder. Never a symlink.
    pub fn check_simulator_runtime(&self, path: &Path) -> Result<PathBuf, SafetyError> {
        use crate::simruntime::{is_runtime_bundle_name, is_runtime_image_name, BUNDLES, IMAGES};
        let p = Self::resolve(path)?;
        let meta = std::fs::symlink_metadata(&p).map_err(|_| SafetyError::Missing)?;
        if meta.file_type().is_symlink() {
            return Err(SafetyError::Protected("a link, not a runtime".into()));
        }
        let root = Self::canon(&self.env.root);
        let dir = |rel: &str| Self::canon(&root.join(rel.trim_start_matches('/')));
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let parent = p.parent().ok_or(SafetyError::OutsideAllowedRoots)?;
        let ok = (parent == dir(IMAGES) && meta.is_file() && is_runtime_image_name(&name))
            || (parent == dir(BUNDLES) && meta.is_dir() && is_runtime_bundle_name(&name));
        if ok {
            Ok(p)
        } else {
            Err(SafetyError::OutsideAllowedRoots)
        }
    }

    /// For files the user picked by hand in the large-file explorer.
    pub fn check_user_file(&self, path: &Path) -> Result<PathBuf, SafetyError> {
        let p = Self::resolve(path)?;
        self.common(&p)?;
        let home = self.home();
        if !p.starts_with(&home) {
            return Err(SafetyError::OutsideAllowedRoots);
        }
        // Never a top-level home folder itself (~/Documents, ~/Library…).
        if p.parent() == Some(home.as_path()) && p.is_dir() {
            return Err(SafetyError::Protected("top-level home folder".into()));
        }
        for rel in USER_DATA {
            if p == self.under_home(rel) {
                return Err(SafetyError::Protected(format!("~/{rel}")));
            }
        }
        for (dir, allowed) in APP_DATA_USER_ALLOWED {
            if p.starts_with(self.under_home(dir)) && !allowed.iter().any(|rel| p.starts_with(self.under_home(rel))) {
                return Err(SafetyError::Protected(format!(
                    "~/{dir} (app data — use the cleaner instead)"
                )));
            }
        }
        // Windows keeps the user's registry in ntuser.dat* files in the home folder.
        let top_level = p.parent() == Some(home.as_path());
        if top_level
            && p.file_name()
                .is_some_and(|n| n.to_string_lossy().to_ascii_lowercase().starts_with("ntuser"))
        {
            return Err(SafetyError::Protected("Windows user registry".into()));
        }
        if p.components().any(|c| c.as_os_str() == ".git") {
            return Err(SafetyError::Protected("inside a git repository's .git folder".into()));
        }
        Ok(p)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeleteMode {
    /// Move to the system Trash (default, undo-able).
    Trash,
    /// Delete immediately.
    Permanent,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct DeleteReport {
    pub removed: Vec<Removed>,
    pub failed: Vec<Failed>,
    pub bytes_freed: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Removed {
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Failed {
    pub path: String,
    pub error: String,
}

impl DeleteReport {
    pub fn fail(&mut self, path: &Path, error: impl ToString) {
        self.failed.push(Failed {
            path: path.display().to_string(),
            error: error.to_string(),
        });
    }

    pub fn merge(&mut self, other: DeleteReport) {
        self.bytes_freed += other.bytes_freed;
        self.removed.extend(other.removed);
        self.failed.extend(other.failed);
    }
}

/// Remove an already-checked path. Callers must get `path` from a `Guard`.
pub fn remove_checked(path: &Path, mode: DeleteMode, report: &mut DeleteReport) {
    let bytes = crate::fsutil::path_size(path, None, None);
    let result = match mode {
        DeleteMode::Trash => move_to_trash(path),
        DeleteMode::Permanent => {
            let meta = std::fs::symlink_metadata(path);
            match meta {
                Ok(m) if m.is_dir() => std::fs::remove_dir_all(path).map_err(|e| e.to_string()),
                Ok(_) => std::fs::remove_file(path).map_err(|e| e.to_string()),
                Err(e) => Err(e.to_string()),
            }
        }
    };
    match result {
        Ok(()) => {
            report.bytes_freed += bytes;
            report.removed.push(Removed {
                path: path.display().to_string(),
                bytes,
            });
        }
        Err(e) => report.fail(path, e),
    }
}

/// Quote for `/bin/sh` inside single quotes.
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// The shell script run with admin rights to remove runtimes: detach each
/// mounted image, then delete it. Built only from guard-checked paths.
pub fn runtime_removal_script(items: &[(PathBuf, Option<PathBuf>)]) -> String {
    let mut parts = Vec::new();
    for (path, mount) in items {
        if let Some(m) = mount {
            parts.push(format!(
                "/usr/bin/hdiutil detach -force {} >/dev/null 2>&1",
                sh_quote(&m.to_string_lossy())
            ));
        }
        parts.push(format!("/bin/rm -rf {}", sh_quote(&path.to_string_lossy())));
    }
    parts.join("; ")
}

/// Wrap a shell script in AppleScript that asks macOS for the admin password.
pub fn admin_applescript(script: &str) -> String {
    let escaped = script.replace('\\', "\\\\").replace('"', "\\\"");
    format!("do shell script \"{escaped}\" with administrator privileges")
}

/// Remove Xcode simulator runtimes. They belong to the system, so this uses
/// `xcrun simctl runtime delete` when Xcode is installed, and otherwise one
/// macOS password prompt. Every path passes `check_simulator_runtime` first.
pub fn remove_simulator_runtimes(env: &Env, paths: &[PathBuf], xcode: bool, report: &mut DeleteReport) {
    let guard = Guard::new(env);
    let mut checked = Vec::new();
    for p in paths {
        match guard.check_simulator_runtime(p) {
            Ok(c) => checked.push((c.clone(), crate::fsutil::path_size(&c, None, None))),
            Err(e) => report.fail(p, e),
        }
    }
    if checked.is_empty() {
        return;
    }
    if !cfg!(target_os = "macos") {
        for (p, _) in &checked {
            report.fail(p, "simulator runtimes can only be removed on macOS");
        }
        return;
    }
    let mut left: Vec<(PathBuf, u64)> = Vec::new();
    for (p, bytes) in checked {
        let uuid = p
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".dmg"))
            .map(str::to_string);
        let deleted = xcode
            && uuid.is_some_and(|id| {
                std::process::Command::new("/usr/bin/xcrun")
                    .args(["simctl", "runtime", "delete", &id])
                    .status()
                    .is_ok_and(|s| s.success())
            })
            && !p.exists();
        if deleted {
            report.bytes_freed += bytes;
            report.removed.push(Removed {
                path: p.display().to_string(),
                bytes,
            });
        } else {
            left.push((p, bytes));
        }
    }
    if left.is_empty() {
        return;
    }
    let mounts = crate::simruntime::mounts();
    let items: Vec<_> = left
        .iter()
        .map(|(p, _)| {
            (
                p.clone(),
                crate::simruntime::mount_of(p, &mounts).map(Path::to_path_buf),
            )
        })
        .collect();
    let script = runtime_removal_script(&items);
    let out = std::process::Command::new("/usr/bin/osascript")
        .args(["-e", &admin_applescript(&script)])
        .output();
    let cancelled = out
        .as_ref()
        .is_ok_and(|o| String::from_utf8_lossy(&o.stderr).contains("-128"));
    for (p, bytes) in left {
        if !p.exists() {
            report.bytes_freed += bytes;
            report.removed.push(Removed {
                path: p.display().to_string(),
                bytes,
            });
        } else if cancelled {
            report.fail(&p, "cancelled — no password was entered");
        } else {
            report.fail(&p, "macOS didn't allow removing it");
        }
    }
}

fn move_to_trash(path: &Path) -> Result<(), String> {
    #[allow(unused_mut)]
    let mut ctx = trash::TrashContext::default();
    #[cfg(target_os = "macos")]
    {
        // Finder-based trashing pops permission prompts and is slow.
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        ctx.set_delete_method(DeleteMethod::NsFileManager);
    }
    ctx.delete(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Os;
    use std::fs;

    fn setup() -> (tempfile::TempDir, Env) {
        let dir = tempfile::tempdir().unwrap();
        let env = Env::sandboxed(dir.path(), Os::Mac);
        for d in [
            "Library/Caches/Foo",
            "Library/Developer/Xcode/DerivedData/App-abc",
            "Documents/work",
            "Downloads",
            ".ssh",
            "Library/Preferences",
            "projects/app/.git",
        ] {
            fs::create_dir_all(env.home.join(d)).unwrap();
        }
        fs::create_dir_all(env.root.join("System")).unwrap();
        fs::write(env.home.join("Documents/work/report.pdf"), b"x").unwrap();
        fs::write(env.home.join("Downloads/big.dmg"), b"x").unwrap();
        fs::write(env.home.join(".ssh/id_ed25519"), b"x").unwrap();
        (dir, env)
    }

    #[test]
    fn cache_allows_inside_root() {
        let (_d, env) = setup();
        let g = Guard::new(&env);
        let root = env.home.join("Library/Developer/Xcode/DerivedData");
        assert!(g
            .check_cache(&root.join("App-abc"), std::slice::from_ref(&root))
            .is_ok());
        assert!(g.check_cache(&root, std::slice::from_ref(&root)).is_ok());
    }

    #[test]
    fn cache_refuses_outside_root_and_user_data() {
        let (_d, env) = setup();
        let g = Guard::new(&env);
        let root = env.home.join("Library/Caches");
        assert_eq!(
            g.check_cache(&env.home.join("Downloads/big.dmg"), std::slice::from_ref(&root)),
            Err(SafetyError::Protected("~/Downloads".into()))
        );
        // Even a bad rule that lists Documents as a root is refused.
        let docs = env.home.join("Documents");
        assert!(matches!(
            g.check_cache(&docs.join("work"), std::slice::from_ref(&docs)),
            Err(SafetyError::Protected(_))
        ));
        assert_eq!(
            g.check_cache(&env.home.join("Library/Preferences"), &[env.home.join("Library")]),
            Err(SafetyError::Protected("~/Library/Preferences".into()))
        );
    }

    #[test]
    fn refuses_home_parents_dotdot_and_relative() {
        let (_d, env) = setup();
        let g = Guard::new(&env);
        let home = env.home.clone();
        assert!(matches!(
            g.check_cache(&home, std::slice::from_ref(&home)),
            Err(SafetyError::Protected(_))
        ));
        let sneaky = home.join("Library/Caches/../../Documents/work");
        assert!(g.check_cache(&sneaky, &[home.join("Library/Caches")]).is_err());
        assert_eq!(
            g.check_cache(Path::new("Library/Caches"), &[]),
            Err(SafetyError::NotAbsolute)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_parent_cannot_escape() {
        let (_d, env) = setup();
        let g = Guard::new(&env);
        let caches = env.home.join("Library/Caches");
        std::os::unix::fs::symlink(env.home.join("Documents"), caches.join("evil")).unwrap();
        // Through the link, the real location is ~/Documents/work: refused.
        let through = caches.join("evil/work");
        assert!(g.check_cache(&through, std::slice::from_ref(&caches)).is_err());
        // The link itself is fine to remove; its target is untouched.
        assert!(g
            .check_cache(&caches.join("evil"), std::slice::from_ref(&caches))
            .is_ok());
    }

    #[test]
    fn user_file_policy() {
        let (_d, env) = setup();
        let g = Guard::new(&env);
        let h = &env.home;
        assert!(g.check_user_file(&h.join("Downloads/big.dmg")).is_ok());
        assert!(g.check_user_file(&h.join("Documents/work/report.pdf")).is_ok());
        assert!(g.check_user_file(&h.join("Documents")).is_err());
        assert!(g.check_user_file(&h.join(".ssh/id_ed25519")).is_err());
        assert!(g.check_user_file(&h.join("Library/Preferences")).is_err());
        assert!(g.check_user_file(&h.join("Library/Caches/Foo")).is_ok());
        assert!(g.check_user_file(&h.join("projects/app/.git")).is_err());
        assert!(g.check_user_file(&env.root.join("System")).is_err());
    }

    #[test]
    fn permanent_remove_reports_bytes() {
        let (_d, env) = setup();
        let target = env.home.join("Library/Caches/Foo");
        fs::write(target.join("blob"), vec![0u8; 10_000]).unwrap();
        let checked = Guard::new(&env)
            .check_cache(&target, &[env.home.join("Library/Caches")])
            .unwrap();
        let mut report = DeleteReport::default();
        remove_checked(&checked, DeleteMode::Permanent, &mut report);
        assert!(!target.exists());
        assert_eq!(report.removed.len(), 1);
        assert!(report.bytes_freed > 0);
    }

    #[test]
    fn simulator_runtime_policy() {
        let (_d, env) = setup();
        let g = Guard::new(&env);
        let images = env.root.join("Library/Developer/CoreSimulator/Images");
        let bundles = env.root.join("Library/Developer/CoreSimulator/Profiles/Runtimes");
        fs::create_dir_all(&images).unwrap();
        fs::create_dir_all(bundles.join("iOS 17.5.simruntime")).unwrap();
        let dmg = images.join("7D832126-03E7-45A9-86CF-E1E9A1B2C3D4.dmg");
        fs::write(&dmg, b"x").unwrap();
        fs::write(images.join("images.plist"), b"x").unwrap();
        fs::write(images.join("other.dmg"), b"x").unwrap();

        assert!(g.check_simulator_runtime(&dmg).is_ok());
        assert!(g.check_simulator_runtime(&bundles.join("iOS 17.5.simruntime")).is_ok());
        assert!(g.check_simulator_runtime(&images.join("images.plist")).is_err());
        assert!(
            g.check_simulator_runtime(&images.join("other.dmg")).is_err(),
            "not a UUID name"
        );
        assert!(g.check_simulator_runtime(&images).is_err(), "the folder itself");
        assert!(g
            .check_simulator_runtime(&images.join("../Images/7D832126-03E7-45A9-86CF-E1E9A1B2C3D4.dmg"))
            .is_err());
        assert!(g.check_simulator_runtime(&env.home.join("Library/Caches/Foo")).is_err());
        assert!(g.check_simulator_runtime(&env.root.join("System")).is_err());

        // A UUID-named link pointing elsewhere is refused.
        #[cfg(unix)]
        {
            let link = images.join("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE.dmg");
            std::os::unix::fs::symlink(env.home.join("Documents/work"), &link).unwrap();
            assert!(g.check_simulator_runtime(&link).is_err());
        }
        // A runtime-shaped name in any other folder is refused.
        let elsewhere = env.home.join("Downloads/7D832126-03E7-45A9-86CF-E1E9A1B2C3D4.dmg");
        fs::write(&elsewhere, b"x").unwrap();
        assert!(g.check_simulator_runtime(&elsewhere).is_err());
    }

    #[test]
    fn runtime_removal_script_quotes_everything() {
        let script = runtime_removal_script(&[
            (
                PathBuf::from("/Library/Developer/CoreSimulator/Images/7D83.dmg"),
                Some(PathBuf::from("/Library/Developer/CoreSimulator/Volumes/iOS_21F79")),
            ),
            (
                PathBuf::from("/Library/Developer/CoreSimulator/Profiles/Runtimes/it's iOS 16.simruntime"),
                None,
            ),
        ]);
        assert_eq!(
            script,
            "/usr/bin/hdiutil detach -force '/Library/Developer/CoreSimulator/Volumes/iOS_21F79' >/dev/null 2>&1; \
             /bin/rm -rf '/Library/Developer/CoreSimulator/Images/7D83.dmg'; \
             /bin/rm -rf '/Library/Developer/CoreSimulator/Profiles/Runtimes/it'\\''s iOS 16.simruntime'"
        );
        let apple = admin_applescript(r#"echo "a\b""#);
        assert_eq!(
            apple,
            r#"do shell script "echo \"a\\b\"" with administrator privileges"#
        );
    }

    #[test]
    fn windows_layout_is_protected() {
        let dir = tempfile::tempdir().unwrap();
        let env = Env::sandboxed(dir.path(), Os::Windows);
        let h = &env.home;
        for d in [
            "AppData/Local/Temp/setup",
            "AppData/Roaming/Slack",
            "AppData/Roaming/Microsoft/Credentials",
            "Downloads",
        ] {
            fs::create_dir_all(h.join(d)).unwrap();
        }
        fs::write(h.join("NTUSER.DAT"), b"x").unwrap();
        fs::write(h.join("Downloads/setup.exe"), b"x").unwrap();
        fs::create_dir_all(env.root.join("Windows/System32")).unwrap();
        fs::create_dir_all(env.root.join("Program Files/App")).unwrap();
        let g = Guard::new(&env);
        assert!(g.check_user_file(&h.join("Downloads/setup.exe")).is_ok());
        assert!(g.check_user_file(&h.join("AppData/Local/Temp/setup")).is_ok());
        assert!(g.check_user_file(&h.join("AppData/Roaming/Slack")).is_err());
        assert!(g.check_user_file(&h.join("NTUSER.DAT")).is_err());
        let all = [env.root.clone(), h.clone()];
        assert!(g.check_cache(&env.root.join("Windows/System32"), &all).is_err());
        assert!(g.check_cache(&env.root.join("Program Files/App"), &all).is_err());
        assert!(g
            .check_cache(&h.join("AppData/Roaming/Microsoft/Credentials"), &all)
            .is_err());
    }
}
