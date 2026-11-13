# Installing agent-desktop

agent-desktop runs on Windows x64 and ARM64. Install the `agent-desktop` npm package with npm, pnpm, Yarn, or Bun. It needs Node 18 or later and bundles the native executable for Windows:

```bash
npm install -g agent-desktop
agent-desktop version
```

Without `-g`, the command goes to `./node_modules/.bin` instead of your PATH. Run it with `npx agent-desktop`, or install globally.

## Older installations

If an older installation shadows the command, uninstall it through its original package manager. Check resolution with `Get-Command agent-desktop`.

## Blocked install scripts

The npm package's `postinstall` script replaces the Node launcher with a link to the native binary. Some package managers block install scripts, and npm 12 does by default. Then every call runs through the Node launcher, which adds about 125 ms. The launcher prints a one-time notice to stderr. Allow the script and reinstall:

```bash
npm install -g agent-desktop --allow-scripts=agent-desktop
```

For a project install, run `npm install-scripts approve agent-desktop` inside the project, then `npm rebuild agent-desktop`. The approve command only edits the project's `package.json` and fails for a global install.

## Release archives

Download the `agent-desktop-windows-<arch>` archive from [GitHub Releases](https://github.com/vercel-labs/agent-desktop/releases). Extract `agent-desktop.exe` into one directory on your PATH. To upgrade, replace the executable with the one from the same release.

The release's `checksums.txt` catches corrupted downloads but does not prove where the archive came from. Verify its origin with the build attestation:

```bash
gh attestation verify <downloaded-archive> --repo vercel-labs/agent-desktop
```

## From source

You need the Rust toolchain pinned in `rust-toolchain.toml` (1.89.0). `rustup` installs it on first use. You also need the MSVC build tools.

```bash
git clone https://github.com/vercel-labs/agent-desktop && cd agent-desktop
cargo build --release -p agent-desktop
```

The executable is written to `target/release/agent-desktop.exe`.

## Permissions

Windows needs no grant for apps at your terminal's integrity level. An app running as administrator can be read, but only an elevated agent can send it input. The binary is unsigned. SmartScreen may warn on a browser download, and antivirus may quarantine it. On a clean Windows 11 install, Smart App Control blocks it, and no download method or launch mode avoids that. Turn Smart App Control off to run it. npm installs do not trigger SmartScreen. To restore a quarantined file, first check its SHA-256 checksum against the release.

## Platform notes

`list-surfaces` lists each process's `window`, `focused`, `sheet`, and `menu` surfaces. `snapshot --surface` also reads `taskbar`, `system-tray`, `system-tray-overflow`, `start-menu`, and `action-center` without `--app`. `quick-settings` returns `PLATFORM_NOT_SUPPORTED` and names `action-center` as the surface that carries those controls.

`launch` takes an absolute path, or a bare name under System32 or the Windows directory, with or without `.exe`. It does not resolve a display name such as "Google Chrome". Use `list-apps` to find a running app.
