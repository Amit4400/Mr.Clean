//! "What is this folder?" Plain-English explanations for folders people find
//! in the large-file explorer, with a hint about whether deleting is wise.
//! Read-only: this never changes anything.

use std::path::Path;

use serde::Serialize;

use crate::env::Env;
use crate::icons;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Advice {
    /// Rebuilt or re-downloaded automatically.
    SafeToDelete,
    /// Belongs to an app; remove it from inside the app (or uninstall the app).
    UseTheApp,
    /// Needed by macOS or apps; don't touch.
    LeaveIt,
    /// The user's own files.
    Yours,
    /// Unknown or leftover; look inside before deleting.
    CheckFirst,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Explanation {
    pub title: String,
    pub text: String,
    pub advice: Advice,
}

fn ex(title: impl Into<String>, text: impl Into<String>, advice: Advice) -> Option<Explanation> {
    Some(Explanation {
        title: title.into(),
        text: text.into(),
        advice,
    })
}

use Advice::*;

/// Known locations, relative to the home folder.
const KNOWN: &[(&str, &str, &str, Advice)] = &[
    ("Library", "Your Library", "Settings, caches and data for your apps and macOS. Never delete the folder itself; the Clean page clears the safe parts inside it.", LeaveIt),
    ("Library/Caches", "App caches", "Temporary files apps rebuild when needed. The Clean page clears them safely.", SafeToDelete),
    ("Library/Logs", "Logs", "Diagnostic and crash logs. Safe to clear.", SafeToDelete),
    ("Library/Application Support", "App data", "Apps' own data: databases, downloads and settings. Deleting parts of it can reset an app or sign you out. Pick a folder inside to see which app it belongs to.", UseTheApp),
    ("Library/Containers", "Sandboxed app data", "Data of apps that run in Apple's sandbox (Mail, Notes, many App Store apps). Remove it by deleting the app or from inside the app.", UseTheApp),
    ("Library/Group Containers", "Shared app data", "Data an app shares with its widgets and extensions (for example Office, WhatsApp, Notes). Manage it from the app.", UseTheApp),
    ("Library/Mobile Documents", "iCloud Drive", "Your iCloud Drive files. Deleting here deletes them from iCloud on all your devices.", Yours),
    ("Library/CloudStorage", "Cloud drives", "Google Drive, Dropbox, OneDrive and other synced folders. Deleting here deletes them in the cloud too.", Yours),
    ("Library/Developer", "Developer data", "Xcode build data, simulators and device support files. The Clean page removes the safe parts.", SafeToDelete),
    ("Library/Developer/CoreSimulator", "iOS simulators", "Simulators and the apps installed in them. Remove old ones from the Clean page.", SafeToDelete),
    ("Library/Developer/Xcode/DerivedData", "Xcode build data", "Build products and indexes. Xcode rebuilds them on the next build.", SafeToDelete),
    ("Library/Mail", "Mail", "Your email and attachments downloaded by Mail. Manage it in Mail.", Yours),
    ("Library/Messages", "Messages", "Your iMessage history and attachments. Manage it in Messages → Settings.", Yours),
    ("Library/Preferences", "Preferences", "Settings for every app. Needed; deleting resets apps.", LeaveIt),
    ("Library/Photos", "Photos data", "Data used by the Photos app.", LeaveIt),
    ("Library/Keychains", "Keychains", "Your saved passwords and certificates. Never delete.", LeaveIt),
    ("Library/Saved Application State", "Window state", "Remembers which windows apps had open. Small and safe to clear.", SafeToDelete),
    ("Downloads", "Downloads", "Files you downloaded. Often full of old installers (.dmg, .pkg, .zip) you no longer need.", Yours),
    ("Documents", "Documents", "Your documents. Check before deleting anything here.", Yours),
    ("Desktop", "Desktop", "Files on your desktop.", Yours),
    ("Movies", "Movies", "Your videos and screen recordings, often large.", Yours),
    ("Music", "Music", "Your music library.", Yours),
    ("Pictures", "Pictures", "Your photos, including the Photos library. Manage it in Photos.", Yours),
    (".Trash", "Trash", "Files you already deleted. Empty the Trash to get the space back.", SafeToDelete),
    (".npm", "npm cache", "npm's download cache. npm downloads what it needs again.", SafeToDelete),
    (".yarn", "Yarn data", "Yarn's cache and settings. Yarn downloads what it needs again.", SafeToDelete),
    (".gradle", "Gradle", "Gradle's downloaded dependencies and build caches. Rebuilt on the next build.", SafeToDelete),
    (".m2", "Maven repository", "Java libraries downloaded by Maven. Downloaded again when needed.", SafeToDelete),
    (".cocoapods", "CocoaPods", "CocoaPods' specs and cache. Downloaded again when needed.", SafeToDelete),
    (".cache", "Tool caches", "Caches of command-line tools (pip, Hugging Face models, pre-commit…). Usually safe to clear.", SafeToDelete),
    (".android", "Android tools", "Android emulators (AVDs) and SDK settings. Remove old emulators from Android Studio's Device Manager.", UseTheApp),
    (".cursor", "Cursor", "The Cursor editor's extensions and settings. Remove extensions from inside Cursor.", UseTheApp),
    (".vscode", "VS Code", "VS Code's extensions and settings. Remove extensions from inside VS Code.", UseTheApp),
    (".gemini", "Gemini CLI", "Settings and history of Google's Gemini command-line tool. Created when you (or a tool) used Gemini in the Terminal.", UseTheApp),
    (".claude", "Claude Code", "Settings and history of Claude Code.", UseTheApp),
    (".nvm", "Node versions (nvm)", "Node.js versions installed with nvm. Remove old ones with `nvm uninstall <version>`.", UseTheApp),
    (".pyenv", "Python versions (pyenv)", "Python versions installed with pyenv. Remove old ones with `pyenv uninstall`.", UseTheApp),
    (".cargo", "Rust (cargo)", "Rust tools and downloaded crates. The Clean page clears the caches inside.", UseTheApp),
    (".rustup", "Rust toolchains", "Installed Rust compilers. Remove old ones with `rustup toolchain uninstall`.", UseTheApp),
    (".docker", "Docker settings", "Docker command-line settings. Docker's images live in its own disk image.", LeaveIt),
    (".ssh", "SSH keys", "Your SSH keys for GitHub and servers. Never delete.", LeaveIt),
    (".config", "Tool settings", "Settings for command-line tools.", LeaveIt),
    (".local", "Tool data", "Programs and data installed by command-line tools.", CheckFirst),
];

/// `com.apple.foo`, `group.com.x`, `UBF8T346G9.Office`: looks like a bundle ID.
fn looks_like_bundle_id(name: &str) -> bool {
    name.matches('.').count() >= 1
        && !name.contains(' ')
        && !name.starts_with('.')
        && name.chars().any(|c| c.is_ascii_alphabetic())
}

/// The installed app a folder name points to, by bundle ID or plain name.
fn app_for(env: &Env, name: &str) -> Option<String> {
    let id = name.strip_prefix("group.").unwrap_or(name);
    if looks_like_bundle_id(id) {
        if let Some(app) = icons::app_for_bundle_id(env, id) {
            return Some(app);
        }
    }
    icons::find_app(env, name).map(|_| name.to_string())
}

fn is_ai_folder(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("intelligence") || n.contains("generativemodels") || n.contains("modelcatalog") || n.contains("siri")
}

/// Explain `path`, if we know or can work out what it is.
pub fn explain(env: &Env, path: &Path) -> Option<Explanation> {
    let name = path.file_name()?.to_string_lossy().to_string();
    let rel = path
        .strip_prefix(&env.home)
        .ok()
        .map(|r| r.to_string_lossy().replace('\\', "/"));

    if let Some(rel) = rel.as_deref() {
        if let Some((_, title, text, advice)) = KNOWN.iter().find(|(k, ..)| *k == rel) {
            return ex(*title, *text, *advice);
        }
    }
    if is_ai_folder(&name) && rel.as_deref().is_some_and(|r| r.starts_with("Library/")) {
        return ex(
            "Apple Intelligence and on-device AI",
            "Models and caches for Apple Intelligence, Siri and other on-device features. macOS downloads and manages these itself, even if you've never turned Apple Intelligence on, so deleting them may not stick. To stop the downloads, turn Apple Intelligence off in System Settings.",
            LeaveIt,
        );
    }
    if name == "node_modules" {
        return ex("Project packages", "JavaScript packages for this project. `npm install` brings them back, so old projects' copies are safe to delete.", SafeToDelete);
    }
    if name == ".git" {
        return ex(
            "Git history",
            "The full history of this repository. Deleting it loses every commit that isn't pushed.",
            LeaveIt,
        );
    }

    // A folder inside a well-known app-data location: say which app it belongs to.
    let parent_rel = rel.as_deref().map(|r| r.rsplit_once('/').map_or("", |(p, _)| p));
    let app = || app_for(env, &name);
    match parent_rel {
        Some("Library/Caches") => {
            let who = app().unwrap_or_else(|| name.clone());
            ex(format!("Cache of {who}"), format!("Temporary files {who} rebuilds when needed. Safe to clear; quit the app first."), SafeToDelete)
        }
        Some("Library/Logs") => ex(format!("Logs of {}", app().unwrap_or_else(|| name.clone())), "Diagnostic logs. Safe to clear.", SafeToDelete),
        Some("Library/Application Support" | "Library/Containers" | "Library/Group Containers") => Some(match app() {
            Some(a) => Explanation {
                title: format!("Data for {a}"),
                text: format!("Settings, downloads and databases {a} keeps. Deleting it can reset {a} or sign you out, so clear it from inside the app."),
                advice: UseTheApp,
            },
            None => Explanation {
                title: format!("Data for {name}"),
                text: format!("App data for \"{name}\". No installed app matches it, so it may be left over from an app you removed. Look inside before deleting."),
                advice: CheckFirst,
            },
        }),
        Some("") if name.starts_with('.') => ex(
            format!("{name} (tool settings)"),
            "A hidden folder a command-line tool or app created for its settings and data. Check which tool uses it before deleting.",
            CheckFirst,
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::Os;
    use std::fs;

    fn env() -> (tempfile::TempDir, Env) {
        let d = tempfile::tempdir().unwrap();
        let env = Env::sandboxed(d.path(), Os::Mac);
        let app = env.root.join("Applications/Slack.app/Contents");
        fs::create_dir_all(&app).unwrap();
        fs::write(
            app.join("Info.plist"),
            r#"<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>com.tinyspeck.slackmacgap</string></dict></plist>"#,
        )
        .unwrap();
        (d, env)
    }

    #[test]
    fn known_folders() {
        let (_d, env) = env();
        let h = &env.home;
        assert_eq!(explain(&env, &h.join("Library/Caches")).unwrap().advice, SafeToDelete);
        assert_eq!(
            explain(&env, &h.join("Library/Mobile Documents")).unwrap().advice,
            Yours
        );
        assert_eq!(explain(&env, &h.join(".gemini")).unwrap().title, "Gemini CLI");
        assert_eq!(explain(&env, &h.join("Library/Keychains")).unwrap().advice, LeaveIt);
    }

    #[test]
    fn apple_intelligence_is_explained() {
        let (_d, env) = env();
        let e = explain(
            &env,
            &env.home
                .join("Library/Application Support/com.apple.intelligenceplatform"),
        )
        .unwrap();
        assert!(e.title.contains("Apple Intelligence"));
        assert_eq!(e.advice, LeaveIt);
        let e = explain(&env, &env.home.join("Library/Caches/com.apple.GenerativeModels")).unwrap();
        assert!(e.title.contains("Apple Intelligence"));
    }

    #[test]
    fn app_data_names_the_app() {
        let (_d, env) = env();
        let h = &env.home;
        let e = explain(&env, &h.join("Library/Containers/com.tinyspeck.slackmacgap")).unwrap();
        assert_eq!(e.title, "Data for Slack");
        assert_eq!(e.advice, UseTheApp);
        let e = explain(&env, &h.join("Library/Caches/com.tinyspeck.slackmacgap")).unwrap();
        assert_eq!(e.title, "Cache of Slack");
        let e = explain(&env, &h.join("Library/Application Support/Slack")).unwrap();
        assert_eq!(e.title, "Data for Slack");
        let e = explain(&env, &h.join("Library/Application Support/com.gone.app")).unwrap();
        assert_eq!(e.advice, CheckFirst, "no installed app → may be leftover");
    }

    #[test]
    fn other_folders() {
        let (_d, env) = env();
        let h = &env.home;
        assert_eq!(
            explain(&env, &h.join("code/web/node_modules")).unwrap().advice,
            SafeToDelete
        );
        assert_eq!(explain(&env, &h.join(".someweirdtool")).unwrap().advice, CheckFirst);
        assert!(explain(&env, &h.join("projects/shop")).is_none());
    }
}
