# Third-party notices

This repository contains two Apache-2.0 licensed upstream projects, imported as source snapshots and modified locally.

## agent-desktop

- Upstream: https://github.com/vercel-labs/agent-desktop
- Snapshot: `54855b6225aec00da4ac65e2d0362fd41cd9fcb3` (`main`, imported 2026-11-12)
- License: Apache License 2.0, see `agent-desktop/LICENSE`
- Copyright: Vercel, Inc. and agent-desktop contributors

Local modifications:

- Added background window clicks: `mouse-click --background --window-id --capture-id`, implemented with `PostMessageW` in `agent-desktop/crates/windows/src/input/mouse/mouse_background.rs` and dispatched through `agent-desktop/crates/core/src/commands/mouse_click_background.rs`.
- Added capture metadata: `capture_id`, `window_id`, `window_bounds`, `capture_bounds` and `capture_time_ms` on window screenshots, bound by `agent-desktop/crates/core/src/system/capture_token.rs`. Windows capture bounds come from `DWMWA_EXTENDED_FRAME_BOUNDS`.
- Added execution policy: `AGENT_DESKTOP_READ_ONLY`, `AGENT_DESKTOP_ALLOWED_COMMANDS`, `AGENT_DESKTOP_AUDIT_LOG` and `AGENT_DESKTOP_MAX_CAPTURE_AGE_MS`, in `agent-desktop/src/command_policy/safety.rs` and `agent-desktop/src/dispatch/keyboard_mouse.rs`.
- Added Windows OCR fallback: `screenshot --ocr`, `--ocr-language` and `--ocr-query`, using `Windows.Media.Ocr` in `agent-desktop/crates/windows/src/system/ocr.rs`, with shared result types in `agent-desktop/crates/core/src/system/ocr_*.rs` and query matching in `agent-desktop/crates/core/src/commands/screenshot.rs`.
- Added minimized-window capture: `screenshot --window-id` restores a minimized window for `PrintWindow` and minimizes it again, in `agent-desktop/crates/windows/src/system/capture/capture_window.rs`; window geometry uses the placement's normal rectangle in `window_enum.rs`.
- Extended background clicks with `--restore-minimized`, `--cursor-fallback` and `--semantic`, in `agent-desktop/crates/windows/src/input/mouse/mouse_background.rs` and `mouse_background_semantic.rs`, with options in `agent-desktop/crates/core/src/input/background_click_options.rs`.
- Added session placement recovery: a command started in another WTS session is relaunched on the active user's `Default` desktop through `WTSQueryUserToken` and `CreateProcessAsUserW` when the caller holds `SeTcbPrivilege`; locked, secure and absent desktops are refused. See `agent-desktop/src/desktop_session.rs` and `agent-desktop/crates/windows/src/system/session_probe.rs` / `session_relaunch.rs`.
- Updated the matching skills under `agent-desktop/skills/` and the documentation under `agent-desktop/docs/` and `agent-desktop/apps/docs/`. New CLI flags are exposed to MCP automatically from the clap command tree.

## agent-browser

- Upstream: https://github.com/vercel-labs/agent-browser
- Snapshot: `2f1649cefb82f417abbb2712f5397439d010698e` (`main`, imported 2026-11-12)
- License: Apache License 2.0, see `agent-browser/LICENSE`
- Copyright: Vercel, Inc. and agent-browser contributors

Local modifications:

- Local Chrome launches without a profile now reuse a persistent per-session profile under the agent-browser state directory, so cookies, storage, and cache survive browser and daemon restarts. Explicit profiles, state replay, CA certificates, allowed-domains containment, CDP attach, auto-connect, and non-Chrome engines keep their existing behavior. See `agent-browser/cli/src/native/state.rs` and `agent-browser/cli/src/native/actions.rs`, with matching docs under `agent-browser/docs/`, `agent-browser/skill-data/`, and `agent-browser/cli/src/output.rs`.

## Bundled third-party assets

Both projects carry their own third-party license files:

- `agent-browser/cli/src/native/a11y/LICENSE-axe-core.txt`
- `agent-browser/cli/src/native/a11y/LICENSE-axe-core-THIRD-PARTY.txt`
