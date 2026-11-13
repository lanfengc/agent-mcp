use crate::args::{
    AppsArgs, ClearDataArgs, ConnectArgs, FindArgs, InstallArgs, KeyArgs, LogcatArgs,
    LongPressArgs, OpenUrlArgs, PackageArgs, ScreenshotArgs, ScrollArgs, SnapshotArgs, SwipeArgs,
    TapArgs, TypeArgs, UninstallArgs, WaitArgs,
};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "agent-android",
    version,
    about = "ADB-backed Android control CLI and stdio MCP server"
)]
pub struct Cli {
    /// Target device serial; defaults to the only ready device.
    #[arg(long, global = true, env = "ANDROID_SERIAL")]
    pub serial: Option<String>,
    /// Path to adb, overriding PATH and common SDK locations.
    #[arg(long, global = true, env = "AGENT_ANDROID_ADB")]
    pub adb: Option<PathBuf>,
    /// State directory for snapshots.
    #[arg(long, global = true, env = "AGENT_ANDROID_HOME")]
    pub home: Option<PathBuf>,
    /// Print diagnostics to stderr.
    #[arg(long, global = true)]
    pub verbose: bool,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// List connected Android devices.
    Devices,
    /// Show model, Android version, ABI and screen details.
    Info,
    /// Diagnose adb discovery and device state.
    Doctor,
    /// Capture a PNG screenshot.
    Screenshot(ScreenshotArgs),
    /// Dump the UI hierarchy and return snapshot refs.
    Snapshot(SnapshotArgs),
    /// Find elements in a fresh UI snapshot.
    Find(FindArgs),
    /// Tap by coordinates or snapshot ref.
    Tap(TapArgs),
    /// Long-press by coordinates or snapshot ref.
    LongPress(LongPressArgs),
    /// Swipe between two points.
    Swipe(SwipeArgs),
    /// Type text into the focused field.
    Type(TypeArgs),
    /// Send an Android key event.
    Key(KeyArgs),
    /// Scroll the screen or a scrollable ref.
    Scroll(ScrollArgs),
    /// Wait for text, a ref or a foreground package.
    Wait(WaitArgs),
    /// List installed packages.
    Apps(AppsArgs),
    /// Show the foreground package and activity.
    Foreground,
    /// Launch an app by package name.
    Launch(PackageArgs),
    /// Force-stop an app.
    Terminate(PackageArgs),
    /// Install an APK.
    Install(InstallArgs),
    /// Uninstall a package.
    Uninstall(UninstallArgs),
    /// Clear application data.
    ClearData(ClearDataArgs),
    /// Read recent logcat output.
    Logcat(LogcatArgs),
    /// Open a URL in the default handler.
    OpenUrl(OpenUrlArgs),
    /// Connect to a wireless ADB device.
    Connect(ConnectArgs),
    /// Serve the MCP protocol on stdin/stdout.
    Mcp,
}
