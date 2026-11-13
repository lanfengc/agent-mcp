# FFI Python smoke harness

Proves the C ABI works from a non-Rust host (Python ctypes) and gates it in CI.

## What it tests

| Leg | Check |
|-----|-------|
| 1 | `ad_abi_version()` equals `AD_ABI_VERSION_MAJOR` from the header. Catches a header and DLL mismatch at import. |
| 2 | Every `ad_*_size()` getter equals its `AD_*_SIZE` macro. Catches struct layout drift. |
| 3 | `ad_version()` returns `ok:true` JSON with `data.version`. |
| 4 | `ad_adapter_create` → `ad_snapshot` → `ad_free_string` → `ad_adapter_destroy` runs without a crash or leak. A stub adapter returns a clean `PLATFORM_NOT_SUPPORTED` envelope. |
| 5 | Opt-in with `AD_EXPECT_REAL=1` (optional `AD_APP`, default `notepad.exe`). The real adapter returns a complete snapshot with refs for the app, on the main thread and on a worker thread. |

## Building the DLL

```bash
# From the repo root:
cargo build --locked --profile release-ffi -p agent-desktop-ffi --features stub-adapter
```

The DLL lands at `target/release-ffi/agent_desktop_ffi.dll`.

Use `--profile release-ffi`. The default `release` profile sets `panic = "abort"`, which silently disables the FFI's `catch_unwind` traps.

`--features stub-adapter` replaces the real adapter with one that returns `PLATFORM_NOT_SUPPORTED` for every call. CI uses this build, so the harness runs without a desktop.

## Running locally

```bash
python tests/ffi-python/smoke.py \
  target/release-ffi/agent_desktop_ffi.dll \
  crates/ffi/include/agent_desktop.h
```

Or set environment variables:

```bash
AD_DYLIB_PATH=target/release-ffi/agent_desktop_ffi.dll \
AD_HEADER_PATH=crates/ffi/include/agent_desktop.h \
python tests/ffi-python/smoke.py
```

## Stub enforcement (`AD_EXPECT_STUB=1`)

Set `AD_EXPECT_STUB=1` when you run the harness against a stub build. The harness then fails if `ad_snapshot()` returns `ok:true`, which would mean the DLL was not built with `--features stub-adapter`. A real-adapter DLL therefore cannot pass the stub gate.

## Dependencies

`smoke.py` uses only the Python standard library, so no `pip install` is needed.

## Real-adapter happy path

Legs 1 to 4 cover the surface that needs no desktop: the ABI version, struct sizes, `ad_version`, and the `PLATFORM_NOT_SUPPORTED` path. Leg 5 covers a successful real-adapter snapshot, and the Windows E2E harness covers the CLI against real apps.
