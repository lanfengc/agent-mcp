---
name: agent-desktop-windows
tags: windows-automation, accessibility, uia, ai-agent, gui-automation, cli
requirements:
  - agent-desktop
description: >
  Windows platform guide for the agent-desktop CLI. Lists per command group
  what works on Windows and what refuses, the first-contact gotchas
  (PowerShell ref quoting, minimized windows, headed-only input), and where
  to find shell surfaces, notifications, the cursor overlay, UIPI elevation,
  Chromium/Electron settle behavior and troubleshooting.
  Triggers on: "windows desktop automation", "UIA tree", "automate Windows app",
  "agent-desktop on Windows", "elevated window input blocked", "E_ACCESSDENIED",
  or any Windows GUI interaction task.
---

# agent-desktop-windows

This file covers only what differs on Windows. The observe-act loop, refs, JSON
envelope, sessions and tracing are the same on every platform.

Requires Windows 10 1809+ / Windows Server 2019+ (x64 or ARM64). Run it in the
interactive desktop session of the signed-in user. A command started outside
every session with a user at its desktop (session 0, a service or a SYSTEM
task) is rerun inside the logged-in desktop when the caller holds
`SeTcbPrivilege`, as LocalSystem does: the rerun command inherits stdout and
stderr, reads stdin from `NUL`, and its exit code is returned. It starts from
the logged-in user's environment and state root, so pass `--session` and
`--agent-id` as flags; `AGENT_DESKTOP_*` variables set on the service do not
reach it. The execution policy is the exception: the caller applies it before
the rerun. Without the privilege the command returns `PERM_DENIED`. The secure
desktop (UAC prompts, the sign-in screen) is refused with `PERM_DENIED`:
observation, actions and capture cannot run there. A locked session is not
detected while its lock screen is showing, because the input desktop is still
`Default`: commands run, and clipboard commands return `TIMEOUT`. Server Core
is not supported. `references/permissions-and-elevation.md` has the details.

## Capability Table

| Group | Commands | Status on Windows |
|-------|----------|-------------------|
| Observation | `snapshot`, `find`, `get`, `is`, `screenshot` | Works. Surfaces are `window`, `focused`, a Chromium modal as `sheet`, and an open application menu as `menu`. `screenshot --ocr` returns recognized lines and words with image-pixel rectangles |
| Surfaces | `list-surfaces` | Works. Lists `window`, `focused` and `sheet` per process, plus `menu` with `item_count` while a menu is open |
| Shell surfaces | `open-system-surface` | Works for `start-menu`, `taskbar`, `system-tray`, `system-tray-overflow` and `action-center`, and returns the window `snapshot --surface <kind>` reads. `quick-settings` returns `INVALID_ARGS` naming `action-center` |
| Ref interaction | `click`, `right-click`, `type`, `set-value`, `clear`, `select`, `toggle`, `check`, `uncheck`, `expand`, `collapse`, `scroll`, `scroll-to` | Works, semantic in headless and `--headed` mode. `right-click` and `type` need `--headed` and return `POLICY_DENIED` headless |
| Multi-click and focus | `double-click`, `triple-click`, `focus` | Works with global `--headed` only |
| Keyboard and mouse | `press`, `hover`, `drag`, `mouse-click`, `mouse-move`, `mouse-wheel` | Works. Cursor-moving input needs `--headed`; `mouse-click --background` is headless and posts window messages. First Contact covers `press --app` and background clicks |
| Held input | `key-down`, `key-up`, `mouse-down`, `mouse-up` | Fails closed on every platform with `ACTION_NOT_SUPPORTED`. A headless `mouse-down` or `mouse-up` returns `POLICY_DENIED` first |
| App and window | `launch`, `close-app`, `list-windows`, `list-apps`, `focus-window`, `resize-window`, `move-window`, `minimize`, `maximize`, `restore` | Works. First Contact covers the name `launch` takes |
| Displays | `list-displays` | Works |
| Clipboard | `clipboard-get`, `clipboard-set`, `clipboard-clear` | Works; typed text and image content |
| Wait | `wait` | Works for ms, `--element`, `--window`, `--text`, `--menu`, `--menu-closed`, `--event` and `--notification` |
| Notifications | `list-notifications`, `dismiss-notification`, `dismiss-all-notifications`, `notification-action` | Works over the Action Center. Pass `--headed` when the center is closed |
| Cursor overlay | `cursor-overlay` | Works. `enable` returns `data.rendered: true` only after the renderer acknowledges over its pipe. See `references/shell-and-overlay.md` |
| System | `status`, `permissions`, `version`, `batch`, `skills`, `session`, `trace` | Works |

