use crate::commands::{Ctx, device};
use crate::error::{AppError, Result};
use serde_json::{Value, json};

pub fn list(ctx: &Ctx<'_>) -> Result<Value> {
    let devices = ctx.adb.devices()?;
    let ready = devices.iter().filter(|device| device.ready()).count();
    Ok(json!({
        "devices": devices,
        "ready_count": ready,
    }))
}

pub fn info(ctx: &Ctx<'_>) -> Result<Value> {
    let serial = ctx.adb.serial()?.to_string();
    let props = ctx.adb.shell(&["getprop"])?;
    let (width, height) = device::screen_size(ctx)?;
    let density = device::screen_density(ctx).ok();
    Ok(json!({
        "serial": serial,
        "manufacturer": prop(&props, "ro.product.manufacturer"),
        "model": prop(&props, "ro.product.model"),
        "android": prop(&props, "ro.build.version.release"),
        "sdk": prop(&props, "ro.build.version.sdk"),
        "abi": prop(&props, "ro.product.cpu.abi"),
        "screen": { "width": width, "height": height, "density": density },
    }))
}

pub fn screen_size(ctx: &Ctx<'_>) -> Result<(u32, u32)> {
    let output = ctx.adb.shell(&["wm", "size"])?;
    parse_wm_size(&output).ok_or_else(|| {
        AppError::new(
            "DEVICE_INFO_FAILED",
            format!("Could not read screen size: {output}"),
        )
    })
}

pub fn screen_density(ctx: &Ctx<'_>) -> Result<u32> {
    let output = ctx.adb.shell(&["wm", "density"])?;
    parse_wm_density(&output).ok_or_else(|| {
        AppError::new(
            "DEVICE_INFO_FAILED",
            format!("Could not read screen density: {output}"),
        )
    })
}

pub fn prop(output: &str, key: &str) -> Option<String> {
    let prefix = format!("[{key}]: [");
    output.lines().find_map(|line| {
        line.trim()
            .strip_prefix(&prefix)
            .and_then(|value| value.strip_suffix(']'))
            .map(|value| value.to_string())
    })
}

pub fn parse_wm_size(output: &str) -> Option<(u32, u32)> {
    let mut physical = None;
    let mut override_size = None;
    for line in output.lines() {
        let lower = line.to_ascii_lowercase();
        let Some((_, value)) = line.split_once(':') else {
            continue;
        };
        let Some(size) = parse_dimensions(value.trim()) else {
            continue;
        };
        if lower.contains("override") {
            override_size = Some(size);
        } else if lower.contains("physical") {
            physical = Some(size);
        }
    }
    override_size.or(physical)
}

pub fn parse_dimensions(value: &str) -> Option<(u32, u32)> {
    let (width, height) = value.split_once('x')?;
    Some((width.trim().parse().ok()?, height.trim().parse().ok()?))
}

pub fn parse_wm_density(output: &str) -> Option<u32> {
    output.lines().rev().find_map(|line| {
        let (_, value) = line.split_once(':')?;
        value.split_whitespace().next()?.parse::<u32>().ok()
    })
}

#[cfg(test)]
#[path = "device_tests.rs"]
mod tests;
