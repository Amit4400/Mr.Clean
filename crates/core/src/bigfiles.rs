//! Large-file & folder explorer: one parallel walk builds a folder-size index
//! (for drill-down) plus the biggest files and a by-type breakdown.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::env::Env;
use crate::fsutil::{disk_bytes, to_unix, Cancel, Progress};
use crate::safety::{remove_checked, DeleteMode, DeleteReport, Guard};

const TOP_FILES: usize = 200;
const MIN_TOP_FILE: u64 = 10 * 1024 * 1024;
/// Virtual or external mounts we never walk into.
const PRUNE_ABSOLUTE: &[&str] = &[
    "/Volumes",
    "/System/Volumes",
    "/dev",
    "/proc",
    "/sys",
    "/private/var/vm",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Video,
    Image,
    Audio,
    Archive,
    DiskImage,
    Installer,
    Document,
    Other,
}

pub fn kind_of(path: &Path) -> Kind {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "mp4" | "mov" | "mkv" | "avi" | "m4v" | "webm" | "wmv" | "flv" => Kind::Video,
        "jpg" | "jpeg" | "png" | "heic" | "gif" | "tiff" | "tif" | "psd" | "cr2" | "nef" | "dng" | "webp" => {
            Kind::Image
        }
        "mp3" | "wav" | "m4a" | "aac" | "flac" | "aiff" | "ogg" => Kind::Audio,
        "zip" | "rar" | "7z" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "zst" => Kind::Archive,
        "dmg" | "iso" | "img" | "vmdk" | "qcow2" | "vdi" | "vhd" | "vhdx" | "sparseimage" | "sparsebundle" => {
            Kind::DiskImage
        }
        "pkg" | "exe" | "msi" | "apk" | "aab" | "ipa" | "deb" | "rpm" | "appimage" | "xip" => Kind::Installer,
        "pdf" | "doc" | "docx" | "ppt" | "pptx" | "xls" | "xlsx" | "key" | "pages" | "numbers" | "sketch" | "fig" => {
            Kind::Document
        }
        _ => Kind::Other,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub bytes: u64,
    pub kind: Kind,
    pub modified: Option<i64>,
    pub accessed: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub path: String,
    pub name: String,
    pub bytes: u64,
    pub is_dir: bool,
    pub modified: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KindTotal {
    pub kind: Kind,
    pub bytes: u64,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub root: String,
    pub total_bytes: u64,
    pub file_count: u64,
    pub unreadable: u64,
    pub top_files: Vec<FileEntry>,
    pub kinds: Vec<KindTotal>,
}

#[derive(Debug, Default)]
pub struct BigScan {
    pub root: PathBuf,
    sizes: HashMap<PathBuf, u64>,
    children: HashMap<PathBuf, Vec<PathBuf>>,
    top: Vec<FileEntry>,
    kinds: HashMap<Kind, (u64, u64)>,
    file_count: u64,
    unreadable: u64,
}

fn file_entry(path: PathBuf, bytes: u64) -> FileEntry {
    let meta = std::fs::symlink_metadata(&path).ok();
    FileEntry {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        kind: kind_of(&path),
        modified: meta.as_ref().and_then(|m| m.modified().ok()).and_then(to_unix),
        accessed: meta.as_ref().and_then(|m| m.accessed().ok()).and_then(to_unix),
        path: path.display().to_string(),
        bytes,
    }
}

pub fn scan(root: &Path, cancel: &Cancel, progress: &Progress) -> BigScan {
    let root = root.to_path_buf();
    let mut own: HashMap<PathBuf, u64> = HashMap::new();
    let mut dirs: Vec<(usize, PathBuf)> = vec![(0, root.clone())];
    let mut heap: BinaryHeap<Reverse<(u64, PathBuf)>> = BinaryHeap::new();
    let mut kinds: HashMap<Kind, (u64, u64)> = HashMap::new();
    let mut file_count = 0u64;
    let mut unreadable = 0u64;

    let walker = jwalk::WalkDir::new(&root)
        .follow_links(false)
        .skip_hidden(false)
        .min_depth(1)
        .process_read_dir(|_, _, _, children| {
            for child in children.iter_mut().flatten() {
                let p = child.path();
                if PRUNE_ABSOLUTE.iter().any(|x| p == Path::new(x)) {
                    child.read_children = None;
                }
            }
        });

    for entry in walker {
        if cancel.is_cancelled() {
            break;
        }
        let e = match entry {
            Ok(e) => e,
            Err(_) => {
                unreadable += 1;
                continue;
            }
        };
        if e.read_children.as_ref().is_some_and(|rc| rc.error().is_some()) {
            unreadable += 1;
        }
        let path = e.path();
        if e.file_type().is_dir() {
            dirs.push((e.depth(), path));
            continue;
        }
        let Ok(meta) = e.metadata() else {
            unreadable += 1;
            continue;
        };
        let bytes = disk_bytes(&meta);
        progress.add(bytes);
        file_count += 1;
        *own.entry(e.parent_path().to_path_buf()).or_default() += bytes;
        let k = kinds.entry(kind_of(&path)).or_default();
        k.0 += bytes;
        k.1 += 1;
        if bytes >= MIN_TOP_FILE {
            heap.push(Reverse((bytes, path)));
            if heap.len() > TOP_FILES {
                heap.pop();
            }
        }
    }

    // Roll folder totals up, deepest first.
    dirs.sort_by_key(|d| std::cmp::Reverse(d.0));
    let mut sizes = own;
    let mut children: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for (_, d) in &dirs {
        sizes.entry(d.clone()).or_default();
    }
    for (depth, d) in &dirs {
        if *depth == 0 {
            continue;
        }
        let Some(parent) = d.parent() else { continue };
        let s = sizes.get(d).copied().unwrap_or(0);
        *sizes.entry(parent.to_path_buf()).or_default() += s;
        children.entry(parent.to_path_buf()).or_default().push(d.clone());
    }

    let mut top: Vec<FileEntry> = heap.into_iter().map(|Reverse((b, p))| file_entry(p, b)).collect();
    top.sort_by_key(|a| std::cmp::Reverse(a.bytes));
    BigScan {
        root,
        sizes,
        children,
        top,
        kinds,
        file_count,
        unreadable,
    }
}

impl BigScan {
    pub fn summary(&self) -> Summary {
        let mut kinds: Vec<KindTotal> = self
            .kinds
            .iter()
            .map(|(k, (b, c))| KindTotal {
                kind: *k,
                bytes: *b,
                count: *c,
            })
            .collect();
        kinds.sort_by_key(|a| std::cmp::Reverse(a.bytes));
        Summary {
            root: self.root.display().to_string(),
            total_bytes: self.sizes.get(&self.root).copied().unwrap_or(0),
            file_count: self.file_count,
            unreadable: self.unreadable,
            top_files: self.top.clone(),
            kinds,
        }
    }

    /// Folders (from the index) and files (read live) directly inside `dir`.
    pub fn children(&self, dir: &Path) -> Vec<Node> {
        let mut out: Vec<Node> = self
            .children
            .get(dir)
            .into_iter()
            .flatten()
            .map(|d| Node {
                name: d
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                bytes: self.sizes.get(d).copied().unwrap_or(0),
                is_dir: true,
                modified: crate::fsutil::modified_secs(d),
                path: d.display().to_string(),
            })
            .collect();
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let Ok(m) = e.metadata() else { continue };
                if m.is_dir() {
                    continue;
                }
                out.push(Node {
                    name: e.file_name().to_string_lossy().to_string(),
                    bytes: disk_bytes(&m),
                    is_dir: false,
                    modified: m.modified().ok().and_then(to_unix),
                    path: e.path().display().to_string(),
                });
            }
        }
        out.sort_by_key(|a| std::cmp::Reverse(a.bytes));
        out
    }

    /// Update the index after a path was deleted.
    pub fn forget(&mut self, path: &Path, bytes: u64) {
        let mut cur = path.parent();
        while let Some(p) = cur {
            if let Some(s) = self.sizes.get_mut(p) {
                *s = s.saturating_sub(bytes);
            }
            if p == self.root {
                break;
            }
            cur = p.parent();
        }
        self.sizes.retain(|k, _| !k.starts_with(path));
        if let Some(parent) = path.parent() {
            if let Some(list) = self.children.get_mut(parent) {
                list.retain(|c| c != path);
            }
        }
        self.top.retain(|f| !Path::new(&f.path).starts_with(path));
    }
}

