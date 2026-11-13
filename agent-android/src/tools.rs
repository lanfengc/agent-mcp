use crate::args::{
    AppsArgs, ClearDataArgs, ConnectArgs, FindArgs, InstallArgs, KeyArgs, LogcatArgs,
    LongPressArgs, OpenUrlArgs, PackageArgs, ScreenshotArgs, ScrollArgs, SnapshotArgs, SwipeArgs,
    TapArgs, TypeArgs, UninstallArgs, WaitArgs,
};
use crate::commands::{Ctx, apps, device, doctor, input, screen, snapshot, wait};
use crate::error::{AppError, Result};
use serde::de::DeserializeOwned;
use serde_json::Value;

pub struct ToolResult {
    pub command: &'static str,
    pub data: Value,
    pub image: Option<Vec<u8>>,
}

pub struct ToolFailure {
    pub command: String,
    pub error: AppError,
}

pub type ToolOutcome = std::result::Result<ToolResult, ToolFailure>;

pub fn call(name: &str, arguments: Value, ctx: &Ctx<'_>) -> ToolOutcome {
    match name {
        "android_list_devices" => run("devices", arguments, |_: EmptyArgs| device::list(ctx)),
        "android_device_info" => run("info", arguments, |_: EmptyArgs| device::info(ctx)),
        "android_doctor" => run("doctor", arguments, |_: EmptyArgs| doctor::doctor(ctx)),
        "android_connect" => run("connect", arguments, |args: ConnectArgs| {
            apps::connect(ctx, &args)
        }),
        "android_screenshot" => outcome(
            "screenshot",
            parse(arguments).and_then(|args: ScreenshotArgs| screenshot(ctx, &args)),
        ),
        "android_snapshot" => run("snapshot", arguments, |args: SnapshotArgs| {
            snapshot::snapshot(ctx, &args)
        }),
        "android_find" => run("find", arguments, |args: FindArgs| {
            snapshot::find(ctx, &args)
        }),
        "android_foreground_app" => run("foreground", arguments, |_: EmptyArgs| {
            apps::foreground(ctx)
        }),
        "android_list_apps" => run("apps", arguments, |args: AppsArgs| apps::list(ctx, &args)),
        "android_logcat" => run("logcat", arguments, |args: LogcatArgs| {
            apps::logcat(ctx, &args)
        }),
        "android_tap" => run("tap", arguments, |args: TapArgs| input::tap(ctx, &args)),
        "android_long_press" => run("long_press", arguments, |args: LongPressArgs| {
            input::long_press(ctx, &args)
        }),
        "android_swipe" => run("swipe", arguments, |args: SwipeArgs| {
            input::swipe(ctx, &args)
        }),
        "android_type" => run("type", arguments, |args: TypeArgs| {
            input::type_text(ctx, &args)
        }),
        "android_key" => run("key", arguments, |args: KeyArgs| input::key(ctx, &args)),
        "android_scroll" => run("scroll", arguments, |args: ScrollArgs| {
            input::scroll(ctx, &args)
        }),
        "android_wait" => run("wait", arguments, |args: WaitArgs| wait::wait(ctx, &args)),
        "android_launch_app" => run("launch", arguments, |args: PackageArgs| {
            apps::launch(ctx, &args.package)
        }),
        "android_terminate_app" => run("terminate", arguments, |args: PackageArgs| {
            apps::terminate(ctx, &args.package)
        }),
        "android_install_app" => run("install", arguments, |args: InstallArgs| {
            apps::install(ctx, &args)
        }),
        "android_uninstall_app" => run("uninstall", arguments, |args: UninstallArgs| {
            apps::uninstall(ctx, &args)
        }),
        "android_clear_app_data" => run("clear_data", arguments, |args: ClearDataArgs| {
            apps::clear_data(ctx, &args)
        }),
        "android_open_url" => run("open_url", arguments, |args: OpenUrlArgs| {
            apps::open_url(ctx, &args)
        }),
        _ => Err(ToolFailure {
            command: name.strip_prefix("android_").unwrap_or(name).to_string(),
            error: AppError::new("UNKNOWN_TOOL", format!("Unknown tool: {name}")),
        }),
    }
}

fn run<T, F>(command: &'static str, arguments: Value, action: F) -> ToolOutcome
where
    T: DeserializeOwned,
    F: FnOnce(T) -> Result<Value>,
{
    outcome(
        command,
        parse(arguments).and_then(action).map(|data| (data, None)),
    )
}

fn outcome(command: &'static str, result: Result<(Value, Option<Vec<u8>>)>) -> ToolOutcome {
    match result {
        Ok((data, image)) => Ok(ToolResult {
            command,
            data,
            image,
        }),
        Err(error) => Err(ToolFailure {
            command: command.to_string(),
            error,
        }),
    }
}

fn screenshot(ctx: &Ctx<'_>, args: &ScreenshotArgs) -> Result<(Value, Option<Vec<u8>>)> {
    let output = screen::screenshot(ctx, args)?;
    let mut data = output.data;
    if output.png.is_some()
        && let Some(object) = data.as_object_mut()
    {
        object.remove("base64");
    }
    Ok((data, output.png))
}

fn parse<T: DeserializeOwned>(arguments: Value) -> Result<T> {
    serde_json::from_value(arguments)
        .map_err(|error| AppError::invalid(format!("Invalid tool arguments: {error}")))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyArgs {}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
