use crate::args::{FindArgs, SnapshotArgs};
use crate::bounds::{Bounds, Point};
use crate::commands::snapshot_parse::{extract_xml, parse_ui};
use crate::commands::{Ctx, device};
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_DUMP_ATTEMPTS: usize = 10;
const DUMP_RETRY_DELAY: Duration = Duration::from_millis(300);
const DUMP_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotOutput {
    pub snapshot_id: String,
    pub package: String,
    pub screen: ScreenSize,
    pub elements: Vec<UiElement>,
    pub node_count: usize,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ScreenSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElement {
    #[serde(rename = "ref")]
    pub reference: String,
    pub role: String,
    pub class: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    pub bounds: Bounds,
    pub center: Point,
    #[serde(flatten)]
    pub flags: UiFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiFlags {
    pub clickable: bool,
    pub long_clickable: bool,
    pub scrollable: bool,
    pub checkable: bool,
    pub checked: bool,
    pub enabled: bool,
    pub focused: bool,
    pub selected: bool,
    pub password: bool,
}

impl Default for UiFlags {
    fn default() -> Self {
        Self {
            clickable: false,
            long_clickable: false,
            scrollable: false,
            checkable: false,
            checked: false,
            enabled: true,
            focused: false,
            selected: false,
            password: false,
        }
    }
}

pub fn snapshot(ctx: &Ctx<'_>, args: &SnapshotArgs) -> Result<Value> {
    let output = capture(ctx, args)?;
    serde_json::to_value(output).map_err(|error| {
        AppError::new(
            "SERIALIZE_FAILED",
            format!("Could not serialize snapshot: {error}"),
        )
    })
}

pub fn find(ctx: &Ctx<'_>, args: &FindArgs) -> Result<Value> {
    if args.text.is_none()
        && args.resource_id.is_none()
        && args.desc.is_none()
        && args.class.is_none()
    {
        return Err(AppError::invalid(
            "Find needs at least one of text, resource_id, desc or class",
        ));
    }
    let snapshot_args = SnapshotArgs {
        all: false,
        max_depth: None,
    };
    let output = capture(ctx, &snapshot_args)?;
    let matches = output
        .elements
        .iter()
        .filter(|element| element_matches(element, args))
        .collect::<Vec<_>>();
    Ok(json!({
        "snapshot_id": output.snapshot_id,
        "package": output.package,
        "count": matches.len(),
        "matches": matches,
    }))
}

pub fn resolve_ref(
    ctx: &Ctx<'_>,
    reference: &str,
    snapshot_id: Option<&str>,
) -> Result<(Point, String)> {
    let (element, snapshot_id) = resolve_element(ctx, reference, snapshot_id)?;
    Ok((element.center, snapshot_id))
}

pub fn resolve_element(
    ctx: &Ctx<'_>,
    reference: &str,
    snapshot_id: Option<&str>,
) -> Result<(UiElement, String)> {
    let snapshot_id = snapshot_id.ok_or_else(AppError::snapshot_id_required)?;
    let output = ctx.store().load(snapshot_id)?;
    let element = output
        .elements
        .into_iter()
        .find(|element| element.reference == reference)
        .ok_or_else(|| AppError::stale_ref(reference, snapshot_id))?;
    if element.bounds.is_empty() {
        return Err(AppError::invalid(format!(
            "Snapshot ref {reference} has empty bounds, so it cannot be targeted; take a new snapshot"
        )));
    }
    Ok((element, output.snapshot_id))
}

pub fn capture(ctx: &Ctx<'_>, args: &SnapshotArgs) -> Result<SnapshotOutput> {
    let output = read_ui(ctx, args, MAX_DUMP_ATTEMPTS)?;
    ctx.store().save(&output)?;
    Ok(output)
}

pub fn capture_once(ctx: &Ctx<'_>, args: &SnapshotArgs) -> Result<SnapshotOutput> {
    read_ui(ctx, args, 1)
}

fn read_ui(ctx: &Ctx<'_>, args: &SnapshotArgs, attempts: usize) -> Result<SnapshotOutput> {
    let xml = dump_ui(ctx, attempts)?;
    let parsed = parse_ui(&xml, args.max_depth, args.all)?;
    let screen = if parsed.screen.width > 0 && parsed.screen.height > 0 {
        parsed.screen
    } else {
        device::screen_size(ctx)
            .map(|(width, height)| ScreenSize { width, height })
            .unwrap_or_default()
    };
    Ok(SnapshotOutput {
        snapshot_id: next_snapshot_id(),
        package: parsed.package,
        screen,
        elements: parsed.elements,
        node_count: parsed.node_count,
    })
}

fn dump_ui(ctx: &Ctx<'_>, attempts: usize) -> Result<String> {
    let mut last_detail = String::new();
    for attempt in 0..attempts {
        let output = ctx
            .adb
            .exec_out_output(&["uiautomator", "dump", "/dev/tty"], DUMP_TIMEOUT)?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Some(xml) = extract_xml(&stdout).or_else(|| extract_xml(&stderr)) {
            return Ok(xml.to_string());
        }
        last_detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else {
            stdout.trim().to_string()
        };
        if !output.status.success() {
            if let Some(error) = AppError::device_failure(&last_detail) {
                return Err(error);
            }
            if !last_detail.contains("null root node")
                && !last_detail.contains("could not get idle state")
            {
                break;
            }
        }
        if attempt + 1 < attempts {
            thread::sleep(DUMP_RETRY_DELAY);
        }
    }
    Err(AppError::new(
        "UI_NOT_AVAILABLE",
        "uiautomator did not return a UI hierarchy",
    )
    .with_details(json!({ "adb_output": last_detail }))
    .with_suggestion(
        "Unlock the device, keep the screen on, and retry; some secure screens block uiautomator",
    ))
}

pub fn element_matches(element: &UiElement, args: &FindArgs) -> bool {
    if let Some(text) = &args.text {
        let matches = element
            .text
            .as_deref()
            .is_some_and(|value| matches_value(value, text, args.exact))
            || element
                .desc
                .as_deref()
                .is_some_and(|value| matches_value(value, text, args.exact));
        if !matches {
            return false;
        }
    }
    if let Some(desc) = &args.desc
        && !matches_value(element.desc.as_deref().unwrap_or(""), desc, args.exact)
    {
        return false;
    }
    if let Some(resource_id) = &args.resource_id
        && !matches_value(
            element.resource_id.as_deref().unwrap_or(""),
            resource_id,
            args.exact,
        )
    {
        return false;
    }
    if let Some(class) = &args.class
        && !matches_value(&element.class, class, args.exact)
        && !matches_value(&element.role, class, args.exact)
    {
        return false;
    }
    true
}

fn matches_value(value: &str, query: &str, exact: bool) -> bool {
    if exact {
        value == query
    } else {
        value.to_lowercase().contains(&query.to_lowercase())
    }
}

fn next_snapshot_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("s{millis:x}{:x}", std::process::id())
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
