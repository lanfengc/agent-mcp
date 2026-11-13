use crate::args::{KeyArgs, LongPressArgs, ScrollArgs, SwipeArgs, TapArgs, TypeArgs};
use crate::bounds::{Bounds, Point};
use crate::commands::{Ctx, device, snapshot};
use crate::error::{AppError, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};
use std::thread;
use std::time::Duration;

const UNCHECKED_KEY_NOTICE: &str = "This key name was passed to the device as given; Android ignores a name it does not know and reports no error";

pub fn tap(ctx: &Ctx<'_>, args: &TapArgs) -> Result<Value> {
    ctx.check_write("tap")?;
    let (point, snapshot_id) = resolve_point(
        ctx,
        args.x,
        args.y,
        args.reference.as_deref(),
        args.snapshot_id.as_deref(),
    )?;
    tap_point(ctx, point)?;
    Ok(point_json(point, snapshot_id.as_deref()))
}

pub fn long_press(ctx: &Ctx<'_>, args: &LongPressArgs) -> Result<Value> {
    ctx.check_write("long_press")?;
    let (point, snapshot_id) = resolve_point(
        ctx,
        args.x,
        args.y,
        args.reference.as_deref(),
        args.snapshot_id.as_deref(),
    )?;
    let duration = args.duration_ms.max(50);
    swipe_command(ctx, point, point, duration)?;
    let mut value = point_json(point, snapshot_id.as_deref());
    value["duration_ms"] = json!(duration);
    Ok(value)
}

pub fn swipe(ctx: &Ctx<'_>, args: &SwipeArgs) -> Result<Value> {
    ctx.check_write("swipe")?;
    ensure_point(args.x1, args.y1)?;
    ensure_point(args.x2, args.y2)?;
    let duration = args.duration_ms.max(1);
    let start = Point {
        x: args.x1,
        y: args.y1,
    };
    let end = Point {
        x: args.x2,
        y: args.y2,
    };
    swipe_command(ctx, start, end, duration)?;
    Ok(json!({
        "start": start,
        "end": end,
        "duration_ms": duration,
    }))
}

pub fn type_text(ctx: &Ctx<'_>, args: &TypeArgs) -> Result<Value> {
    ctx.check_write("type")?;
    let mut snapshot_id = None;
    if let Some(reference) = &args.reference {
        let (point, id) = snapshot::resolve_ref(ctx, reference, args.snapshot_id.as_deref())?;
        tap_point(ctx, point)?;
        thread::sleep(Duration::from_millis(150));
        snapshot_id = Some(id);
    }
    if args.clear {
        clear_field(ctx)?;
    }
    let method = send_text(ctx, &args.text)?;
    let mut value = json!({
        "text_length": args.text.chars().count(),
        "method": method,
    });
    if let Some(id) = snapshot_id {
        value["snapshot_id"] = json!(id);
    }
    Ok(value)
}

pub fn key(ctx: &Ctx<'_>, args: &KeyArgs) -> Result<Value> {
    ctx.check_write("key")?;
    let code = normalize_key(&args.key)?;
    ctx.adb.shell(&["input", "keyevent", &code])?;
    let mut value = json!({ "key": code });
    if !code.chars().all(|character| character.is_ascii_digit()) {
        value["notice"] = json!(UNCHECKED_KEY_NOTICE);
    }
    Ok(value)
}

pub fn scroll(ctx: &Ctx<'_>, args: &ScrollArgs) -> Result<Value> {
    ctx.check_write("scroll")?;
    let direction = args.direction.to_ascii_lowercase();
    if !matches!(direction.as_str(), "up" | "down" | "left" | "right") {
        return Err(AppError::invalid(
            "direction must be one of up, down, left or right",
        ));
    }
    let bounds = if let Some(reference) = &args.reference {
        let (element, _) = snapshot::resolve_element(ctx, reference, args.snapshot_id.as_deref())?;
        element.bounds
    } else {
        let (width, height) = device::screen_size(ctx)?;
        Bounds {
            left: 0,
            top: 0,
            right: width as i32,
            bottom: height as i32,
        }
    };
    let (start, end) = scroll_points(bounds, &direction, args.distance)?;
    swipe_command(ctx, start, end, 300)?;
    Ok(json!({
        "direction": direction,
        "start": start,
        "end": end,
        "bounds": bounds,
    }))
}

fn resolve_point(
    ctx: &Ctx<'_>,
    x: Option<i32>,
    y: Option<i32>,
    reference: Option<&str>,
    snapshot_id: Option<&str>,
) -> Result<(Point, Option<String>)> {
    if let Some(reference) = reference {
        let (point, id) = snapshot::resolve_ref(ctx, reference, snapshot_id)?;
        return Ok((point, Some(id)));
    }
    match (x, y) {
        (Some(x), Some(y)) => {
            ensure_point(x, y)?;
            Ok((Point { x, y }, None))
        }
        _ => Err(AppError::invalid(
            "Provide both x and y, or a ref from android_snapshot",
        )),
    }
}

fn ensure_point(x: i32, y: i32) -> Result<()> {
    if x < 0 || y < 0 {
        return Err(AppError::invalid("Coordinates must be zero or greater"));
    }
    Ok(())
}

fn tap_point(ctx: &Ctx<'_>, point: Point) -> Result<()> {
    let x = point.x.to_string();
    let y = point.y.to_string();
    ctx.adb.shell(&["input", "tap", &x, &y])?;
    Ok(())
}

fn swipe_command(ctx: &Ctx<'_>, start: Point, end: Point, duration_ms: u64) -> Result<()> {
    let x1 = start.x.to_string();
    let y1 = start.y.to_string();
    let x2 = end.x.to_string();
    let y2 = end.y.to_string();
    let duration = duration_ms.to_string();
    ctx.adb
        .shell(&["input", "swipe", &x1, &y1, &x2, &y2, &duration])?;
    Ok(())
}

