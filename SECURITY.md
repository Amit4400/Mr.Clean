# Security policy

Mr.Clean deletes files and inspects security-sensitive locations, so we take vulnerabilities seriously.

## Reporting a vulnerability

**Please don't open a public issue.** Report privately via GitHub: **Security → Report a vulnerability** on this repository ([direct link](https://github.com/Sahilsalariasoftradix/Mr.Clean/security/advisories/new)).

Include what you found, how to reproduce it, and the impact. We aim to reply within 7 days and to fix confirmed issues before disclosing them.

## In scope

- Any way to make Mr.Clean delete, move or quarantine something outside what the UI showed and the user confirmed (e.g. escaping `safety.rs` via symlinks, `..`, races, crafted file names).
- Stopping protected or other users' processes.
- Code execution through scanned files (the scanner must only read).
- Malicious code or dependencies getting into the repo or release builds.

## Out of scope

- The scanner missing a piece of malware, or flagging a harmless file. Please open a normal issue for these.
- Issues that need an attacker to already control your user account.

## Our commitments

- No network calls or telemetry: everything stays on your machine.
- Releases are built by GitHub Actions from `main`, which is protected: no force-pushes, and every change is reviewed.
