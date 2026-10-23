## What and why

## Checklist
- [ ] `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass
- [ ] Schema changes are a **new numbered migration** (no edits to released ones)
- [ ] UI changes checked right-to-left and on a narrow screen; still no CDN or third-party scripts
- [ ] Docs / `CHANGELOG.md` updated when behaviour or configuration changes
- [ ] No secrets, tokens, uploads or real customer data in the diff
