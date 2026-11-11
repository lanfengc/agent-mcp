use crate::{
    AdapterError, AppError, Deadline, DeliverySemantics, ErrorCode, MouseButton, Point,
    WindowFilter, WindowInfo, adapter::PlatformAdapter,
    commands::operation_response::operation_response, context::CommandContext,
    system::capture_token::CaptureToken,
};
use serde_json::{Value, json};

pub struct BackgroundClickArgs {
    pub window_id: String,
    pub capture_id: Option<String>,
    pub space: String,
    pub max_capture_age_ms: u64,
}

pub fn execute(
    args: BackgroundClickArgs,
    x: f64,
    y: f64,
    button: MouseButton,
    count: u32,
    adapter: &dyn PlatformAdapter,
    _context: &CommandContext,
) -> Result<Value, AppError> {
    crate::validate_mouse_click_count(count)?;
    let deadline = Deadline::standard()?;
    let lease = crate::commands::helpers::acquire_interaction_lease(adapter)?;
    let window = resolve_window(&args.window_id, adapter, deadline)?;
    let (screen_point, image_point) = resolve_point(&args, &window, x, y)?;
    let report = adapter.background_click(&window, screen_point.clone(), button, count, &lease)?;
    let mut request = json!({
        "window_id": window.id,
        "space": args.space,
        "screen_x": screen_point.x,
        "screen_y": screen_point.y,
        "capture_id": args.capture_id,
    });
    if let Some((image_x, image_y)) = image_point {
        request["image_x"] = json!(image_x);
        request["image_y"] = json!(image_y);
    }
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

fn resolve_point(
    args: &BackgroundClickArgs,
    window: &WindowInfo,
    x: f64,
    y: f64,
) -> Result<(Point, Option<(f64, f64)>), AppError> {
    let space = args.space.to_ascii_lowercase();
    match space.as_str() {
        "screen" => {
            let point = Point { x, y };
            point.validate()?;
            Ok((point, None))
        }
        "image" => {
            let capture_id = args.capture_id.as_deref().ok_or_else(|| {
                AppError::invalid_input_with_suggestion(
                    "Image coordinates require a capture_id from screenshot",
                    "Take a screenshot and pass its capture_id unchanged.",
                )
            })?;
            let token = CaptureToken::decode(capture_id)?;
            if token.window_id != window.id {
                return Err(
                    stale_capture("The capture belongs to a different window", capture_id).into(),
                );
            }
            if token.process_instance != window.process_instance.as_deref().unwrap_or_default() {
                return Err(stale_capture(
                    "The window now belongs to a different process",
                    capture_id,
                )
                .into());
            }
            if token.age_ms() > args.max_capture_age_ms {
                return Err(stale_capture("The capture is older than allowed", capture_id).into());
            }
            if window.bounds != Some(token.window_bounds) {
                return Err(stale_capture(
                    "The window moved or resized since the capture",
                    capture_id,
                )
                .into());
            }
            let point = token.image_to_screen(x, y)?;
            Ok((point, Some((x, y))))
        }
        _ => Err(AppError::invalid_input_with_suggestion(
            format!("Unsupported coordinate space '{space}'"),
            "Use --space screen or --space image.",
        )),
    }
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
