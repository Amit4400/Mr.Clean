# CLAUDE.md

Guidance for Claude Code (and any AI assistant) working in this repository. Humans: see [CONTRIBUTING.md](CONTRIBUTING.md); the rules are the same.

## What this is

Mr.Clean is a Tauri 2 desktop app that frees disk space on developer machines, finds large files, scans for developer-targeted malware, and shows RAM use. macOS is the first platform; Linux works; Windows is planned.

```
crates/core/        All logic, no UI. Every behaviour lives and is tested here.
  src/safety.rs     THE deletion guard. Every delete must pass through it.
  src/cleaner/      rules.rs = catalog of cache locations; mod.rs = scan/clean; node_modules.rs
  src/bigfiles.rs   folder-size index, biggest files, user-picked deletes
  src/memory.rs     processes, dev leftovers, protected quit
  src/security/     malware checks (read-only), iocs.json, quarantine, scan_tree (CI self-scan)
  src/system.rs     dashboard info
  examples/         scan.rs (read-only CLI of every scanner), repo_scan.rs (CI malware gate)
src-tauri/          Thin Tauri commands wrapping the core. No logic here.
src/                React + TypeScript + Tailwind UI. lib/api.ts calls Tauri, or lib/mock.ts in a browser.
e2e/                Playwright smoke tests (run against the mock backend)
```

## Golden rules (never break these)

1. **Every file deletion goes through `crates/core/src/safety.rs`** (`Guard::check_cache` or `Guard::check_user_file`, then `remove_checked`). Never call `fs::remove_*` or `trash::delete` anywhere else.
2. **New cache locations are added only as entries in `cleaner/rules.rs`.** Mark them honestly: `Safe` (rebuilt automatically), `Review` (user may want it) or `ReportOnly` (needs a tool command or admin rights).
3. **Never widen what can be deleted** (no user-data folders, credentials, `~/Library/Preferences`, system paths) without an explicit request from the maintainer and new tests.
4. **The security scanner is read-only.** It reports; only the user can quarantine, and quarantine must stay restorable.
5. **No network calls, telemetry or auto-updates.** Everything runs locally.
6. **Never quit protected processes.** Protection is re-checked in Rust, not just the UI.
7. **The UI must never offer an action the Rust guard would refuse.** `src/lib/paths.ts` mirrors the guard; keep them in sync and update both test suites.
8. Tests never touch the real home folder. Use `Env::sandboxed(tempdir, os)`.

## Commands

```bash
npm install                                        # once
npm run tauri dev                                  # run the app (hot reload)
npm run dev                                        # UI only, in a browser, with mock data (http://localhost:1420)

cargo fmt --all                                    # format Rust
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                             # Rust tests (sandboxed fake homes)
npm run typecheck                                  # TypeScript
npm test                                           # Vitest unit tests
npm run test:e2e                                   # Playwright smoke tests (builds the UI first)
cargo run -p mrclean-core --example repo_scan -- . # malware self-scan, same as CI
cargo run --release -p mrclean-core --example scan -- all   # read-only scan of this machine
```

The Rust toolchain is pinned in `rust-toolchain.toml` (same version as CI). Tauri needs `dist/` to exist for `cargo` builds; run `npm run build` once, or `mkdir -p dist`.

## Workflow

- **Never push to `main`.** It's protected; all changes go through a pull request that the maintainer reviews and merges.
- Branch names: `feat/<short-name>`, `fix/<short-name>`, `chore/<short-name>`, `docs/<short-name>`.
- Commits: [Conventional Commits](https://www.conventionalcommits.org/) (`feat: add Flutter cache rule`, `fix(safety): …`). Small, focused commits.
- Before pushing, run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && npm run typecheck && npm test`.
- Fill in the PR template, including how you tested and the safety checklist.
- AI-assisted commits keep at most a one-line `Co-Authored-By:` trailer. Don't add other generated-by banners to code, docs, commits or PR descriptions.

## Style

- Match the surrounding code; keep comments short and about *why*.
- Rust: `rustfmt.toml` (120 cols), clippy clean. Return user-facing error strings in plain English.
- UI copy: short, plain words for non-experts ("Move to Trash", not "Unlink inode").
- Every new behaviour needs a test: Rust unit test for logic, Vitest for UI helpers, and extend `e2e/smoke.spec.ts` when a flow changes.