## First Contact

- **Quote every ref in PowerShell.** PowerShell reads a bare `@token` as its
  splatting operator and deletes the argument before the binary sees it, so
  `set-value @s8f3k2p9:e1 hi` fails `INVALID_ARGS`. Write
  `set-value '@s8f3k2p9:e1' hi`. cmd.exe and bash need no quoting.
- **A window behind another window is drivable. A minimized one is not.**
  Every element of a minimized window reports `offscreen` and ref actions
  fail. Run `restore --app <image>` and snapshot again. Do not use
  `focus-window` by reflex: it steals the user's foreground. `screenshot
  --window-id` still captures a minimized window: it restores the window for
  the capture and minimizes it again, so the window shows on screen for a
  moment.
- **Background clicks use image pixels, not window bounds.**
  `screenshot --window-id` returns `capture_id`, `window_bounds` and
  `capture_bounds`. Pass `capture_id` unchanged to
  `mouse-click --background --window-id <id> --capture-id <capture_id> --xy PX,PY`;
  `PX,PY` are pixels in that image and are mapped through `capture_bounds`.
  The click posts `WM_MOUSEMOVE`, button down and button up to the window, or
  to the child control under the point, and needs no `--headed`. The cursor
  and focus stay where they are unless you add `--cursor-fallback` or
  `--restore-minimized`. `--count N` repeats the three messages 100 ms apart
  and never sends a double-click message, so a control that acts only on that
  message may not react. It is best effort and always `delivered_unverified`:
  verify from fresh state before reporting success. The capture is refused
  with `STALE_REF` (`details.kind: "stale_capture"`) when it belongs to
  another window, the window moved or resized, its process instance changed,
  or it is older than the capture-age limit (10 seconds by default; set
  `AGENT_DESKTOP_MAX_CAPTURE_AGE_MS` for a slower vision round-trip). Points
  outside the client area and `--modifiers` are refused with `INVALID_ARGS`.
- **Three background-click options cover the hard targets.** A minimized
  target is refused unless `--restore-minimized` is passed; that restores it
  for the click and minimizes it again, and the window is shown and can take
  focus while it is restored. `--cursor-fallback` aims the cursor at the point
  while the messages are posted, then puts it back, for frameworks that read
  the physical cursor. `--semantic` invokes the UI Automation element at the
  point instead of posting messages; it finds the element in the target
  window's own subtree, so it works while the window is covered or minimized
  with `--restore-minimized`, and it is the reliable path for WinForms buttons.
  It accepts only the left button, and it falls back to posted messages when
  nothing at the point can be invoked. `details.method` reports `PostMessage`,
  `PostMessage+cursor` or `UIA.Invoke`; for a posted click,
  `details.target_hwnd` is the window that received the messages.
- **Some frameworks ignore posted clicks.** A classic Win32 button was verified
  to click while its window was unfocused and the cursor was elsewhere. A
  WinForms control checks the physical cursor position and async key state, so
  neither a posted click nor `--cursor-fallback` triggers it; `--semantic`
  does. Treat a posted click as best effort and verify every one.
- **No permission dialog exists.** `permissions` reports `automation` as
  `not_required`. Elevation boundaries are the only access control.
- **Chromium and Electron apps read thin before they settle.** Pass a longer
  `--timeout-ms` to `snapshot` instead of concluding the tree is empty.
- **Blocked shortcuts.** `alt+f4`, `win+l`, `win+d`, `alt+tab`, `win+r`,
  `win+x`, `win+e`, `ctrl+shift+esc` and their modifier supersets are refused
  without `--force`. In combos, `cmd` and `meta` are the Windows key, so
  `meta+c` is a shell shortcut. Write `ctrl+c` to copy.
- **`type` needs `--headed`.** It is keystroke synthesis through `SendInput`,
  so a strict-headless `type` returns `POLICY_DENIED`. Use `set-value`, or pass
  `--headed`. It ends `delivered_unverified` (`verification_scope:
  "unavailable"`) when the field exposes no readable Text selection, and can
  verify when one is readable: confirm with a fresh `get`.
- **`press --app` writes to the foreground queue** with no per-process
  targeting. A headless press to a non-frontmost app fails closed. A
  non-interactive caller (service, scheduled task, CI job) gets
  `delivered_unverified`.
- **`launch` takes an absolute path or a bare name** under System32 or the
  Windows directory, with or without `.exe` (`notepad` or `notepad.exe`), not
  a display name. Use `list-apps` to find a running app.
- **Notifications:** `list-notifications` with the Action Center closed needs
  `--headed`. Single-item `dismiss-notification` and `notification-action` need
  `--headed` and `--expected-app` or `--expected-title` for the entry at INDEX.
- **Surfaces:** an open or context menu is the `menu` surface. Windows does
  not advertise a `menubar` surface.
- **Commands that raise shell chrome take the foreground.** Strict headless
  refuses them with `POLICY_DENIED` before anything is raised.
- **Stateful actions run once, then verify by re-reading.** `delivered_verified`
  means the readback matched. `ACTION_FAILED` with
  `details.kind: "post_action_verification"` means the action landed but the
  readback differs: read `details.post_state` and do not retry. A `set-value`
  the app reformats reads as failed. See `references/troubleshooting.md`.
- **Unsigned binary warnings are expected.** See `references/troubleshooting.md`.

## What differs on Windows

- Sandboxes are not available on Windows yet. To run agent-desktop in a container you manage yourself, see `agent-browser.dev/desktop/guides/docker-container`.

- Ref actions are semantic UI Automation calls in headless and `--headed` mode, except `right-click`, `double-click`, `triple-click`, `type` and `focus`, which are physical or take focus and need `--headed`.
- Windows adds `mouse-click --background`: a click posted to a window as `WM_MOUSEMOVE`, `WM_LBUTTONDOWN` and `WM_LBUTTONUP` (or the right or middle variants). It needs no `--headed`, and by default it neither moves the cursor nor changes focus. `--restore-minimized`, `--cursor-fallback` and `--semantic` extend it to minimized windows, cursor-reading frameworks and UI Automation controls. It is Windows-only; other platforms return `PLATFORM_NOT_SUPPORTED`. See First Contact for the capture and coordinate rules.
- `screenshot --ocr [--ocr-query TEXT]` recognizes text with the Windows OCR engine and returns `data.ocr`: lines and words with rectangles in image pixels. `--ocr-query` adds `data.ocr_match` with `center_x` and `center_y` for the best match: a word that is or contains the text, else the text as a phrase across the words of one line, else the first line that contains it. `ocr_match` is omitted when nothing matches. Feed that center to `mouse-click --background --capture-id` to click a non-UIA, drawn or game-like surface. `--ocr-language` pins the engine language; the default uses the user's profile languages. Both flags need `--ocr`.
- A ref resolves only when it was the only element in the window its snapshot captured with its role, native id, name and description, and exactly one live element still matches. Uniqueness counts the whole window, not the shallow tree a `--skeleton` or `--max-depth` snapshot presents, so a twin below the cut still refuses the ref. Identical twins, such as several identical `Delete` buttons, are refused with `AMBIGUOUS_TARGET`, `details.kind: identity_not_unique`, before anything reaches the app; a live second match is `saved_ref_ambiguous`, and a removed element is `STALE_REF`, `saved_ref_absent`. Take a fresh snapshot after a list changes, and use `--headed mouse-click --xy` when you must pick one of two identical controls.
- The cursor overlay plays its per-action travel only for headless semantic actions. A `--headed` action moves the real cursor instead.

## References

- `references/shell-and-overlay.md`: open it for `open-system-surface`, tray
  and taskbar surfaces, notifications, the cursor overlay, hosted (UWP) window
  identity and menu detection coverage.
- `references/permissions-and-elevation.md`: integrity levels, UIPI, blocked
  combos, protected processes, the cross-process interaction lease.
- `references/chromium-and-electron.md`: settle timing, covered-window hazard,
  WPF wrong-provider trap, unnamed web content.
- `references/troubleshooting.md`: symptom-to-cause map, action verification
  outcomes, saving a document headless, install warnings.

Read any file from the binary:
`agent-desktop skills get agent-desktop-windows references/troubleshooting.md`.
