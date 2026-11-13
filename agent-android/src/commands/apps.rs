use crate::adb::INSTALL_TIMEOUT;
use crate::args::{
    AppsArgs, ClearDataArgs, ConnectArgs, InstallArgs, LogcatArgs, OpenUrlArgs, UninstallArgs,
};
use crate::commands::Ctx;
use crate::error::{AppError, Result};
use serde_json::{Value, json};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const INSTALLED_HINT: &str =
    "Check the package name with android_list_apps; all=true includes system packages";
const PAIRED_NOTICE: &str = concat!(
    "Pairing does not connect. Check android_list_devices; if the device is missing, call ",
    "android_connect again without pair_code and with the connection port shown under ",
    "Wireless debugging, which differs from the pairing port."
);

pub fn list(ctx: &Ctx<'_>, args: &AppsArgs) -> Result<Value> {
    let mut command = vec!["pm", "list", "packages"];
    if !args.all {
        command.push("-3");
    }
    let output = ctx.adb.shell(&command)?;
    let packages = parse_packages(&output);
    Ok(json!({
        "packages": packages,
        "count": packages.len(),
        "include_system": args.all,
    }))
}

pub fn foreground(ctx: &Ctx<'_>) -> Result<Value> {
    let output = ctx.adb.shell(&["dumpsys", "activity", "activities"])?;
    let foreground = parse_foreground(&output).ok_or_else(|| {
        AppError::new(
            "UI_NOT_AVAILABLE",
            "Could not determine the foreground package from dumpsys",
        )
    })?;
    Ok(json!({
        "package": foreground.package,
        "activity": foreground.activity,
    }))
}

pub fn launch(ctx: &Ctx<'_>, package: &str) -> Result<Value> {
    ctx.check_write("launch")?;
    validate_package(package)?;
    let report = ctx.adb.shell_report(&[
        "monkey",
        "-p",
        package,
        "-c",
        "android.intent.category.LAUNCHER",
        "1",
    ])?;
    if report.text.contains("No activities found") {
        return Err(AppError::new(
            "APP_NOT_FOUND",
            format!("No launcher activity was found for {package}"),
        )
        .with_suggestion(INSTALLED_HINT));
    }
    report.check()?;
    Ok(json!({ "package": package, "launched": true }))
}

pub fn terminate(ctx: &Ctx<'_>, package: &str) -> Result<Value> {
    ctx.check_write("terminate")?;
    validate_package(package)?;
    ctx.adb.shell(&["am", "force-stop", package])?;
    Ok(json!({ "package": package, "terminated": true }))
}

pub fn install(ctx: &Ctx<'_>, args: &InstallArgs) -> Result<Value> {
    ctx.check_write("install")?;
    if !args.path.is_file() {
        return Err(AppError::invalid(format!(
            "APK not found: {}",
            args.path.display()
        )));
    }
    let path = args.path.display().to_string();
    if path.starts_with('-') {
        return Err(AppError::invalid("APK path must not start with a dash"));
    }
    validate_apk(&args.path)?;
    let mut command = vec!["install"];
    if args.replace {
        command.push("-r");
    }
    command.push(&path);
    let report = ctx.adb.device_report(&command, INSTALL_TIMEOUT)?;
    if let Some(line) = line_containing(&report.text, "Failure") {
        return Err(AppError::new("INSTALL_FAILED", line));
    }
    report.check()?;
    Ok(json!({
        "path": path,
        "replace": args.replace,
        "installed": true,
    }))
}

pub fn uninstall(ctx: &Ctx<'_>, args: &UninstallArgs) -> Result<Value> {
    ctx.check_destructive("uninstall", args.confirm)?;
    validate_package(&args.package)?;
    let mut command = vec!["uninstall"];
    if args.keep_data {
        command.push("-k");
    }
    command.push(&args.package);
    let report = ctx.adb.device_report(&command, INSTALL_TIMEOUT)?;
    if let Some(line) = line_containing(&report.text, "Failure") {
        return Err(AppError::new("UNINSTALL_FAILED", line).with_suggestion(INSTALLED_HINT));
    }
    report.check()?;
    Ok(json!({
        "package": args.package,
        "keep_data": args.keep_data,
        "uninstalled": true,
    }))
}

pub fn clear_data(ctx: &Ctx<'_>, args: &ClearDataArgs) -> Result<Value> {
    ctx.check_destructive("clear_data", args.confirm)?;
    validate_package(&args.package)?;
    let report = ctx.adb.shell_report(&["pm", "clear", &args.package])?;
    if !report.text.contains("Success") {
        return Err(AppError::new(
            "ACTION_FAILED",
            format!("pm clear did not report success: {}", report.text),
        )
        .with_suggestion(INSTALLED_HINT));
    }
    Ok(json!({ "package": args.package, "cleared": true }))
}

pub fn logcat(ctx: &Ctx<'_>, args: &LogcatArgs) -> Result<Value> {
    let limit = args.lines.clamp(1, 5000);
    let limit_text = limit.to_string();
    let output = ctx.adb.run_device(
        &["logcat", "-d", "-t", &limit_text],
        Duration::from_secs(30),
    )?;
    let lines = output
        .lines()
        .filter(|line| {
            args.filter
                .as_deref()
                .map(|filter| line.contains(filter))
                .unwrap_or(true)
        })
        .take(limit as usize)
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    Ok(json!({
        "count": lines.len(),
        "filter": args.filter,
        "lines": lines,
        "notice": "Logcat can contain personal data; keep it local",
    }))
}