/// Delete files/folders the user picked. Each one passes the user-file guard.
pub fn remove(env: &Env, paths: &[String], mode: DeleteMode) -> DeleteReport {
    let guard = Guard::new(env);
    let mut report = DeleteReport::default();
    for p in paths.iter().map(PathBuf::from) {
        match guard.check_user_file(&p) {
            Ok(checked) => remove_checked(&checked, mode, &mut report),
            Err(e) => report.fail(&p, e),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Os;
    use std::fs;

    #[test]
    fn indexes_folders_and_top_files() {
        let d = tempfile::tempdir().unwrap();
        let env = Env::sandboxed(d.path(), Os::Mac);
        let h = &env.home;
        let big = h.join("Downloads/Xcode_15.xip");
        fs::create_dir_all(big.parent().unwrap()).unwrap();
        fs::write(&big, vec![7u8; 12 * 1024 * 1024]).unwrap();
        fs::create_dir_all(h.join("Documents/a/b")).unwrap();
        fs::write(h.join("Documents/a/b/note.txt"), vec![1u8; 8192]).unwrap();

        let mut s = scan(h, &Cancel::new(), &Progress::default());
        let sum = s.summary();
        assert_eq!(sum.file_count, 2);
        assert_eq!(sum.top_files.len(), 1);
        assert_eq!(sum.top_files[0].kind, Kind::Installer);
        let top = s.children(h);
        assert_eq!(top[0].name, "Downloads");
        assert!(top[0].bytes >= 12 * 1024 * 1024);
        let docs = s.children(&h.join("Documents"));
        assert!(docs[0].bytes >= 8192, "nested sizes roll up");

        let before = sum.total_bytes;
        let r = remove(&env, &[big.display().to_string()], DeleteMode::Permanent);
        assert_eq!(r.removed.len(), 1);
        s.forget(&big, r.bytes_freed);
        assert!(s.summary().total_bytes < before);
        assert!(s.summary().top_files.is_empty());
    }

    #[test]
    fn kinds() {
        assert_eq!(kind_of(Path::new("a/Movie.MOV")), Kind::Video);
        assert_eq!(kind_of(Path::new("x.dmg")), Kind::DiskImage);
        assert_eq!(kind_of(Path::new("noext")), Kind::Other);
    }
}
