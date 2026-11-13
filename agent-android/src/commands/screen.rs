use crate::args::ScreenshotArgs;
use crate::commands::Ctx;
use crate::error::{AppError, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

pub struct ScreenshotOutput {
    pub data: Value,
    pub png: Option<Vec<u8>>,
}

pub fn screenshot(ctx: &Ctx<'_>, args: &ScreenshotArgs) -> Result<ScreenshotOutput> {
    if let Some(path) = &args.path {
        ensure_local_path(path)?;
    }
    let bytes = ctx.adb.exec_out(&["screencap", "-p"])?;
    let (width, height) = png_dimensions(&bytes).ok_or_else(|| {
        AppError::new(
            "SCREENSHOT_FAILED",
            "adb screencap did not return a PNG image",
        )
        .with_details(json!({ "bytes": bytes.len() }))
    })?;
    if let Some(path) = &args.path {
        std::fs::write(path, &bytes).map_err(|error| {
            AppError::new(
                "SCREENSHOT_FAILED",
                format!("Could not write {}: {error}", path.display()),
            )
        })?;
        return Ok(ScreenshotOutput {
            data: json!({
                "path": path.display().to_string(),
                "width": width,
                "height": height,
                "bytes": bytes.len(),
            }),
            png: None,
        });
    }
    Ok(ScreenshotOutput {
        data: json!({
            "mime_type": "image/png",
            "width": width,
            "height": height,
            "bytes": bytes.len(),
            "base64": STANDARD.encode(&bytes),
        }),
        png: Some(bytes),
    })
}

pub fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if bytes.len() < 24 || bytes[..8] != SIGNATURE || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}

fn ensure_local_path(path: &std::path::Path) -> Result<()> {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        if let Some(Component::Prefix(prefix)) = path.components().next()
            && matches!(
                prefix.kind(),
                Prefix::UNC(..) | Prefix::VerbatimUNC(..) | Prefix::DeviceNS(..)
            )
        {
            return Err(AppError::new(
                "PERM_DENIED",
                "Screenshot path must be on a local drive, not a network share",
            ));
        }
    }
    let _ = path;
    Ok(())
}

#[cfg(test)]
#[path = "screen_tests.rs"]
mod tests;
