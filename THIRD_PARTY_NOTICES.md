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
- Updated the matching skills under `agent-desktop/skills/` and the documentation under `agent-desktop/docs/` and `agent-desktop/apps/docs/`.

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
