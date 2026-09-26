//! Read-only helpers for Xcode simulator runtimes: which files are runtimes,
//! where each one is mounted, and a friendly name ("iOS 17.5 (21F79)").
//! Removing them lives in `safety.rs`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Root-owned folders where CoreSimulator keeps downloaded runtimes.
pub const IMAGES: &str = "/Library/Developer/CoreSimulator/Images";
pub const BUNDLES: &str = "/Library/Developer/CoreSimulator/Profiles/Runtimes";

/// `<UUID>.dmg`, the way CoreSimulator names downloaded runtime images.
pub fn is_runtime_image_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".dmg") else {
        return false;
    };
    let parts: Vec<&str> = stem.split('-').collect();
    parts.iter().map(|p| p.len()).eq([8, 4, 4, 4, 12]) && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_hexdigit()))
}

pub fn is_runtime_bundle_name(name: &str) -> bool {
    name.ends_with(".simruntime") && name.len() > ".simruntime".len()
}

/// Whether a scanned item is a runtime (skips `images.plist` and other bookkeeping).
pub fn is_runtime_item(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    is_runtime_image_name(&name) || is_runtime_bundle_name(&name)
}

/// Pairs of (disk image, mount point) from `hdiutil info -plist` output.
pub fn parse_hdiutil_info(xml: &[u8]) -> Vec<(PathBuf, PathBuf)> {
    let Ok(plist::Value::Dictionary(root)) = plist::from_bytes::<plist::Value>(xml) else {
        return vec![];
    };
    let Some(images) = root.get("images").and_then(|v| v.as_array()) else {
        return vec![];
    };
    let mut out = Vec::new();
    for img in images.iter().filter_map(|v| v.as_dictionary()) {
        let Some(image) = img.get("image-path").and_then(|v| v.as_string()) else {
            continue;
        };
        let entities = img.get("system-entities").and_then(|v| v.as_array());
        for e in entities.into_iter().flatten().filter_map(|v| v.as_dictionary()) {
            if let Some(mount) = e.get("mount-point").and_then(|v| v.as_string()) {
                out.push((PathBuf::from(image), PathBuf::from(mount)));
            }
        }
    }
    out
}

/// Current disk image mounts (macOS only; empty elsewhere or on error).
pub fn mounts() -> Vec<(PathBuf, PathBuf)> {
    if !cfg!(target_os = "macos") {
        return vec![];
    }
    Command::new("/usr/bin/hdiutil")
        .args(["info", "-plist"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| parse_hdiutil_info(&o.stdout))
        .unwrap_or_default()
}

pub fn mount_of<'a>(image: &Path, mounts: &'a [(PathBuf, PathBuf)]) -> Option<&'a Path> {
    mounts.iter().find(|(i, _)| i == image).map(|(_, m)| m.as_path())
}

/// Human name for a runtime item, e.g. "iOS 17.5 (21F79)".
pub fn label(path: &Path, mounts: &[(PathBuf, PathBuf)]) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if let Some(stem) = name.strip_suffix(".simruntime") {
        return stem.to_string();
    }
    let short = name.split('-').next().unwrap_or(&name).to_string();
    let Some(mount) = mount_of(path, mounts) else {
        return format!("Simulator runtime {short}…");
    };
    // The image holds one bundle, e.g. "iOS 17.5.simruntime", inside its runtimes folder.
    let inner = mount.join("Library/Developer/CoreSimulator/Profiles/Runtimes");
    let version = std::fs::read_dir(&inner)
        .ok()
        .and_then(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .find(|n| is_runtime_bundle_name(n))
        })
        .map(|n| n.trim_end_matches(".simruntime").to_string());
    // Volumes are named like "iOS_21F79".
    let build = mount.file_name().map(|n| n.to_string_lossy().replace('_', " "));
    match (version, build) {
        (Some(v), Some(b)) => match b.rsplit(' ').next() {
            Some(code) if code != v => format!("{v} ({code})"),
            _ => v,
        },
        (Some(v), None) => v,
        (None, Some(b)) => b,
        (None, None) => format!("Simulator runtime {short}…"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_runtime_names() {
        assert!(is_runtime_image_name("7D832126-03E7-45A9-86CF-E1E9A1B2C3D4.dmg"));
        assert!(!is_runtime_image_name("images.plist"));
        assert!(!is_runtime_image_name("notes.dmg"));
        assert!(!is_runtime_image_name("7D832126-03E7-45A9-86CF-E1E9A1B2C3D4.dmg.bak"));
        assert!(is_runtime_bundle_name("iOS 17.5.simruntime"));
        assert!(!is_runtime_bundle_name(".simruntime"));
        assert!(is_runtime_item(Path::new("/x/iOS 16.4.simruntime")));
        assert!(!is_runtime_item(Path::new("/x/images.plist")));
    }

    #[test]
    fn parses_hdiutil_mounts_and_labels() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict><key>images</key><array>
  <dict>
    <key>image-path</key><string>/Library/Developer/CoreSimulator/Images/7D832126-03E7-45A9-86CF-E1E9A1B2C3D4.dmg</string>
    <key>system-entities</key><array>
      <dict><key>dev-entry</key><string>/dev/disk5</string></dict>
      <dict><key>dev-entry</key><string>/dev/disk5s1</string><key>mount-point</key><string>/Library/Developer/CoreSimulator/Volumes/iOS_21F79</string></dict>
    </array>
  </dict>
</array></dict></plist>"#;
        let m = parse_hdiutil_info(xml);
        assert_eq!(m.len(), 1);
        assert!(m[0].1.ends_with("iOS_21F79"));
        assert!(parse_hdiutil_info(b"not a plist").is_empty());

        let d = tempfile::tempdir().unwrap();
        let mount = d.path().join("iOS_21F79");
        std::fs::create_dir_all(mount.join("Library/Developer/CoreSimulator/Profiles/Runtimes/iOS 17.5.simruntime"))
            .unwrap();
        let img = PathBuf::from("/imgs/7D832126-03E7-45A9-86CF-E1E9A1B2C3D4.dmg");
        let mounts = vec![(img.clone(), mount)];
        assert_eq!(label(&img, &mounts), "iOS 17.5 (21F79)");
        assert_eq!(label(&img, &[]), "Simulator runtime 7D832126…");
        assert_eq!(label(Path::new("/r/watchOS 10.2.simruntime"), &[]), "watchOS 10.2");
    }
}
