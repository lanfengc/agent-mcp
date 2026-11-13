use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScreenshotArgs {
    /// Write the PNG to this path instead of returning base64 data.
    #[arg(long, value_name = "PATH")]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotArgs {
    /// Include every node, not only actionable or labelled ones.
    #[arg(long)]
    #[serde(default)]
    pub all: bool,
    /// Limit traversal depth; 0 keeps only the root node, omit for no limit.
    #[arg(long)]
    pub max_depth: Option<u32>,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindArgs {
    /// Match text or content description.
    #[arg(long)]
    pub text: Option<String>,
    /// Match resource id (for example com.example:id/button).
    #[arg(long)]
    pub resource_id: Option<String>,
    /// Match content description only.
    #[arg(long)]
    pub desc: Option<String>,
    /// Match the widget class or its short role, for example Button.
    #[arg(long)]
    pub class: Option<String>,
    /// Require an exact, case-sensitive match.
    #[arg(long)]
    #[serde(default)]
    pub exact: bool,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TapArgs {
    /// X coordinate in device pixels.
    #[arg(long)]
    pub x: Option<i32>,
    /// Y coordinate in device pixels.
    #[arg(long)]
    pub y: Option<i32>,
    /// Element ref from a snapshot, for example @a12; requires --snapshot-id.
    #[arg(long = "ref")]
    #[serde(rename = "ref")]
    pub reference: Option<String>,
    /// Snapshot id that produced the ref; required with --ref.
    #[arg(long)]
    pub snapshot_id: Option<String>,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LongPressArgs {
    /// X coordinate in device pixels.
    #[arg(long)]
    pub x: Option<i32>,
    /// Y coordinate in device pixels.
    #[arg(long)]
    pub y: Option<i32>,
    /// Element ref from a snapshot, for example @a12; requires --snapshot-id.
    #[arg(long = "ref")]
    #[serde(rename = "ref")]
    pub reference: Option<String>,
    /// Snapshot id that produced the ref; required with --ref.
    #[arg(long)]
    pub snapshot_id: Option<String>,
    /// How long to hold the press, in milliseconds; at least 50.
    #[arg(long, default_value_t = 600)]
    #[serde(default = "default_long_press_ms")]
    pub duration_ms: u64,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwipeArgs {
    /// Start X coordinate in device pixels.
    #[arg(long)]
    pub x1: i32,
    /// Start Y coordinate in device pixels.
    #[arg(long)]
    pub y1: i32,
    /// End X coordinate in device pixels.
    #[arg(long)]
    pub x2: i32,
    /// End Y coordinate in device pixels.
    #[arg(long)]
    pub y2: i32,
    /// How long the swipe takes, in milliseconds.
    #[arg(long, default_value_t = 300)]
    #[serde(default = "default_swipe_ms")]
    pub duration_ms: u64,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeArgs {
    /// Text to type. ASCII only unless ADBKeyboard is the selected input method.
    #[arg(value_name = "TEXT", allow_hyphen_values = true)]
    pub text: String,
    /// Tap this snapshot ref before typing.
    #[arg(long = "ref")]
    #[serde(rename = "ref")]
    pub reference: Option<String>,
    /// Snapshot id that produced the ref; required with --ref.
    #[arg(long)]
    pub snapshot_id: Option<String>,
    /// Clear the focused field before typing.
    #[arg(long)]
    #[serde(default)]
    pub clear: bool,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyArgs {
    /// Android key name or number, for example ENTER, BACK or 66.
    #[arg(value_name = "KEY")]
    pub key: String,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScrollArgs {
    /// Direction the view moves: up, down, left or right. `down` reveals what is below, as a scroll wheel does.
    #[arg(value_name = "DIRECTION")]
    pub direction: String,
    /// Optional swipe distance in pixels.
    #[arg(long)]
    pub distance: Option<u32>,
    /// Scroll inside this snapshot ref instead of the whole screen.
    #[arg(long = "ref")]
    #[serde(rename = "ref")]
    pub reference: Option<String>,
    /// Snapshot id that produced the ref; required with --ref.
    #[arg(long)]
    pub snapshot_id: Option<String>,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitArgs {
    /// Wait until an element contains this text.
    #[arg(long)]
    pub text: Option<String>,
    /// Wait until this positional ref exists in a fresh snapshot; prefer text.
    #[arg(long = "ref")]
    #[serde(rename = "ref")]
    pub reference: Option<String>,
    /// Wait until this package is in the foreground.
    #[arg(long)]
    pub foreground: Option<String>,
    /// Give up after this many milliseconds.
    #[arg(long, default_value_t = 10000)]
    #[serde(default = "default_wait_timeout")]
    pub timeout_ms: u64,
    /// Pause between checks, in milliseconds; at least 50.
    #[arg(long, default_value_t = 300)]
    #[serde(default = "default_wait_interval")]
    pub interval_ms: u64,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppsArgs {
    /// Include system packages.
    #[arg(long)]
    #[serde(default)]
    pub all: bool,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageArgs {
    /// Application package name, for example com.android.settings.
    #[arg(value_name = "PACKAGE")]
    pub package: String,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallArgs {
    /// Path to an APK file on this computer.
    #[arg(value_name = "APK")]
    pub path: PathBuf,
    /// Pass `-r`, which older Android versions need to install over an existing package.
    #[arg(long)]
    #[serde(default)]
    pub replace: bool,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UninstallArgs {
    /// Application package name to remove.
    #[arg(value_name = "PACKAGE")]
    pub package: String,
    /// Keep application data and cache directories.
    #[arg(long)]
    #[serde(default)]
    pub keep_data: bool,
    /// Confirm this destructive action.
    #[arg(long)]
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClearDataArgs {
    /// Application package name whose data is erased.
    #[arg(value_name = "PACKAGE")]
    pub package: String,
    /// Confirm this destructive action.
    #[arg(long)]
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogcatArgs {
    /// Number of recent lines to read, from 1 to 5000.
    #[arg(long, default_value_t = 200)]
    #[serde(default = "default_logcat_lines")]
    pub lines: u32,
    /// Only return lines containing this text.
    #[arg(long)]
    pub filter: Option<String>,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenUrlArgs {
    /// An http or https URL.
    #[arg(value_name = "URL")]
    pub url: String,
}

#[derive(Debug, Clone, clap::Args, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectArgs {
    /// Device host or host:port for wireless ADB.
    #[arg(value_name = "HOST")]
    pub host: String,
    /// Port to use when host has no port; defaults to 5555.
    #[arg(long)]
    pub port: Option<u16>,
    /// Pairing code for `adb pair`; give the pairing endpoint as host:port. Pairs only: connect afterwards without this option.
    #[arg(long)]
    pub pair_code: Option<String>,
}

fn default_long_press_ms() -> u64 {
    600
}

fn default_swipe_ms() -> u64 {
    300
}

fn default_wait_timeout() -> u64 {
    10000
}

fn default_wait_interval() -> u64 {
    300
}

fn default_logcat_lines() -> u32 {
    200
}

#[cfg(test)]
#[path = "args_tests.rs"]
mod tests;
