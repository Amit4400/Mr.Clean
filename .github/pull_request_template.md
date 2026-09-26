## What and why

<!-- What does this change, and why is it needed? Link the issue if there is one. -->

## How I tested it

<!-- Commands you ran, and what you checked by hand (which OS?). Screenshots for UI changes. -->

- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`
- [ ] `npm run typecheck`, `npm test`, `npm run test:e2e`

## Safety checklist

- [ ] Any file deletion goes through `crates/core/src/safety.rs` (no other `remove_*` / trash calls)
- [ ] No new network calls, telemetry or auto-update
- [ ] New cache rules are labelled honestly (`Safe` only if the tool rebuilds it automatically)
- [ ] The security scanner still only reads files
- [ ] New behaviour has tests
