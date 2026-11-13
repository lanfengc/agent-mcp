## Summary

<!-- What changed and why — 1–3 lines. -->

## Related issues

Closes #

## Type of change

- [ ] `feat:` — new feature
- [ ] `fix:` — bug fix
- [ ] `docs:` — documentation only
- [ ] `refactor:` — no behavior change
- [ ] `perf:` — performance improvement
- [ ] `test:` — adding or fixing tests
- [ ] `chore:` — maintenance / dependencies
- [ ] `ci:` — CI/CD changes
- [ ] `BREAKING CHANGE` — incompatible ABI or CLI change

## How tested

<!-- Check every gate you ran. -->

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --lib`
- [ ] `cargo test -p agent-desktop`
- [ ] `cargo test -p agent-desktop-ffi --tests`
- [ ] `powershell -NoProfile -File tests/e2e-windows/Run-E2E.ps1` (requires a `--release` build and an exclusive desktop; run when behavior changes)

## Checklist

- [ ] PR title uses a conventional-commit prefix (`type: description`, lowercase, no capital after colon)
- [ ] `cargo fmt` and `cargo clippy -D warnings` are clean
- [ ] All tests pass
- [ ] No `unwrap()` added in non-test code
- [ ] No inline comments added (only `///` doc-comments on public items)
- [ ] All modified files are within the 400 LOC limit
- [ ] Skill docs updated if a command, flag, or JSON output changed (`skills/agent-desktop/`, `skills/agent-desktop-windows/` or `skills/agent-desktop-ffi/`)
- [ ] README / other docs updated if the public interface changed
