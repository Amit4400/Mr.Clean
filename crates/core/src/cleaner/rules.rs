//! The catalog of known junk locations. Adding a cache = adding an entry here.

use serde::Serialize;

use crate::env::Os;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Xcode,
    Android,
    JavaScript,
    Languages,
    Tools,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Safety {
    /// Rebuilt automatically when needed. Pre-selected.
    Safe,
    /// Removes something the user may want (emulators, archives). Never pre-selected.
    Review,
    /// We only measure it; freeing it needs a tool command or admin rights.
    ReportOnly,
    /// System-owned (simulator runtimes): removable, but macOS asks for the
    /// admin password. Never pre-selected; always permanent.
    NeedsPassword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Remove the folder itself.
    Dir(&'static str),
    /// Remove each entry inside the folder, keeping the folder.
    Contents(&'static str),
    /// Remove the entries inside the folder whose name starts with the prefix
    /// (e.g. only `AndroidStudio*` inside `~/Library/Caches/Google`).
    Prefixed(&'static str, &'static str),
}

impl Target {
    pub fn raw(&self) -> &'static str {
        match self {
            Target::Dir(p) | Target::Contents(p) | Target::Prefixed(p, _) => p,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: &'static str,
    pub name: &'static str,
    pub category: Category,
    pub description: &'static str,
    pub safety: Safety,
    pub os: &'static [Os],
    pub targets: &'static [Target],
    /// A command that frees this space properly (shown in the UI).
    pub command: Option<&'static str>,
    /// Emptying the Trash can't go to the Trash.
    pub always_permanent: bool,
}

const MAC: &[Os] = &[Os::Mac];
const LINUX: &[Os] = &[Os::Linux];
const WIN: &[Os] = &[Os::Windows];
const UNIX: &[Os] = &[Os::Mac, Os::Linux];
const ALL: &[Os] = &[Os::Mac, Os::Linux, Os::Windows];

use Category::*;
use Safety::*;
use Target::*;

macro_rules! rule {
    ($id:expr, $name:expr, $cat:expr, $safety:expr, $os:expr, [$($t:expr),+ $(,)?], $desc:expr $(, cmd = $cmd:expr)? $(, permanent = $perm:expr)?) => {
        Rule {
            id: $id,
            name: $name,
            category: $cat,
            description: $desc,
            safety: $safety,
            os: $os,
            targets: &[$($t),+],
            command: { #[allow(unused_mut, unused_assignments)] let mut c = None; $(c = Some($cmd);)? c },
            always_permanent: { #[allow(unused_mut, unused_assignments)] let mut p = false; $(p = $perm;)? p },
        }
    };
}

pub fn catalog() -> Vec<Rule> {
    vec![
        // ---------------------------------------------------------------- Xcode
        rule!("xcode-derived-data", "Xcode DerivedData", Xcode, Safe, MAC,
            [Contents("~/Library/Developer/Xcode/DerivedData")],
            "Build products and indexes. Xcode rebuilds them on the next build."),
        rule!("xcode-device-support", "iOS device support files", Xcode, Safe, MAC,
            [Contents("~/Library/Developer/Xcode/iOS DeviceSupport"),
             Contents("~/Library/Developer/Xcode/watchOS DeviceSupport"),
             Contents("~/Library/Developer/Xcode/tvOS DeviceSupport"),
             Contents("~/Library/Developer/Xcode/visionOS DeviceSupport")],
            "Debug symbols copied from every phone you've plugged in. Re-copied when you connect a device again."),
        rule!("xcode-previews", "SwiftUI preview data", Xcode, Safe, MAC,
            [Contents("~/Library/Developer/Xcode/UserData/Previews")],
            "Cached SwiftUI preview simulators and builds."),
        rule!("xcode-caches", "Xcode caches", Xcode, Safe, MAC,
            [Dir("~/Library/Caches/com.apple.dt.Xcode"),
             Contents("~/Library/Developer/Xcode/DocumentationCache")],
            "Xcode's own caches and downloaded documentation."),
        rule!("xcode-archives", "Xcode archives", Xcode, Review, MAC,
            [Contents("~/Library/Developer/Xcode/Archives")],
            "Old app builds you archived for the App Store. Keep any you may need to re-upload or symbolicate crashes for."),
        rule!("simulator-devices", "iOS simulators", Xcode, Review, MAC,
            [Contents("~/Library/Developer/CoreSimulator/Devices")],
            "Every simulator and the apps/data installed in it. Often huge and forgotten, especially after Xcode is removed. Xcode recreates default simulators when needed.",
            cmd = "xcrun simctl delete unavailable"),
        rule!("simulator-caches", "Simulator caches", Xcode, Safe, MAC,
            [Contents("~/Library/Developer/CoreSimulator/Caches")],
            "Dyld and other caches the simulator rebuilds."),
        // Volumes/ holds mounts of the Images/ disk images, so it isn't counted.
        rule!("simulator-runtimes", "Simulator runtimes (iOS versions)", Xcode, NeedsPassword, MAC,
            [Contents("/Library/Developer/CoreSimulator/Images"),
             Contents("/Library/Developer/CoreSimulator/Profiles/Runtimes")],
            "Downloaded iOS/watchOS runtimes, several GB each. They belong to macOS, so it asks for your password to remove them. Xcode downloads one again if you need it.",
            permanent = true),
        // -------------------------------------------------------------- Android
        rule!("gradle-caches", "Gradle caches", Android, Safe, ALL,
            [Dir("~/.gradle/caches")],
            "Downloaded dependencies and build caches. Gradle re-downloads what a project needs."),
        rule!("gradle-wrapper", "Gradle wrapper distributions", Android, Safe, ALL,
            [Contents("~/.gradle/wrapper/dists")],
            "Every Gradle version any project ever used (~150 MB each)."),
        rule!("gradle-daemon", "Gradle daemon logs", Android, Safe, ALL,
            [Contents("~/.gradle/daemon"), Contents("~/.gradle/native")],
            "Logs and native libraries from old Gradle daemons."),
        rule!("android-cache", "Android tool caches", Android, Safe, ALL,
            [Contents("~/.android/cache"), Contents("~/.android/build-cache")],
            "SDK manager and build caches."),
        rule!("android-avd", "Android emulators (AVDs)", Android, Review, ALL,
            [Contents("~/.android/avd")],
            "Emulator disk images, 2–10 GB each. Delete the ones you no longer use; recreate them in Android Studio."),
        rule!("android-system-images", "Android emulator system images", Android, Review, MAC,
            [Contents("~/Library/Android/sdk/system-images")],
            "Android OS images for emulators. Re-downloadable from the SDK Manager."),
        rule!("android-system-images-linux", "Android emulator system images", Android, Review, LINUX,
            [Contents("~/Android/Sdk/system-images")],
            "Android OS images for emulators. Re-downloadable from the SDK Manager."),
        rule!("kotlin-daemon", "Kotlin daemon files", Android, Safe, MAC,
            [Contents("~/Library/Application Support/kotlin/daemon")],
            "Logs and state from Kotlin compile daemons."),
        rule!("maven", "Maven repository", Android, Safe, ALL,
            [Dir("~/.m2/repository")],
            "Downloaded Java libraries. Re-downloaded on the next build."),
        // ------------------------------------------------------------ JavaScript
        rule!("npm-cache", "npm cache", JavaScript, Safe, UNIX,
            [Dir("~/.npm/_cacache"), Contents("~/.npm/_logs"), Contents("~/.npm/_npx")],
            "npm's download cache, logs and npx packages."),
        rule!("npm-cache-win", "npm cache", JavaScript, Safe, WIN,
            [Dir("%LOCALAPPDATA%/npm-cache/_cacache"), Contents("%LOCALAPPDATA%/npm-cache/_logs")],
            "npm's download cache and logs."),
        rule!("yarn-cache", "Yarn cache", JavaScript, Safe, ALL,
            [Dir("~/Library/Caches/Yarn"), Dir("~/.cache/yarn"), Dir("~/.yarn/berry/cache"),
             Dir("%LOCALAPPDATA%/Yarn/Cache")],
            "Yarn's package cache."),
        rule!("pnpm-store", "pnpm store", JavaScript, Safe, ALL,
            [Dir("~/Library/pnpm/store"), Dir("~/.local/share/pnpm/store"), Dir("~/.pnpm-store"),
             Dir("%LOCALAPPDATA%/pnpm/store")],
            "pnpm's shared package store. Projects re-link on the next install."),
        rule!("bun-cache", "Bun cache", JavaScript, Safe, ALL,
            [Dir("~/.bun/install/cache")],
            "Bun's package cache."),
        rule!("node-gyp", "node-gyp headers", JavaScript, Safe, UNIX,
            [Dir("~/Library/Caches/node-gyp"), Dir("~/.cache/node-gyp")],
            "Node.js headers used to compile native modules."),
        rule!("browser-test-tools", "Playwright / Puppeteer / Cypress browsers", JavaScript, Safe, ALL,
            [Dir("~/Library/Caches/ms-playwright"), Dir("~/.cache/ms-playwright"),
             Dir("~/.cache/puppeteer"), Dir("~/Library/Caches/Cypress"), Dir("~/.cache/Cypress"),
             Dir("%LOCALAPPDATA%/ms-playwright")],
            "Test browsers (hundreds of MB each). Re-downloaded by the next test run."),
        rule!("electron-cache", "Electron downloads", JavaScript, Safe, UNIX,
            [Dir("~/Library/Caches/electron"), Dir("~/.cache/electron")],
            "Electron binaries downloaded by projects."),
        // -------------------------------------------------- iOS deps & languages
        rule!("cocoapods", "CocoaPods cache", Languages, Safe, MAC,
            [Dir("~/Library/Caches/CocoaPods")],
            "Downloaded pods. `pod install` re-downloads what it needs."),
        rule!("swiftpm", "Swift Package Manager cache", Languages, Safe, MAC,
            [Dir("~/Library/Caches/org.swift.swiftpm"), Dir("~/Library/Caches/org.carthage.CarthageKit")],
            "SwiftPM and Carthage package caches."),
        rule!("flutter-pub", "Flutter / Dart pub cache", Languages, Review, UNIX,
            [Dir("~/.pub-cache")],
            "Dart packages. Run `flutter pub get` in your projects afterwards."),
        rule!("pip", "pip cache", Languages, Safe, ALL,
            [Dir("~/Library/Caches/pip"), Dir("~/.cache/pip"), Dir("%LOCALAPPDATA%/pip/Cache")],
            "Python package downloads."),
        rule!("cargo", "Rust cargo caches", Languages, Safe, ALL,
            [Dir("~/.cargo/registry/cache"), Dir("~/.cargo/registry/src"), Dir("~/.cargo/git/checkouts")],
            "Downloaded crates. Cargo re-downloads them on the next build."),
        rule!("go-build", "Go build cache", Languages, Safe, UNIX,
            [Dir("~/Library/Caches/go-build"), Dir("~/.cache/go-build")],
            "Go's compile cache."),
        rule!("go-mod", "Go module cache", Languages, ReportOnly, ALL,
            [Dir("~/go/pkg/mod")],
            "Go modules are stored read-only, so use Go's own command to clear them.",
            cmd = "go clean -modcache"),
        // ------------------------------------------------------------ Tools/IDEs
        rule!("homebrew", "Homebrew downloads", Tools, Safe, MAC,
            [Dir("~/Library/Caches/Homebrew")],
            "Old package downloads. `brew cleanup` also removes old versions.",
            cmd = "brew cleanup --prune=all"),
        rule!("vscode", "VS Code / Cursor caches", Tools, Safe, MAC,
            [Dir("~/Library/Application Support/Code/Cache"),
             Dir("~/Library/Application Support/Code/CachedData"),
             Dir("~/Library/Application Support/Code/CachedExtensionVSIXs"),
             Contents("~/Library/Application Support/Code/logs"),
             Dir("~/Library/Application Support/Cursor/Cache"),
             Dir("~/Library/Application Support/Cursor/CachedData"),
             Contents("~/Library/Application Support/Cursor/logs")],
            "Editor caches and logs. Quit the editor first."),
        rule!("jetbrains", "JetBrains / Android Studio caches", Tools, Safe, MAC,
            [Contents("~/Library/Caches/JetBrains"), Prefixed("~/Library/Caches/Google", "AndroidStudio"),
             Contents("~/Library/Logs/JetBrains"), Prefixed("~/Library/Logs/Google", "AndroidStudio")],
            "IDE indexes and logs. The IDE re-indexes on next open."),
        rule!("docker", "Docker disk image", Tools, ReportOnly, MAC,
            [Dir("~/Library/Containers/com.docker.docker/Data/vms")],
            "Docker keeps images, containers and volumes in one virtual disk. Free space inside it with Docker's own command.",
            cmd = "docker system prune -a   # add --volumes to also drop unused volumes"),
        // --------------------------------------------------------------- System
        rule!("ios-backups", "iPhone / iPad backups", System, Review, MAC,
            [Contents("~/Library/Application Support/MobileSync/Backup")],
            "Local device backups made by Finder. Check you have an iCloud backup before deleting."),
        rule!("user-caches", "App caches", System, Safe, MAC,
            [Contents("~/Library/Caches")],
            "Caches of all your apps. Apps rebuild them; quit apps first for best results."),
        rule!("user-logs", "Logs", System, Safe, MAC,
            [Contents("~/Library/Logs")],
            "App and crash logs."),
        rule!("linux-cache", "User cache folder", System, Safe, LINUX,
            [Contents("~/.cache")],
            "Caches of all your apps."),
        rule!("win-temp", "Temporary files", System, Safe, WIN,
            [Contents("%LOCALAPPDATA%/Temp")],
            "Temporary files left by apps and installers."),
        rule!("trash", "Trash", System, Review, UNIX,
            [Contents("~/.Trash"), Contents("~/.local/share/Trash/files")],
            "Files you already deleted. Emptying the Trash is permanent.",
            permanent = true),
    ]
}

pub fn rules_for(os: Os) -> Vec<Rule> {
    catalog().into_iter().filter(|r| r.os.contains(&os)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = catalog().iter().map(|r| r.id).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }

    #[test]
    fn every_target_expands_somewhere() {
        let env = crate::env::Env::sandboxed(std::path::Path::new("/x"), Os::Windows);
        for r in catalog() {
            for t in r.targets {
                assert!(env.expand(t.raw()).is_some(), "{} {}", r.id, t.raw());
            }
        }
    }

    #[test]
    fn no_rule_targets_user_data() {
        for r in catalog() {
            for t in r.targets {
                let p = t.raw();
                for bad in ["~/Documents", "~/Desktop", "~/Downloads", "~/Pictures", "~/Movies"] {
                    assert!(!p.starts_with(bad), "{} targets {}", r.id, p);
                }
            }
        }
    }
}
