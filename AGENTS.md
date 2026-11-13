# AGENTS.md

This repository holds two imported upstream projects and one original tool side by side. Each directory keeps its own `AGENTS.md`; read the one for the project you change, and follow it over this file. Everything here targets Windows; macOS and Linux are not supported.

## Layout

- `agent-desktop/` is a Cargo workspace: `crates/core`, `crates/windows`, `src/` for the CLI, `skills/` for compiled-in skills.
- `agent-browser/` is a pnpm workspace; its Rust CLI is a separate Cargo package at `agent-browser/cli/`.
- `agent-android/` is a standalone Cargo project with no root workspace: an adb-backed CLI and stdio MCP server for Android devices.
- `bin/` holds the three built entry points, `agent-desktop.exe`, `agent-browser.exe` and `agent-android.exe`, plus `skill-data/` and `skills/`, a copy of the browser skills that `agent-browser.exe` looks up beside itself. The build script writes all of it and git ignores the directory.
- `scripts/build-windows.ps1` builds all three binaries and refreshes `bin/`. It does not touch any project's own build files.

## Rules

- Local modifications to imported code are listed in `THIRD_PARTY_NOTICES.md`. Do not add app-specific branches to imported code.
- Keep each project's build system independent. Do not add a root Cargo workspace, pnpm workspace or shared lockfile.
- Follow the desktop commit rules: Conventional Commits, lowercase after the colon, linear `main`, no `Co-Authored-By` lines, no AI attribution and no "Generated with" footers.
- `agent-desktop` rejects inline comments, hand-written Rust files over 400 lines, structs over 7 fields and functions over 5 parameters. Its tests sit beside their code.
- `agent-android` is original MIT code. It keeps hand-written Rust files under 400 lines, avoids inline comments and keeps tests beside their code.
- `agent-browser` requires pnpm 11 and Node 22 for its Node launcher and dashboard. The Rust CLI alone needs only Cargo.

## Verification

Run each block inside the directory it names. Before the `agent-desktop` and `agent-browser` tests, point `HOME` and `USERPROFILE` at an empty scratch directory and keep `CARGO_HOME` and `RUSTUP_HOME` on the real ones. Desktop tests otherwise write refmaps into the real `~/.agent-desktop`, which is why the desktop project's Windows CI lane isolates both variables. The same isolation keeps browser tests away from `~/.agent-browser`, which holds login sessions.

`agent-desktop/`. The local patch spans `crates/core`, `crates/windows`, `crates/ffi` and `src/`, so run every host test target, not only the binary crate. The workspace is Windows-only and builds every package on this host.

```powershell
$pkgs = '-p', 'agent-desktop-core', '-p', 'agent-desktop-windows', '-p', 'agent-desktop', '-p', 'agent-desktop-ffi'
cargo fmt --all -- --check
cargo clippy --locked @pkgs --all-targets -- -D warnings
cargo test --locked @pkgs
```

The two source gates are bash scripts. Run them from Git Bash, the shell git uses for hooks:

```bash
scripts/check-architecture.sh
scripts/check-rust-file-size.sh
```

`agent-browser/cli/`:

```powershell
cargo build
cargo test
```

`agent-android/`:

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

Some upstream tests assume an English-locale, elevated host and fail elsewhere. Treat a failure outside the files you changed as environmental only after it also fails on the import commit.

The `agent-desktop-windows` library tests act on the live desktop: they launch Notepad, open shell surfaces such as the Start menu, send input and write the clipboard, which they restore afterwards. Run them on an unlocked session that nobody is using. On a locked session every test that opens the clipboard fails, because Windows refuses clipboard access until the session is unlocked.

If another tool has added an access entry to `%TEMP%`, tests that create a state root there fail with `private file DACL grants an untrusted principal`. Point `TMP` and `TEMP` at a directory that only you, SYSTEM and Administrators can access.
