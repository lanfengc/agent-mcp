# AGENTS.md

This repository holds two imported upstream projects side by side. Each directory keeps its own `AGENTS.md`; read the one for the project you change, and follow it over this file.

## Layout

- `agent-desktop/` is a Cargo workspace: `crates/core`, `crates/windows`, `src/` for the CLI, `skills/` for compiled-in skills.
- `agent-browser/` is a pnpm workspace; its Rust CLI is a separate Cargo package at `agent-browser/cli/`.
- `bin/` holds the two built entry points, `agent-desktop.exe` and `agent-browser.exe`; the build script copies them there and git ignores the directory.
- `scripts/build-windows.ps1` builds both binaries and refreshes `bin/`. It does not touch either project's own build files.

## Rules

- Local modifications are listed in `THIRD_PARTY_NOTICES.md`. Do not add app-specific branches to imported code.
- Keep each project's build system independent. Do not add a root Cargo workspace, pnpm workspace or shared lockfile.
- Follow the desktop commit rules: Conventional Commits, lowercase after the colon, linear `main`, no `Co-Authored-By` lines, no AI attribution and no "Generated with" footers.
- `agent-desktop` rejects inline comments, hand-written Rust files over 400 lines, structs over 7 fields and functions over 5 parameters. Its tests sit beside their code.
- `agent-browser` requires pnpm 11 and Node 22 for its Node launcher and dashboard. The Rust CLI alone needs only Cargo.

## Verification

```powershell
cargo build -p agent-desktop-core -p agent-desktop-windows -p agent-desktop -p agent-desktop-ffi
cargo test -p agent-desktop --bins
cargo build --manifest-path agent-browser/cli/Cargo.toml
```

Run Cargo commands inside the project directory they belong to. The desktop workspace names all three platform crates, so an unscoped `cargo test --workspace` fails on every host.
