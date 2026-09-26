use std::fs::Metadata;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

/// Shared cancel flag for long scans.
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed)
    }
    pub fn reset(&self) {
        self.0.store(false, Ordering::Relaxed)
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Live counters a scan updates so the UI can show progress.
#[derive(Debug, Default)]
pub struct Progress {
    pub files: AtomicU64,
    pub bytes: AtomicU64,
}

impl Progress {
    pub fn add(&self, bytes: u64) {
        self.files.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(bytes, Ordering::Relaxed);
    }
    pub fn snapshot(&self) -> (u64, u64) {
        (self.files.load(Ordering::Relaxed), self.bytes.load(Ordering::Relaxed))
    }
    pub fn reset(&self) {
        self.files.store(0, Ordering::Relaxed);
        self.bytes.store(0, Ordering::Relaxed);
    }
}

/// Space a file actually takes on disk. Uses allocated blocks on Unix so
/// sparse files (Docker.raw) and iCloud placeholders aren't over-counted.
pub fn disk_bytes(meta: &Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        meta.blocks() * 512
    }
    #[cfg(not(unix))]
    {
        meta.len()
    }
}

/// Total on-disk size of a file or folder. Never follows symlinks.
pub fn path_size(path: &Path, cancel: Option<&Cancel>, progress: Option<&Progress>) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if !meta.is_dir() {
        return disk_bytes(&meta);
    }
    let mut total = disk_bytes(&meta);
    for entry in jwalk::WalkDir::new(path)
        .follow_links(false)
        .skip_hidden(false)
        .min_depth(1)
    {
        if cancel.is_some_and(|c| c.is_cancelled()) {
            break;
        }
        let Ok(entry) = entry else { continue };
        if let Ok(m) = entry.metadata() {
            let b = disk_bytes(&m);
            total += b;
            if let Some(p) = progress {
                p.add(b);
            }
        }
    }
    total
}

/// Latest modification time of a path (not recursive).
pub fn modified_secs(path: &Path) -> Option<i64> {
    let m = std::fs::symlink_metadata(path).ok()?.modified().ok()?;
    to_unix(m)
}

pub fn to_unix(t: SystemTime) -> Option<i64> {
    t.duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

pub fn now_secs() -> i64 {
    to_unix(SystemTime::now()).unwrap_or(0)
}
