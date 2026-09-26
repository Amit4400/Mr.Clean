# One-time GitHub setup (maintainer)

These settings protect `main` and make the repo safe to open to contributors. They only need doing once, in the GitHub website: **Repository → Settings**.

## 1. Rulesets for `main`

**Settings → Rules → Rulesets → New ruleset → Import a ruleset**, and import each file:

| File | What it enforces |
|---|---|
| [`.github/rulesets/main-locked.json`](../.github/rulesets/main-locked.json) | `main` can't be **deleted** or **force-pushed**, by anyone, including you. |
| [`.github/rulesets/main-pull-requests.json`](../.github/rulesets/main-pull-requests.json) | Every change goes through a **pull request**. It needs 1 approval from the code owner (you), all conversations resolved, and **all CI and security checks green**, and new pushes dismiss old approvals. |

About merging your own PRs: GitHub doesn't let you approve your own pull request. The second ruleset therefore lets repository admins (only you) **bypass it inside a pull request**. You'll see a "Merge without waiting for requirements" checkbox. **Only tick it once all checks are green.** Direct pushes to `main` stay blocked, even for you.

Contributors can't merge anything: on a public repo only people you give *write* access can merge, and by default that's only you.

## 2. Code security

**Settings → Code security**, turn on:

- **Private vulnerability reporting**: lets people report security issues privately (used by SECURITY.md)
- **Dependency graph** and **Dependabot alerts**
- **Dependabot security updates**
- **Secret scanning** and **Push protection**: blocks pushes that contain tokens or keys
- **Code scanning**: CodeQL results from `security.yml` appear here automatically

## 3. Actions

**Settings → Actions → General**:

- **Fork pull request workflows from outside collaborators** → *Require approval for all external contributors*. Untrusted code never runs in CI until you've looked at it.
- **Workflow permissions** → *Read repository contents and packages permissions*, and leave *Allow GitHub Actions to create and approve pull requests* **unticked**.

## 4. General

**Settings → General → Pull Requests**:

- Allow **squash merging** (keeps `main` history tidy) and turn off *Allow auto-merge*.
- Tick **Automatically delete head branches**.

## Cost

This repository is public, so GitHub Actions minutes (including the macOS build) and all the security features above are free.
