# Contributing to Mr.Clean

Thanks for helping! Mr.Clean deletes files on people's computers, so safety comes before features. Please read this whole page before opening a pull request.

## Ground rules

- All changes go through a pull request. Only the maintainer (@Sahilsalariasoftradix) merges.
- Everything that deletes a file goes through `crates/core/src/safety.rs`. PRs that delete files any other way are closed.
- The security scanner never deletes or uploads anything.
- No network calls, telemetry or analytics.
- Using an AI assistant is fine. Follow [CLAUDE.md](CLAUDE.md) (it applies to humans too), and you're responsible for every line you submit.

## Set up locally

You need **Rust** (via [rustup](https://rustup.rs); the exact version is pinned in `rust-toolchain.toml` and installs automatically) and **Node.js 22**.

| OS | Extra requirements |
|---|---|
| macOS | Xcode Command Line Tools: `xcode-select --install` |
| Ubuntu / Debian | `sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev build-essential` |
| Windows | [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and WebView2 (preinstalled on Windows 11) |

```bash
git clone https://github.com/<you>/Mr.Clean.git   # your fork
cd Mr.Clean
npm install
npm run tauri dev        # opens the app with hot reload
```

Only working on the UI? `npm run dev` opens it in your browser with realistic sample data, so you don't need Rust. Add `?platform=windows` or `?platform=linux` to the URL to see the Windows or Linux version of the screens.

## Make a change

1. Create a branch: `git checkout -b feat/short-description` (or `fix/…`, `chore/…`, `docs/…`).
2. Make small, focused commits using [Conventional Commits](https://www.conventionalcommits.org/): `feat: add Deno cache rule`.
3. Run the checks (CI runs exactly these):
   ```bash
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   npm run typecheck
   npm test
   npm run test:e2e
   ```
4. Push to your fork and open a pull request against `main`. Fill in the template.

Changing a workflow? Pin third-party actions to a full commit SHA with a version comment, e.g. `uses: owner/action@<sha> # v2`. CodeQL flags movable tags, and Dependabot keeps the pins up to date.

## What CI checks on every pull request

| Check | What it does |
|---|---|
| Test (Rust) | formatting, clippy, all core tests (on fake home folders) |
| Test (UI) | TypeScript, Vitest unit tests, Playwright end-to-end tests |
| Self-scan (malware) | Mr.Clean's own scanner checks the PR for poisoned configs, npm worm files, bad install scripts and suspicious workflows |
| CodeQL | static security analysis for Rust, TypeScript and GitHub Actions |
| Dependency review / cargo-deny / npm audit | blocks known-vulnerable or badly licensed dependencies |
| Secret scan | gitleaks checks that no tokens or keys are committed |
| Build macOS app / Linux app / Windows app | build the `.dmg`, `.deb`/`.AppImage` and `.msi`/`.exe`, so reviewers can install your change |

PRs from forks need a maintainer's approval before CI runs, which stops untrusted code from running automatically.

## Common contributions

**Add a cache location:** add a `rule!(…)` in `crates/core/src/cleaner/rules.rs`. Use `Contents` to empty a folder or `Dir` to remove it, and mark it `Safe` only if the tool rebuilds it automatically. Explain in the PR where the folder comes from and what happens when it's deleted.

**Report or fix a false positive in the security scanner:** include the file content (redact secrets) and the finding. Tighten the pattern in `security/patterns.rs` and add a test to `ignores_normal_config`.

**Add a malware indicator:** add it to `crates/core/src/security/iocs.json` with a public source link in the PR.

## Reporting security problems

Please don't open a public issue. See [SECURITY.md](SECURITY.md).

## Releasing (maintainer)

1. Bump the version in `package.json`, `src-tauri/tauri.conf.json` and `Cargo.toml` (`[workspace.package]`).
2. Add a section for it at the top of [CHANGELOG.md](CHANGELOG.md), open a PR and merge it.
3. From an up-to-date `main`, push the tag: `git tag vX.Y.Z && git push origin vX.Y.Z`.
4. The **Build** workflow builds macOS, Linux and Windows, and its **Publish GitHub Release** job creates the release with every installer attached.
