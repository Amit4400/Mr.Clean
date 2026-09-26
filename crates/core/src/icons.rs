//! App icons, read from installed `.app` bundles (read-only, no network).
//!
//! macOS stores each app's icon inside the bundle (`Contents/Resources/*.icns`,
//! named by `CFBundleIconFile` in `Info.plist`). We convert it once to a small
//! PNG with the system's `sips` tool and cache it.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use base64::Engine;

use crate::env::Env;

/// The outermost `.app` bundle a path lives in:
/// `/Applications/Google Chrome.app/Contents/Frameworks/…Helper.app/…` → `/Applications/Google Chrome.app`.
pub fn bundle_of(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in path.components() {
        out.push(c);
        if c.as_os_str().to_string_lossy().ends_with(".app") {
            return Some(out);
        }
    }
    None
}

/// Look for an installed app by its bundle name ("Xcode", "Docker").
pub fn find_app(env: &Env, name: &str) -> Option<PathBuf> {
    let file = format!("{name}.app");
    [
        env.root.join("Applications"),
        env.home.join("Applications"),
        env.root.join("System/Applications"),
        env.root.join("System/Applications/Utilities"),
    ]
    .into_iter()
    .map(|d| d.join(&file))
    .find(|p| p.is_dir())
}

/// The `.icns` file a bundle declares as its icon.
pub fn icon_file(bundle: &Path) -> Option<PathBuf> {
    let resources = bundle.join("Contents/Resources");
    let declared = plist::Value::from_file(bundle.join("Contents/Info.plist"))
        .ok()
        .and_then(|v| {
            v.as_dictionary()?
                .get("CFBundleIconFile")?
                .as_string()
                .map(String::from)
        });
    let candidates = declared.into_iter().chain(["AppIcon".to_string()]);
    for name in candidates {
        let file = if name.ends_with(".icns") {
            name
        } else {
            format!("{name}.icns")
        };
        let p = resources.join(file);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn cache_key(icns: &Path) -> String {
    let mut h = DefaultHasher::new();
    icns.hash(&mut h);
    std::fs::metadata(icns).and_then(|m| m.modified()).ok().hash(&mut h);
    format!("{:016x}.png", h.finish())
}

/// PNG data URL for an app bundle's icon, or `None` if it has none or we're
/// not on macOS. Only paths ending in `.app` are accepted.
pub fn icon_data_url(bundle: &Path, cache_dir: &Path) -> Option<String> {
    if bundle.extension().is_none_or(|e| e != "app") || !bundle.is_dir() {
        return None;
    }
    let icns = icon_file(bundle)?;
    let png = cache_dir.join(cache_key(&icns));
    if !png.is_file() {
        if !cfg!(target_os = "macos") {
            return None;
        }
        std::fs::create_dir_all(cache_dir).ok()?;
        let ok = std::process::Command::new("/usr/bin/sips")
            .args(["-s", "format", "png", "-Z", "64"])
            .arg(&icns)
            .arg("--out")
            .arg(&png)
            .output()
            .is_ok_and(|o| o.status.success());
        if !ok {
            return None;
        }
    }
    let bytes = std::fs::read(&png).ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

pub fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("com.softradix.mrclean/icons")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Os;
    use std::fs;

    #[test]
    fn finds_outer_bundle() {
        let p = Path::new("/Applications/Google Chrome.app/Contents/Frameworks/Helper (Renderer).app/Contents/MacOS/x");
        assert_eq!(bundle_of(p), Some(PathBuf::from("/Applications/Google Chrome.app")));
        assert_eq!(bundle_of(Path::new("/usr/local/bin/node")), None);
    }

    #[test]
    fn reads_declared_icon_and_rejects_non_apps() {
        let d = tempfile::tempdir().unwrap();
        let env = Env::sandboxed(d.path(), Os::Mac);
        let app = env.root.join("Applications/Demo.app");
        fs::create_dir_all(app.join("Contents/Resources")).unwrap();
        fs::write(
            app.join("Contents/Info.plist"),
            r#"<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>CFBundleIconFile</key><string>Demo</string></dict></plist>"#,
        )
        .unwrap();
        fs::write(app.join("Contents/Resources/Demo.icns"), b"icns").unwrap();
        assert_eq!(find_app(&env, "Demo"), Some(app.clone()));
        assert_eq!(icon_file(&app), Some(app.join("Contents/Resources/Demo.icns")));
        assert_eq!(find_app(&env, "Missing"), None);
        assert_eq!(icon_data_url(&env.home, d.path()), None, "not an .app");
    }
}