pub fn open_url(ctx: &Ctx<'_>, args: &OpenUrlArgs) -> Result<Value> {
    ctx.check_write("open_url")?;
    validate_url(&args.url)?;
    let report = ctx.adb.shell_report(&[
        "am",
        "start",
        "-a",
        "android.intent.action.VIEW",
        "-d",
        &args.url,
    ])?;
    if let Some(line) = line_containing(&report.text, "Error:") {
        return Err(AppError::new("ACTION_FAILED", line));
    }
    report.check()?;
    Ok(json!({ "url": args.url, "opened": true }))
}

pub fn connect(ctx: &Ctx<'_>, args: &ConnectArgs) -> Result<Value> {
    ctx.check_write("connect")?;
    let host = args.host.trim();
    if host.is_empty()
        || !host.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '.' | ':' | '-' | '_' | '[' | ']')
        })
    {
        return Err(AppError::invalid(
            "host must be a hostname or host:port without whitespace",
        ));
    }
    let address = if host.contains(':') {
        host.to_string()
    } else {
        format!("{host}:{}", args.port.unwrap_or(5555))
    };
    if let Some(code) = &args.pair_code {
        let report = ctx
            .adb
            .host_report(&["pair", &address, code], CONNECT_TIMEOUT)?;
        if !report.text.contains("Successfully paired") {
            return Err(AppError::new("CONNECT_FAILED", report.text));
        }
        return Ok(json!({
            "address": address,
            "output": report.text,
            "paired": true,
            "connected": false,
            "notice": PAIRED_NOTICE,
        }));
    }
    let report = ctx
        .adb
        .host_report(&["connect", &address], CONNECT_TIMEOUT)?;
    if !report.text.contains("connected to") {
        return Err(AppError::new("CONNECT_FAILED", report.text));
    }
    Ok(json!({
        "address": address,
        "output": report.text,
        "paired": false,
        "connected": true,
    }))
}

fn line_containing<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    text.lines()
        .map(str::trim)
        .find(|line| line.contains(marker))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Foreground {
    pub package: String,
    pub activity: Option<String>,
}

pub fn parse_foreground(output: &str) -> Option<Foreground> {
    let mut best: Option<(u8, Foreground)> = None;
    for line in output.lines() {
        let rank = if line.contains("topResumedActivity") {
            0
        } else if line.contains("mResumedActivity") {
            1
        } else if line.contains("mCurrentFocus") {
            2
        } else {
            continue;
        };
        let Some(foreground) = parse_activity_line(line) else {
            continue;
        };
        if best.as_ref().is_none_or(|(best_rank, _)| rank < *best_rank) {
            best = Some((rank, foreground));
        }
    }
    best.map(|(_, foreground)| foreground)
}

fn parse_activity_line(line: &str) -> Option<Foreground> {
    for raw in line.split_whitespace() {
        let token =
            raw.trim_matches(|character| matches!(character, '{' | '}' | ',' | ';' | '(' | ')'));
        let Some((package, activity)) = token.split_once('/') else {
            continue;
        };
        if !is_package_name(package) {
            continue;
        }
        let activity = activity.trim_matches('}');
        let activity = if activity.starts_with('.') {
            format!("{package}{activity}")
        } else {
            activity.to_string()
        };
        return Some(Foreground {
            package: package.to_string(),
            activity: Some(activity).filter(|value| !value.is_empty()),
        });
    }
    None
}

pub fn parse_packages(output: &str) -> Vec<String> {
    let mut packages = output
        .lines()
        .filter_map(|line| line.trim().strip_prefix("package:"))
        .map(|package| package.trim().to_string())
        .filter(|package| !package.is_empty())
        .collect::<Vec<_>>();
    packages.sort();
    packages.dedup();
    packages
}

pub fn validate_package(package: &str) -> Result<()> {
    if !is_package_name(package) {
        return Err(AppError::invalid(format!(
            "Invalid Android package name: {package}"
        )));
    }
    Ok(())
}

fn is_package_name(package: &str) -> bool {
    package.contains('.')
        && !package.starts_with('.')
        && !package.ends_with('.')
        && package.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '.' || character == '_'
        })
}

fn validate_apk(path: &Path) -> Result<()> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !extension.eq_ignore_ascii_case("apk") {
        return Err(AppError::invalid("APK path must end in .apk"));
    }
    let mut file = File::open(path).map_err(|error| {
        AppError::invalid(format!("Could not read APK {}: {error}", path.display()))
    })?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|_| AppError::invalid(format!("APK is too short: {}", path.display())))?;
    if magic != [0x50, 0x4b, 0x03, 0x04] {
        return Err(AppError::invalid(format!(
            "Not an APK/ZIP file: {}",
            path.display()
        )));
    }
    Ok(())
}

fn validate_url(url: &str) -> Result<()> {
    let lower = url.to_ascii_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(AppError::invalid(
            "Only http:// and https:// URLs are allowed",
        ));
    }
    if url
        .chars()
        .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(AppError::invalid(
            "URL must not contain whitespace or control characters",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "apps_tests.rs"]
mod tests;