fn point_json(point: Point, snapshot_id: Option<&str>) -> Value {
    let mut value = json!({ "x": point.x, "y": point.y });
    if let Some(id) = snapshot_id {
        value["snapshot_id"] = json!(id);
    }
    value
}

fn clear_field(ctx: &Ctx<'_>) -> Result<()> {
    let _ = ctx.adb.shell(&["input", "keyevent", "123"]);
    if ctx
        .adb
        .shell(&["input", "keycombination", "113", "29"])
        .is_ok()
        && ctx.adb.shell(&["input", "keyevent", "67"]).is_ok()
    {
        return Ok(());
    }
    for _ in 0..20 {
        ctx.adb
            .shell(&["input", "keyevent", "67", "67", "67", "67", "67"])?;
    }
    Ok(())
}

fn send_text(ctx: &Ctx<'_>, text: &str) -> Result<&'static str> {
    if text.is_empty() {
        return Ok("empty");
    }
    if !is_ascii_input(text) {
        return send_unicode_text(ctx, text);
    }
    let parts = text.split('\n').collect::<Vec<_>>();
    for (index, part) in parts.iter().enumerate() {
        for argument in input_text_arguments(part) {
            ctx.adb.shell(&["input", "text", &argument])?;
        }
        if index + 1 < parts.len() {
            ctx.adb.shell(&["input", "keyevent", "66"])?;
        }
    }
    Ok("input_text")
}

fn send_unicode_text(ctx: &Ctx<'_>, text: &str) -> Result<&'static str> {
    let input_method = ctx
        .adb
        .shell(&["settings", "get", "secure", "default_input_method"])
        .unwrap_or_default();
    if !input_method.contains("com.android.adbkeyboard") {
        return Err(AppError::new(
            "UNSUPPORTED_TEXT",
            "adb shell input cannot send this text directly; it contains non-ASCII or control characters",
        )
        .with_suggestion(
            "Send printable ASCII, or install ADBKeyboard (com.android.adbkeyboard, GPL-2.0, not bundled here) and select it as the input method",
        ));
    }
    let encoded = STANDARD.encode(text);
    ctx.adb.shell(&[
        "am",
        "broadcast",
        "-a",
        "ADB_INPUT_B64",
        "--es",
        "msg",
        &encoded,
    ])?;
    Ok("adb_keyboard")
}

fn is_ascii_input(text: &str) -> bool {
    text.chars().all(|character| {
        character.is_ascii() && (character == ' ' || character == '\n' || !character.is_control())
    })
}

fn input_text_arguments(text: &str) -> Vec<String> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if character == 's' && current.ends_with('%') {
            arguments.push(std::mem::take(&mut current));
        }
        if character == ' ' {
            current.push_str("%s");
        } else {
            current.push(character);
        }
    }
    if !current.is_empty() {
        arguments.push(current);
    }
    arguments
}

fn normalize_key(key: &str) -> Result<String> {
    let key = key.trim();
    if key.is_empty() {
        return Err(AppError::invalid("key must not be empty"));
    }
    if key.chars().all(|character| character.is_ascii_digit()) {
        return Ok(key.to_string());
    }
    let upper = key.to_ascii_uppercase();
    let name = upper.strip_prefix("KEYCODE_").unwrap_or(&upper);
    if !name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return Err(AppError::invalid(format!("Unknown key name: {key}")));
    }
    let code = match name {
        "ENTER" => "66",
        "BACK" => "4",
        "HOME" => "3",
        "MENU" => "82",
        "APP_SWITCH" => "187",
        "TAB" => "61",
        "SPACE" => "62",
        "DEL" => "67",
        "FORWARD_DEL" => "112",
        "ESCAPE" => "111",
        "DPAD_UP" => "19",
        "DPAD_DOWN" => "20",
        "DPAD_LEFT" => "21",
        "DPAD_RIGHT" => "22",
        "DPAD_CENTER" => "23",
        "VOLUME_UP" => "24",
        "VOLUME_DOWN" => "25",
        "POWER" => "26",
        "CAMERA" => "27",
        "SEARCH" => "84",
        "MEDIA_PLAY_PAUSE" => "85",
        "PAGE_UP" => "92",
        "PAGE_DOWN" => "93",
        "MOVE_HOME" => "122",
        "MOVE_END" => "123",
        _ => return Ok(upper),
    };
    Ok(code.to_string())
}

fn scroll_points(bounds: Bounds, direction: &str, distance: Option<u32>) -> Result<(Point, Point)> {
    if bounds.is_empty() {
        return Err(AppError::invalid(
            "Cannot scroll an element with empty bounds; take a new snapshot",
        ));
    }
    let center = bounds.center();
    let horizontal = scroll_span(bounds.width(), distance) / 2;
    let vertical = scroll_span(bounds.height(), distance) / 2;
    let (finger_dx, finger_dy) = match direction {
        "down" => (0, -vertical),
        "up" => (0, vertical),
        "right" => (-horizontal, 0),
        "left" => (horizontal, 0),
        _ => return Err(AppError::invalid("Unknown scroll direction")),
    };
    Ok((
        Point {
            x: center.x - finger_dx,
            y: center.y - finger_dy,
        },
        Point {
            x: center.x + finger_dx,
            y: center.y + finger_dy,
        },
    ))
}

fn scroll_span(dimension: i32, distance: Option<u32>) -> i32 {
    match distance {
        Some(distance) => (distance as i32).clamp(1, (dimension / 2).max(1)),
        None => ((dimension * 3) / 5).max(1),
    }
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
