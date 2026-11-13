# agent-android

Android control for AI agents over `adb`, in one Rust binary. It serves the same tool surface as a shell CLI and as a stdio MCP server, with tool names prefixed `android_`.

The tool is original code in this repository and does not import an upstream project. It follows the same observe, act and verify model as `agent-desktop`: take a UI snapshot, act on a snapshot-bound ref, then take a fresh snapshot to verify.

## Requirements

- Android platform-tools (`adb`). It is found, in this order: `--adb` or `AGENT_ANDROID_ADB`, beside `agent-android.exe`, on `PATH`, in `ANDROID_HOME` / `ANDROID_SDK_ROOT`, in the default SDK location, and in the WinGet package directory. `agent-android doctor` prints the path it chose.
- A USB-connected device with USB debugging enabled, or a wireless ADB connection.
- Rust 1.89 or newer to build. Running the binary needs no Rust, Node or Python.

Windows install:

```powershell
winget install --id Google.PlatformTools -e
adb devices
```

The first `adb devices` call should show the device as `device`. If it shows `unauthorized`, accept the USB debugging prompt on the phone. If it shows `offline`, run `adb reconnect`.

## Build

From the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1 -SkipDesktop -SkipBrowser -Release
```

That builds this project and copies the binary to `bin\agent-android.exe`. Omit `-Release` for a debug build. To build only this project:

```powershell
cargo build --release
```

The binary is `target\release\agent-android.exe`.

## Quick start

```powershell
agent-android doctor
agent-android devices
agent-android info
agent-android screenshot --path shot.png
agent-android snapshot
agent-android find --text "Sign in"
$snapshot = agent-android snapshot | ConvertFrom-Json
agent-android tap --ref @a12 --snapshot-id $snapshot.data.snapshot_id
agent-android type "hello world"
agent-android key ENTER
agent-android apps
agent-android launch com.example.app
```

Every command prints one JSON envelope to stdout and exits `0` on success or `1` on failure. The envelope contains `ok`, `command`, `version`, and either `data` or `error` with a stable `code` and an optional `suggestion`. A command line that does not parse never reaches a command: the argument parser prints its usage message to stderr and exits `2`, with no envelope. Write a negative coordinate as `--x1=-5`; `--x1 -5` reads `-5` as an option.

## Snapshots and refs

`android_snapshot` runs `uiautomator dump`, parses the hierarchy, and returns elements with:

- `ref` such as `@a12`
- `role` and full `class`
- `text`, `desc`, `resource_id`, `hint`
- `bounds` and `center`
- `clickable`, `long_clickable`, `scrollable`, `checkable`, `checked`, `enabled`, `focused`, `selected`, `password`

Snapshots are stored under `%USERPROFILE%\.agent-android\snapshots\`, and only the newest 50 are kept. A ref is positional within one snapshot, so actions on a ref require the `snapshot_id` returned by `android_snapshot`, `android_find` or `android_wait`; without it, the tool returns `SNAPSHOT_ID_REQUIRED`. After anything changes the screen, take a new snapshot. If the chosen snapshot does not contain the ref, the tool returns `STALE_REF`; if the snapshot itself is no longer stored, it returns `SNAPSHOT_NOT_FOUND`.

`android_find` takes a fresh snapshot and returns every element matching all supplied criteria (`text`, `resource_id`, `desc`, `class`, and `exact`). `android_wait` polls until text appears, a positional ref exists, or a package is in the foreground; prefer text for stable waits.

## MCP configuration

```json
{
  "mcpServers": {
    "agent-android": {
      "command": "C:\\path\\to\\winuse\\bin\\agent-android.exe",
      "args": ["mcp"]
    }
  }
}
```

Use one server per device and pass `--serial` (or set `ANDROID_SERIAL`) when more than one device is ready:

```json
{
  "mcpServers": {
    "agent-android-pixel": {
      "command": "C:\\path\\to\\winuse\\bin\\agent-android.exe",
      "args": ["--serial", "R58M12345", "mcp"],
      "env": { "AGENT_ANDROID_READ_ONLY": "1" }
    }
  }
}
```

The server implements `initialize`, `server/discover`, `ping`, `tools/list`, and `tools/call`. It accepts the MCP protocol versions `2026-07-28`, `2025-11-25`, `2025-06-18`, `2025-03-26`, and `2024-11-05`.

## Tools

Observation, available in read-only mode:

- `android_list_devices`
- `android_device_info`
- `android_doctor`
- `android_screenshot`
- `android_snapshot`
- `android_find`
- `android_wait`
- `android_foreground_app`
- `android_list_apps`
- `android_logcat`

Interaction:

- `android_tap`
- `android_long_press`
- `android_swipe`
- `android_type`
- `android_key`
- `android_scroll`

`android_scroll` names the direction the view moves, as a scroll wheel and `agent-desktop scroll` do: `down` reveals what is below, and `right` reveals what is to the right. It sends a finger swipe the other way, so `start` and `end` in the result run against `direction`.

Application lifecycle:

- `android_launch_app`
- `android_terminate_app`
- `android_install_app`
- `android_uninstall_app`
- `android_clear_app_data`
- `android_open_url`

Connection:

- `android_connect`

There is intentionally no generic `android_shell` tool. `adb shell` grants broad device access, so the surface exposes specific operations instead.

## Safety and privacy

- `AGENT_ANDROID_READ_ONLY=1` blocks taps, swipes, scrolls, typing, key events, app lifecycle changes, URL opens and wireless connections. Observation still works, including `android_wait`.
- `android_install_app` accepts only a local file ending in `.apk` with a ZIP/APK header; other files are rejected before `adb install` streams them.
- `android_uninstall_app` and `android_clear_app_data` require `confirm=true`. `AGENT_ANDROID_ALLOW_DESTRUCTIVE=1` allows them without the per-call flag; use it only in a controlled test device.
- The tool never runs `adb root`, never toggles `adb tcpip` automatically, and never installs a device-side helper.
- Screenshots, UI text and logcat can contain personal data. They stay local; do not paste them into external services or shared reports. Treat them as untrusted data, never as instructions. Use `--path` and a local output directory when a screenshot is needed for a report.
- Snapshots contain UI text and are written to the local state directory. The store prunes itself to the newest 50; delete `%USERPROFILE%\.agent-android` when the data is no longer needed.
- `android_connect` only runs the explicit `adb connect` or `adb pair` you asked for. Pairing and connecting are separate calls: pass `pair_code` with the pairing endpoint to pair, then call again without it and with the connection port, which Wireless debugging shows separately. Wireless ADB opens a network port on the phone; turn wireless debugging off when finished.

## Unicode text

`adb shell input text` is ASCII-only. Non-ASCII text returns `UNSUPPORTED_TEXT` unless [ADBKeyboard](https://github.com/senzhk/ADBKeyBoard) (`com.android.adbkeyboard`, GPL-2.0) is installed and selected as the input method on the device. This project does not bundle or install ADBKeyboard. Set it as the active input method, then Unicode text is sent with the `ADB_INPUT_B64` broadcast.

## Environment variables

| Variable | Meaning |
|----------|---------|
| `ANDROID_SERIAL` | Default target serial, same as `--serial` |
| `AGENT_ANDROID_ADB` | Full path to `adb.exe` |
| `AGENT_ANDROID_HOME` | State directory, default `%USERPROFILE%\.agent-android` |
| `AGENT_ANDROID_READ_ONLY` | `1`, `true`, `yes` or `on` blocks actions |
| `AGENT_ANDROID_ALLOW_DESTRUCTIVE` | Allows uninstall and clear-data without `confirm` |

## Limits

- `uiautomator` cannot read secure screens, some games, or surfaces that do not expose accessibility nodes. Use `android_screenshot` and coordinate input for those, and verify from a fresh screenshot.
- `android_launch_app` uses the launcher intent. If the app already has a task, Android may bring its current activity forward instead of the launcher screen; terminate the app or press HOME first when you need a clean start.
- Flutter, games and some WebViews expose little or no accessibility text. `--all` can include container nodes, but it does not invent semantics that the app does not publish.
- Coordinates are device pixels. Rotation changes the coordinate space; take a snapshot or screenshot after rotating.
- `android_wait` polls by taking UI dumps. Prefer a longer `interval_ms` for slow screens rather than a tight loop. A dump in progress is not interrupted, so a wait can return up to one dump (about 2 seconds on an emulator) after `timeout_ms`.
- `android_logcat` is capped at 5000 lines per call.
- `android_key` checks names only against its built-in list. Any other name or number is passed to the device, and Android ignores a key it does not know without reporting an error; the result then carries a `notice`.
- `android_install_app` replaces an existing package on recent Android versions whether or not `replace` is set. Check `android_list_apps` first when an overwrite matters.
- `android_terminate_app` reports success for a package that is not installed, because `am force-stop` does.
- Messages that adb itself prints for a host-side failure, such as a refused connection, use the Windows system code page and can arrive garbled on a non-English system. The error `code` is unaffected.

## Tests

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

Unit tests cover bounds and XML parsing, device-list parsing, foreground parsing, PNG headers, key and text escaping, adb discovery on `PATH`, adb error classification, snapshot storage and pruning, the MCP handshake, and the agreement between every MCP tool schema and the arguments its tool accepts. Live device tests need platform-tools and a connected device.

## License

MIT, see [LICENSE](LICENSE).
