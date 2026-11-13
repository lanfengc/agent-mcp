# agent-suite

Desktop, browser and Android automation for AI agents, in one repository.

`agent-suite` combines two imported Apache-2.0 tools and one original MIT tool that an agent drives over MCP or the shell. The suite targets Windows: `agent-desktop` and `agent-browser` build and run only on Windows, and `agent-android` needs a Windows host with `adb` for its Android targets.

| Directory | Tool | Upstream snapshot | Local changes |
|-----------|------|-------------------|---------------|
| `agent-desktop/` | Native desktop apps through the OS accessibility tree, plus background window capture and posted clicks on Windows | [`vercel-labs/agent-desktop`](https://github.com/vercel-labs/agent-desktop) at `54855b6225aec00da4ac65e2d0362fd41cd9fcb3` (`main`, 2026-11-12) | Background click options, OCR, minimized-window capture, session recovery, capture metadata, execution policy |
| `agent-browser/` | Web pages through Chrome DevTools Protocol, driven one command at a time | [`vercel-labs/agent-browser`](https://github.com/vercel-labs/agent-browser) at `2f1649cefb82f417abbb2712f5397439d010698e` (`main`, 2026-11-12) | Persistent per-session Chrome profile for local launches that name no profile |
| `agent-android/` | Android phones and emulators through `adb`, with UI snapshots, refs and an MCP server | — (original code in this repository) | — |

All three are tools, not agents. The observe, act and verify loop runs in the calling model.

## Why they live together

A desktop test flow often crosses surfaces: a native client window, a web view opened by that client, and a phone app that pairs with it. One checkout gives the agent all three CLIs, one build script, and one place to record upstream imports and local patches.

The projects keep their own build systems. `agent-desktop` is a Cargo workspace; `agent-browser` is a pnpm workspace whose CLI is a separate Cargo package under `cli/`; `agent-android` is a standalone Cargo project. Nothing is unified at the Cargo or pnpm level, so each upstream can be re-imported without untangling the others.

## What the local desktop patch adds

The desktop tool follows the project's own model: the agent takes a window screenshot, a vision model returns a pixel coordinate, and the tool clicks that pixel in the background. By default the cursor and the foreground window stay where they are.

- `screenshot --window-id <id>` adds `capture_id`, `window_id`, `window_bounds`, `capture_bounds` and `capture_time_ms`.
- `mouse-click --background --window-id <id> --xy X,Y` posts `WM_MOUSEMOVE`, button down and button up with `PostMessageW` to the target window, or to the child control under the point; `--capture-id <id>` interprets `X,Y` as image pixels and maps them through `capture_bounds`.
- The result is always `delivered_unverified` with `retry: unsafe`: the messages were accepted, not proven to have changed the app. Verify from fresh state, do not retry blindly.
- A capture older than `AGENT_DESKTOP_MAX_CAPTURE_AGE_MS` (default 10 s), or one whose window moved, resized or restarted, fails with `STALE_REF` and `details.kind: "stale_capture"`.
- `mouse-click --background` takes three options for targets a posted click does not reach: `--semantic` invokes the UI Automation element at the point instead of posting messages (left button only; it falls back to posted messages when nothing at the point can be invoked), `--cursor-fallback` aims the cursor at the point while the messages are posted and then restores it, and `--restore-minimized` restores a minimized target for the click and minimizes it again.
- `screenshot --window-id <id>` also captures a minimized window: it restores the window for the capture and minimizes it again, so the window shows on screen for a moment.
- `screenshot --ocr` recognizes text with `Windows.Media.Ocr` and returns lines and words with image-pixel rectangles. `--ocr-language` picks the language, and `--ocr-query` adds the best-matching word, phrase or line with its center point.
- A command started outside every Windows session that has a user at its desktop (session 0, a service, a SYSTEM task) is relaunched on the logged-in desktop when the caller holds `SeTcbPrivilege`. The secure desktop (a UAC prompt or the sign-in screen) and a host with no active session are refused. A session that only shows its lock screen is not detected: commands still run there, and clipboard commands time out until it is unlocked.
- Execution policy is read from `AGENT_DESKTOP_READ_ONLY`, `AGENT_DESKTOP_ALLOWED_COMMANDS` and `AGENT_DESKTOP_AUDIT_LOG` before any command reaches an app, and before a relaunch. Read-only mode refuses every command that changes state and keeps observation, including `screenshot PATH`.

Background clicks, OCR, minimized-window capture and session recovery are Windows-only. An app that ignores posted messages needs `--semantic` or a UIA ref action; a framework that reads the physical cursor position needs `--cursor-fallback`.

## Build on Windows

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1 -Release
```

Outputs:

- `bin\agent-desktop.exe`, `bin\agent-browser.exe` and `bin\agent-android.exe`: the root entry points. Point MCP clients and shell calls at these.
- `bin\skill-data\` and `bin\skills\`: a copy of the browser tool's skills. `agent-browser.exe` looks for them beside itself, and `agent-browser skills get core` fails without them.
- `agent-desktop\target\release\agent-desktop.exe`
- `agent-browser\cli\target\release\agent-browser.exe`, copied to `agent-browser\bin\agent-browser-win32-<arch>.exe` for the Node launcher
- `agent-android\target\release\agent-android.exe`

`bin\` is generated by the build script and ignored by git. Rebuild after pulling, after editing `agent-browser\skill-data\`, or after switching between debug and release.

Requirements: Rust 1.89 or newer, the MSVC toolchain, and Windows 10 or newer. Building the Node launcher and dashboard additionally needs Node 22 and pnpm 11 (`corepack pnpm install --frozen-lockfile`, then `pnpm build:native` inside `agent-browser/`). Running `agent-android` additionally needs Android platform-tools (`adb`) and a device with USB debugging or wireless ADB; see [agent-android/README.md](agent-android/README.md).

## MCP configuration

All three binaries serve stdio MCP:

```json
{
  "mcpServers": {
    "agent-desktop": {
      "command": "C:\\path\\to\\agent-mcp\\bin\\agent-desktop.exe",
      "args": ["mcp"]
    },
    "agent-browser": {
      "command": "C:\\path\\to\\agent-mcp\\bin\\agent-browser.exe",
      "args": ["mcp"]
    },
    "agent-android": {
      "command": "C:\\path\\to\\agent-mcp\\bin\\agent-android.exe",
      "args": ["mcp"]
    }
  }
}
```

`agent-browser mcp` serves its `core` tools profile by default, which keeps the tool list small. Use `"args": ["mcp", "--tools", "all"]` for every tool, or name profiles with commas, such as `core,network`.

## Repository rules

- Each subproject keeps its own `AGENTS.md`, `README.md`, license and build instructions. Those files describe that tool.
- Root `bin/` holds only build output: the three entry points and the copy of the browser skills. It is generated, ignored by git, and never edited by hand.
- Local changes to imported code are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
- Commits follow the desktop project's rules: Conventional Commits, lowercase after the colon, no AI attribution and no `Co-Authored-By` lines.
- Upstream snapshots are recorded by commit in this file and in `THIRD_PARTY_NOTICES.md`. An update re-imports the new upstream tree and reapplies the local patch; it does not merge upstream git history into this repository.
