# agent-suite

Desktop and browser automation for AI agents, in one repository.

`agent-suite` combines two Apache-2.0 tools that an agent drives over MCP or the shell:

| Directory | Tool | Upstream snapshot | Local changes |
|-----------|------|-------------------|---------------|
| `agent-desktop/` | Native desktop apps through the OS accessibility tree, plus background window capture and posted clicks on Windows | [`vercel-labs/agent-desktop`](https://github.com/vercel-labs/agent-desktop) at `54855b6225aec00da4ac65e2d0362fd41cd9fcb3` (`main`, 2026-11-12) | Background window click, capture metadata, execution policy |
| `agent-browser/` | Web pages through Chrome DevTools Protocol, driven one command at a time | [`vercel-labs/agent-browser`](https://github.com/vercel-labs/agent-browser) at `2f1649cefb82f417abbb2712f5397439d010698e` (`main`, 2026-11-12) | None |

Both are tools, not agents. The observe, act and verify loop runs in the calling model.

## Why the two live together

A desktop test flow often crosses both surfaces: a native client window, a web view opened by that client, and a dialog between them. One checkout gives the agent both CLIs, one build script, and one place to record upstream imports and local patches.

The two projects keep their own build systems. `agent-desktop` is a Cargo workspace; `agent-browser` is a pnpm workspace whose CLI is a separate Cargo package under `cli/`. Nothing is unified at the Cargo or pnpm level, so each upstream can be re-imported without untangling the other.

## What the local desktop patch adds

The desktop tool follows the project's own model: the agent takes a window screenshot, a vision model returns a pixel coordinate, and the tool clicks that pixel in the background without moving the cursor or changing the foreground window.

- `screenshot --window-id <id>` adds `capture_id`, `window_id`, `window_bounds`, `capture_bounds` and `capture_time_ms`.
- `mouse-click --background --window-id <id> --xy X,Y` posts `WM_MOUSEMOVE`, button down and button up to the target window with `PostMessageW`; `--capture-id <id>` interprets `X,Y` as image pixels and maps them through `capture_bounds`.
- The result is always `delivered_unverified` with `retry: unsafe`: the messages were accepted, not proven to have changed the app. Verify from fresh state, do not retry blindly.
- A capture older than `AGENT_DESKTOP_MAX_CAPTURE_AGE_MS` (default 10 s), or one whose window moved, resized or restarted, fails with `STALE_REF` and `details.kind: "stale_capture"`.
- Execution policy is read from `AGENT_DESKTOP_READ_ONLY`, `AGENT_DESKTOP_ALLOWED_COMMANDS` and `AGENT_DESKTOP_AUDIT_LOG` before any command reaches an app.

Background click is Windows-only. Windows apps that ignore posted messages, including some WinForms controls that read the physical cursor position, need a UIA ref action instead.

## Build on Windows

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1 -Release
```

Outputs:

- `agent-desktop\target\release\agent-desktop.exe`
- `agent-browser\cli\target\release\agent-browser.exe`, copied to `agent-browser\bin\agent-browser-win32-<arch>.exe` for the Node launcher

Requirements: Rust 1.89 or newer, the MSVC toolchain, and Windows 10 or newer. Building the Node launcher and dashboard additionally needs Node 22 and pnpm 11 (`corepack pnpm install --frozen-lockfile`, then `pnpm build:native` inside `agent-browser/`).

## MCP configuration

Both binaries serve stdio MCP:

```json
{
  "mcpServers": {
    "agent-desktop": {
      "command": "C:\\path\\to\\agent-suite\\agent-desktop\\target\\release\\agent-desktop.exe",
      "args": ["mcp"]
    },
    "agent-browser": {
      "command": "C:\\path\\to\\agent-suite\\agent-browser\\cli\\target\\release\\agent-browser.exe",
      "args": ["mcp"]
    }
  }
}
```

## Repository rules

- Each subproject keeps its own `AGENTS.md`, `README.md`, license and build instructions. Those files describe that tool.
- Local changes to imported code are listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
- Commits follow the desktop project's rules: Conventional Commits, lowercase after the colon, no AI attribution and no `Co-Authored-By` lines.
- Upstream snapshots are recorded by commit in this file and in `THIRD_PARTY_NOTICES.md`. An update re-imports the new upstream tree and reapplies the local patch; it does not merge upstream git history into this repository.
