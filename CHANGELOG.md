# Changelog

All notable changes to Mr.Clean. Versions follow [Semantic Versioning](https://semver.org/).

## v0.1.0 — first release

The first public build, for **macOS**, **Windows 10/11** and **Linux** (Ubuntu 22.04+).

### Overview
- Health headline ("Your Mac, in good shape" / "needs attention") with the reason, and a pill that jumps to the fix.
- Device card with the real model, chip, RAM and OS version (MacBook Pro, Lenovo ThinkPad, Dell XPS…).
- Glass storage orb, **Scan everything** (caches, `node_modules`, large files and security in one go), "What we found" and quick actions.
- Light and dark themes; motion respects Reduce motion.

### Clean
- Developer caches for Xcode, Android/Gradle, JavaScript package managers, CocoaPods, SwiftPM, pip, cargo, Go, .NET/NuGet, Homebrew, VS Code/Cursor, JetBrains/Android Studio, app caches, logs, Windows temp files and crash dumps; apt, journal, Flatpak and Snap on Linux (info only, with the command to free them).
- **Simulator runtimes** can be removed: counted once, shown by name ("iOS 17.5"), removed with Xcode's tool or one macOS password prompt.
- Filter chips, tool logos, and an Old `node_modules` panel that skips online-only iCloud/OneDrive copies.

### Files
- Fast home-folder scan, biggest files by type and age, search, and a side panel with what's inside.
- **"What is this?"** explanations for folders (Apple Intelligence caches, `.gemini`, AppData, Store apps, `~/.config`…), with advice on whether to delete.

### Security
- Checks for the Shai-Hulud npm worm, Contagious Interview / BeaverTail projects, known-bad npm versions, risky install scripts, git hooks and identity changes, shell and PowerShell profiles, exposed tokens.
- Startup items on every OS: LaunchAgents, cron, systemd and autostart, Windows Run keys, Startup folder and scheduled tasks.
- **Protection card**: FileVault/BitLocker/disk encryption, firewall, Gatekeeper/SmartScreen, System Integrity Protection/UAC, updates, remote login, each with **Fix** (opens the right settings page) or **How to fix**.
- One-click, confirmed fixes for a global git hooks folder and plain-text git passwords.
- Restorable **quarantine**; quarantining a Mac startup item also stops it straight away.

### Memory
- Apps / wired / compressed / free breakdown, 10-minute pressure chart, apps with their real icons, and developer leftovers you can quit safely.

### Safety
- Every delete passes one tested guard. It never touches your documents, cloud folders, credentials, system folders (`/System`, `C:\Windows`, Program Files…) or app data outside caches and logs. Items go to the Trash / Recycle Bin by default.
- Nothing is uploaded; there's no telemetry and no auto-update.
