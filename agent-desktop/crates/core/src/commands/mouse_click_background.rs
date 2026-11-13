use crate::{
    AdapterError, AppError, BackgroundClickOptions, Deadline, DeliverySemantics, ErrorCode,
    MouseButton, Point, WindowFilter, WindowInfo, adapter::PlatformAdapter,
    commands::operation_response::operation_response, system::capture_token::CaptureToken,
};
use serde_json::{Value, json};

/// A click posted to one window. With `capture_id` the point is in the pixels
/// of that capture; without it the point is in screen coordinates.
pub struct BackgroundClickArgs {
    pub window_id: String,
    pub capture_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub max_capture_age_ms: u64,
    pub options: BackgroundClickOptions,
}

pub fn execute(
    args: BackgroundClickArgs,
    adapter: &dyn PlatformAdapter,
) -> Result<Value, AppError> {
    crate::validate_mouse_click_count(args.options.count)?;
    if args.options.semantic && !matches!(args.options.button, MouseButton::Left) {
        return Err(AppError::invalid_input_with_suggestion(
            "--semantic invokes the element's primary action, so it accepts only the left button",
            "Drop --semantic to post a right or middle click to the window.",
        ));
    }
    let deadline = Deadline::standard()?;
    let lease = crate::commands::helpers::acquire_interaction_lease(adapter)?;
    let window = resolve_window(&args.window_id, adapter, deadline)?;
    let screen_point = resolve_point(&args, &window)?;
    let mut request = json!({
        "window_id": window.id,
        "space": "screen",
        "screen_x": screen_point.x,
        "screen_y": screen_point.y,
        "restore_minimized": args.options.restore_minimized,
        "cursor_fallback": args.options.cursor_fallback,
        "semantic": args.options.semantic,
    });
    if let Some(capture_id) = args.capture_id {
        request["space"] = json!("image");
        request["capture_id"] = json!(capture_id);
        request["image_x"] = json!(args.x);
        request["image_y"] = json!(args.y);
    }
    let report = adapter.background_click(&window, screen_point, args.options, &lease)?;
    operation_response(&report, request)
}

fn resolve_window(
    window_id: &str,
    adapter: &dyn PlatformAdapter,
    deadline: Deadline,
) -> Result<WindowInfo, AppError> {
    let mut matches = adapter
        .list_windows(&WindowFilter::default(), deadline)?
        .into_iter()
        .filter(|window| window.id == window_id);
    let Some(window) = matches.next() else {
        return Err(AdapterError::new(
            ErrorCode::WindowNotFound,
            format!("Window '{window_id}' was not found"),
        )
        .with_suggestion("Run 'list-windows' to refresh window IDs, then retry.")
        .with_disposition(DeliverySemantics::not_delivered())
        .into());
    };
    if matches.next().is_some() {
        return Err(AdapterError::new(
            ErrorCode::AmbiguousTarget,
            format!("Multiple windows matched id '{window_id}'"),
        )
        .with_disposition(DeliverySemantics::not_delivered())
        .into());
    }
    if window.process_instance.as_deref().is_none_or(str::is_empty) {
        return Err(AdapterError::new(
            ErrorCode::ActionNotSupported,
            "The target window has no process instance token",
        )
        .with_suggestion("Refresh the target with 'list-windows', then retry")
        .with_disposition(DeliverySemantics::not_delivered())
        .into());
    }
    Ok(window)
}

fn resolve_point(args: &BackgroundClickArgs, window: &WindowInfo) -> Result<Point, AppError> {
    let Some(capture_id) = args.capture_id.as_deref() else {
        let point = Point {
            x: args.x,
            y: args.y,
        };
        point.validate()?;
        return Ok(point);
    };
    let token = CaptureToken::decode(capture_id)?;
    if token.window_id != window.id {
        return Err(stale_capture("The capture belongs to a different window", capture_id).into());
    }
    if token.process_instance != window.process_instance.as_deref().unwrap_or_default() {
        return Err(
            stale_capture("The window now belongs to a different process", capture_id).into(),
        );
    }
    if token.age_ms() > args.max_capture_age_ms {
        return Err(stale_capture("The capture is older than allowed", capture_id).into());
    }
    if window.bounds != Some(token.window_bounds) {
        return Err(
            stale_capture("The window moved or resized since the capture", capture_id).into(),
        );
    }
    Ok(token.image_to_screen(args.x, args.y)?)
}

fn stale_capture(reason: &str, capture_id: &str) -> AdapterError {
    AdapterError::new(ErrorCode::StaleRef, "The screenshot capture is stale")
        .with_suggestion("Take a new screenshot and use its capture_id unchanged")
        .with_details(json!({
            "kind": "stale_capture",
            "reason": reason,
            "capture_id": capture_id,
        }))
        .with_disposition(DeliverySemantics::not_delivered())
}

#[cfg(test)]
#[path = "mouse_click_background_tests.rs"]
mod tests;
