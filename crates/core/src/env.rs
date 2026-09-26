use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Mac,
    Linux,
    Windows,
}

impl Os {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Os::Mac
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else {
            Os::Linux
        }
    }
}

/// Where things live on this machine. Tests build one pointing at a temp dir,
/// so no test ever touches the real home folder.
#[derive(Debug, Clone)]
pub struct Env {
    pub home: PathBuf,
    /// Filesystem root used for absolute rule paths ("/" in real use).
    pub root: PathBuf,
    pub os: Os,
    pub local_app_data: Option<PathBuf>,
    pub app_data: Option<PathBuf>,
}

impl Env {
    pub fn detect() -> Self {
        let os = Os::current();
        Env {
            home: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
            root: PathBuf::from("/"),
            os,
            local_app_data: if os == Os::Windows { dirs::data_local_dir() } else { None },
            app_data: if os == Os::Windows { dirs::data_dir() } else { None },
        }
    }

    /// An environment rooted in `base`: home is `base/home`, "/" is `base/root`.
    pub fn sandboxed(base: &Path, os: Os) -> Self {
        Env {
            home: base.join("home"),
            root: base.join("root"),
            os,
            local_app_data: Some(base.join("home/AppData/Local")),
            app_data: Some(base.join("home/AppData/Roaming")),
        }
    }

    /// Expand a rule path: `~/x`, `/x` (under `root`), `%LOCALAPPDATA%/x`, `%APPDATA%/x`.
    pub fn expand(&self, p: &str) -> Option<PathBuf> {
        let join = |base: &Path, rest: &str| {
            let mut out = base.to_path_buf();
            for part in rest.split(['/', '\\']).filter(|s| !s.is_empty()) {
                out.push(part);
            }
            out
        };
        if let Some(rest) = p.strip_prefix("~") {
            Some(join(&self.home, rest))
        } else if let Some(rest) = p.strip_prefix("%LOCALAPPDATA%") {
            self.local_app_data.as_deref().map(|b| join(b, rest))
        } else if let Some(rest) = p.strip_prefix("%APPDATA%") {
            self.app_data.as_deref().map(|b| join(b, rest))
        } else if p.starts_with('/') {
            Some(join(&self.root, p))
        } else {
            None
        }
    }
}
