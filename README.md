# Mr.Clean

A small desktop app that keeps developer Macs fast and safe. It cleans out the junk that normal cleaners miss, shows you what's filling your disk, looks for malware that targets developers, and helps you free up RAM.

Built with [Tauri 2](https://tauri.app): a Rust core does all the scanning, and a React UI sits on top. The installer is about 10 MB and the app uses little memory, so it runs well on an 8 GB MacBook. macOS comes first; Linux already builds, and Windows is planned.

## Features

| Page | What it does |
|---|---|
| **Dashboard** | Your Mac's model, CPU, storage used/free and live RAM use, plus a "Scan everything" button. |
| **Dev Cleaner** | Finds caches from Xcode (DerivedData, device support, **iOS simulators**, even after Xcode is uninstalled), Android (Gradle, emulators, system images), npm/yarn/pnpm/bun, CocoaPods, SwiftPM, pip, cargo, Homebrew, IDEs, app caches and logs. Also lists `node_modules` in projects you haven't touched for a while. |
| **Large Files** | A fast parallel scan of your home folder. Browse folders sorted by size, see the biggest files filtered by type and age, and remove what you choose. |
| **Security** | Looks for developer-targeted malware: the **Shai-Hulud** npm worm, **Contagious Interview / BeaverTail** fake-job projects, known-compromised npm package versions, risky install scripts, suspicious LaunchAgents and cron jobs, git hooks that push code, a changed git identity, code injected into shell profiles, and tokens left in plain text. |
| **Memory** | RAM and swap use, apps grouped by memory, and developer leftovers (idle Gradle/Kotlin daemons, ADB, emulators, simulators, forgotten dev servers, orphaned Node processes) that you can quit safely. |

## Safety rules

- **Every delete goes through one guard** (`crates/core/src/safety.rs`) with its own unit tests. The cleaner can only touch folders listed in its catalog. It refuses anything in Documents, Desktop, Downloads, Pictures, iCloud, `.ssh`, keychains, preferences or system folders, and paths are resolved so `..` and symlinks can't escape.
- **Nothing is deleted without confirmation**, and by default items go to the **Trash** (you can change this in Settings).
- Rules are labelled **Safe** (rebuilt automatically, pre-selected), **Review** (emulators, archives, backups; never pre-selected) or **Info only** (e.g. Docker, which needs its own command such as `docker system prune`).
- **The security scanner never deletes anything.** It only reports. You can move a flagged file to quarantine (`~/.mrclean/quarantine`, with execute permission removed) and restore it with one click.
- **Only your own, non-system processes can be quit.** macOS GUI apps get a normal "quit" so they can prompt you to save.
- Everything runs on your Mac. Nothing is uploaded.

## Install (for teammates)

1. Download `Mr.Clean_x.y.z_universal.dmg` from the repo's **Actions → Build → Artifacts** (or from a GitHub Release).
2. Open the dmg and drag **Mr.Clean** into **Applications**.
3. The first time, **right-click the app → Open → Open**. Without an Apple Developer ID the app is unsigned, so macOS shows an "unidentified developer" warning.
4. Recommended: grant **Full Disk Access** (Settings page → *Open Privacy settings* → turn on Mr.Clean → restart the app) so every cache folder is visible.

## Develop

Full setup for macOS, Linux and Windows is in **[CONTRIBUTING.md](CONTRIBUTING.md)**. Quick start:

```bash
npm install
npm run tauri dev          # run the app with hot reload
npm run dev                # UI only, in a browser with sample data (no Rust needed)
cargo test --workspace     # core tests (fake home folders, never your real files)
npm test                   # UI unit tests
npm run test:e2e           # end-to-end tests in Chromium
npm run tauri build        # build an installer for this OS
```

To see what the scanners find on your machine without the UI (read-only):

```bash
cargo run --release -p mrclean-core --example scan -- all   # or: cleaner | files | security | memory
```

Working with Claude Code or another AI assistant? Read **[CLAUDE.md](CLAUDE.md)** first.

### Layout

```
crates/core/          all logic, no UI (unit-tested)
  src/safety.rs       the deletion guard
  src/cleaner/        rule catalog (rules.rs), scan/clean, stale node_modules
  src/bigfiles.rs     folder-size index + biggest files
  src/memory.rs       processes, dev leftovers, safe quit
  src/security/       malware checks + iocs.json (indicators of compromise)
  src/system.rs       dashboard info
src-tauri/            Tauri app: thin command wrappers around the core
src/                  React + TypeScript + Tailwind UI
e2e/                  Playwright end-to-end tests (mock backend)
.github/workflows/    ci.yml (tests), security.yml (CodeQL, self-scan, audits), build.yml (.dmg)
```

### Adding a cache location

Add a `rule!(…)` entry in `crates/core/src/cleaner/rules.rs`. Use `Dir` to delete the folder itself or `Contents` to empty it, and mark it `Safe`, `Review` or `ReportOnly`. The tests check that rule IDs are unique and that no rule points into user-data folders.

### Updating malware indicators

`crates/core/src/security/iocs.json` holds the bundled list of compromised packages, malware file names and folders. You can also drop an extra file at `~/.mrclean/iocs.json` on any Mac, in the same format; it's merged in on the next scan, with no rebuild needed.

## Releasing

Push a tag like `v0.2.0`. CI builds the universal `.dmg` and attaches it to a GitHub Release. To get rid of the "unidentified developer" warning, add an Apple Developer ID certificate and notarization secrets to the workflow ([Tauri guide](https://v2.tauri.app/distribute/sign/macos/)).

## Contributing & security

Pull requests are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md). Report vulnerabilities privately: [SECURITY.md](SECURITY.md).

## License

GPL-3.0
