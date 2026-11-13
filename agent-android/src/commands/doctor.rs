use crate::commands::Ctx;
use crate::error::Result;
use serde_json::{Value, json};

pub fn doctor(ctx: &Ctx<'_>) -> Result<Value> {
    ctx.adb.ensure_ready()?;
    let version = ctx.adb.version()?;
    let devices = ctx.adb.devices()?;
    let selected = ctx
        .adb
        .serial()
        .ok()
        .filter(|serial| {
            devices
                .iter()
                .any(|device| device.ready() && device.serial == *serial)
        })
        .map(|serial| serial.to_string());
    Ok(json!({
        "adb": ctx.adb.exe_display(),
        "version": version,
        "devices": devices,
        "selected": selected,
        "policy": {
            "read_only": ctx.policy.read_only,
            "allow_destructive": ctx.policy.allow_destructive,
        },
        "home": ctx.home.display().to_string(),
    }))
}
