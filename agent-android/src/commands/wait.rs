use crate::args::{SnapshotArgs, WaitArgs};
use crate::commands::{Ctx, apps, snapshot};
use crate::error::{AppError, Result};
use serde_json::{Value, json};
use std::thread;
use std::time::{Duration, Instant};

pub fn wait(ctx: &Ctx<'_>, args: &WaitArgs) -> Result<Value> {
    if args.text.is_none() && args.reference.is_none() && args.foreground.is_none() {
        return Err(AppError::invalid(
            "Wait needs one of text, ref or foreground",
        ));
    }
    let timeout = Duration::from_millis(args.timeout_ms.max(1));
    let interval = Duration::from_millis(args.interval_ms.max(50));
    let started = Instant::now();
    let mut last = json!(null);
    loop {
        match check_once(ctx, args) {
            Ok(Some(mut value)) => {
                value["elapsed_ms"] = json!(started.elapsed().as_millis() as u64);
                return Ok(value);
            }
            Ok(None) => {}
            Err(error) => {
                if is_fatal(&error) {
                    return Err(error);
                }
                last = error.to_value();
            }
        }
        if started.elapsed() >= timeout {
            return Err(AppError::new(
                "TIMEOUT",
                format!("Condition was not met within {} ms", args.timeout_ms.max(1)),
            )
            .with_details(json!({ "last": last })));
        }
        let remaining = timeout.saturating_sub(started.elapsed());
        thread::sleep(interval.min(remaining).max(Duration::from_millis(1)));
    }
}

fn check_once(ctx: &Ctx<'_>, args: &WaitArgs) -> Result<Option<Value>> {
    if let Some(package) = &args.foreground {
        let value = apps::foreground(ctx)?;
        let actual = value["package"].as_str().unwrap_or("");
        if actual == package {
            return Ok(Some(json!({
                "matched": "foreground",
                "package": actual,
            })));
        }
        return Ok(None);
    }

    let snapshot_args = SnapshotArgs {
        all: false,
        max_depth: None,
    };
    let output = snapshot::capture_once(ctx, &snapshot_args)?;
    let needle = args.text.as_ref().map(|text| text.to_lowercase());
    if let Some(needle) = &needle
        && !output
            .elements
            .iter()
            .any(|element| element_contains_text(element, needle))
    {
        return Ok(None);
    }
    if let Some(reference) = &args.reference
        && !output
            .elements
            .iter()
            .any(|element| element.reference == *reference)
    {
        return Ok(None);
    }
    ctx.store().save(&output)?;
    let matched_reference = args
        .reference
        .clone()
        .or_else(|| {
            let needle = needle.as_ref()?;
            output
                .elements
                .iter()
                .find(|element| element_contains_text(element, needle))
                .map(|element| element.reference.clone())
        })
        .unwrap_or_default();
    Ok(Some(json!({
        "matched": "element",
        "ref": matched_reference,
        "snapshot_id": output.snapshot_id,
        "package": output.package,
    })))
}

fn element_contains_text(element: &snapshot::UiElement, needle: &str) -> bool {
    [element.text.as_deref(), element.desc.as_deref()]
        .into_iter()
        .flatten()
        .any(|value| value.to_lowercase().contains(needle))
}

fn is_fatal(error: &AppError) -> bool {
    matches!(
        error.code,
        "ADB_NOT_FOUND"
            | "DEVICE_NOT_FOUND"
            | "DEVICE_NOT_READY"
            | "MULTIPLE_DEVICES"
            | "INVALID_ARGS"
            | "PERM_DENIED"
    )
}

#[cfg(test)]
#[path = "wait_tests.rs"]
mod tests;
