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
];

/// Inside ~/Library a user-picked file may only be deleted from these.
const LIBRARY_USER_ALLOWED: &[&str] = &["Library/Caches", "Library/Logs", "Library/Developer"];

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
        if p.starts_with(self.under_home("Library"))
            && !LIBRARY_USER_ALLOWED
                .iter()
                .any(|rel| p.starts_with(self.under_home(rel)))
        {
            return Err(SafetyError::Protected(
                "~/Library (app data — use the cleaner instead)".into(),
            ));
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
}
