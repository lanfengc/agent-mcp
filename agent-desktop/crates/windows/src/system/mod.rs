mod adapter;
pub(crate) mod app_ops;
pub(crate) mod capture;
pub(crate) mod close;
#[cfg(target_os = "windows")]
pub(crate) mod close_dispatch;
pub(crate) mod close_outcome;
pub(crate) mod cursor_overlay;
pub(crate) mod display;
pub(crate) mod fixture;
#[cfg(target_os = "windows")]
pub(crate) mod interaction_lease;
pub(crate) mod key_dispatch;
pub(crate) mod launch;
mod launch_path;
pub(crate) mod lifecycle_envelope_parity;
#[cfg(all(test, target_os = "windows"))]
pub(crate) mod lifecycle_test_support;
pub(crate) mod menu;
#[cfg(target_os = "windows")]
pub(crate) mod ocr;
pub(crate) mod permissions;
#[cfg(target_os = "windows")]
pub(crate) mod private_file;
pub(crate) mod process_identity;
pub(crate) mod process_state;
pub(crate) mod runtime;
#[cfg(target_os = "windows")]
pub(crate) mod session_probe;
#[cfg(target_os = "windows")]
pub(crate) mod session_relaunch;
pub(crate) mod shell;
pub(crate) mod signal;
pub(crate) mod wait;
pub(crate) mod window;
