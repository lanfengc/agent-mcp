use crate::{
    AdapterError, AppError, Deadline, ErrorCode, ImageBuffer, WindowInfo,
    adapter::{PlatformAdapter, ScreenshotTarget, WindowFilter},
    system::{capture_token::CaptureToken, process_state::ProcessState},
};
use base64::Engine;
use serde_json::{Value, json};
use std::path::PathBuf;

const PROCESS_STATE_PROBE_BUDGET: std::time::Duration = std::time::Duration::from_secs(1);

pub struct ScreenshotArgs {
    pub app: Option<String>,
    pub window_id: Option<String>,
    pub screen: Option<usize>,
    pub output_path: Option<PathBuf>,
    pub ocr: bool,
    pub ocr_language: Option<String>,
    pub ocr_query: Option<String>,
}

pub fn execute(args: ScreenshotArgs, adapter: &dyn PlatformAdapter) -> Result<Value, AppError> {
    let deadline = crate::Deadline::standard()?;
    let target = resolve_target(&args, adapter, deadline)?;
    let window = match &target {
        ScreenshotTarget::ExactWindow(window) => Some(window.clone()),
        _ => None,
    };
    let buf = adapter.screenshot(target, deadline)?;
    let mut fields = capture_fields(window.as_ref(), &buf, adapter)?;

    if args.ocr {
        let result = adapter.ocr_image(&buf, args.ocr_language.as_deref(), deadline)?;
        if let Some(query) = args.ocr_query.as_deref() {
            fields["ocr_match"] = match find_match(&result, query) {
                Some(found) => found,
                None => Value::Null,
            };
        }
        fields["ocr"] = serde_json::to_value(&result)
            .map_err(|error| AppError::Adapter(AdapterError::internal(error.to_string())))?;
    }

    if let Some(path) = args.output_path {
        crate::refs::write_user_file(&path, &buf.data)
            .map_err(|error| crate::refs::unwritable_output("the screenshot", &path, error))?;
        fields["path"] = json!(path.to_string_lossy());
    } else {
        let encoded = base64::engine::general_purpose::STANDARD.encode(&buf.data);
        fields["data"] = json!(encoded);
    }
    Ok(fields)
}

pub(crate) fn find_match(result: &crate::OcrResult, query: &str) -> Option<Value> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return None;
    }
    find_word_match(result, &needle)
        .or_else(|| find_line_match(result, &needle))
        .or_else(|| find_compact_match(result, &needle))
}

fn find_word_match(result: &crate::OcrResult, needle: &str) -> Option<Value> {
    let mut best: Option<(&crate::OcrWord, &crate::OcrLine)> = None;
    for line in &result.lines {
        for word in &line.words {
            if !word.text.to_lowercase().contains(needle) {
                continue;
            }
            let replace = best.is_none_or(|(current, _)| {
                word.rect.width * word.rect.height > current.rect.width * current.rect.height
            });
            if replace {
                best = Some((word, line));
            }
        }
    }
    let (word, line) = best?;
    Some(match_json(&word.text, &line.text, &word.rect))
}

fn find_line_match(result: &crate::OcrResult, needle: &str) -> Option<Value> {
    result
        .lines
        .iter()
        .find(|line| line.text.to_lowercase().contains(needle))
        .map(|line| match_json(&line.text, &line.text, &line.rect))
}

fn find_compact_match(result: &crate::OcrResult, needle: &str) -> Option<Value> {
    let compact = needle
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    if compact.is_empty() {
        return None;
    }
    for line in &result.lines {
        let mut joined = String::new();
        let mut ranges = Vec::with_capacity(line.words.len());
        for word in &line.words {
            let start = joined.len();
            joined.push_str(&word.text.to_lowercase());
            ranges.push((start, joined.len()));
        }
        let Some(position) = joined.find(&compact) else {
            continue;
        };
        let end = position + compact.len();
        let covered = line
            .words
            .iter()
            .zip(ranges)
            .filter(|(_, (start, stop))| *stop > position && *start < end)
            .map(|(word, _)| word);
        let Some(rect) = union_rects(covered) else {
            continue;
        };
        return Some(match_json(needle, &line.text, &rect));
    }
    None
}

fn union_rects<'a>(words: impl Iterator<Item = &'a crate::OcrWord>) -> Option<crate::Rect> {
    let mut bounds: Option<crate::Rect> = None;
    for word in words {
        bounds = Some(match bounds {
            None => word.rect,
            Some(current) => {
                let left = current.x.min(word.rect.x);
                let top = current.y.min(word.rect.y);
                let right = (current.x + current.width).max(word.rect.x + word.rect.width);
                let bottom = (current.y + current.height).max(word.rect.y + word.rect.height);
                crate::Rect {
                    x: left,
                    y: top,
                    width: right - left,
                    height: bottom - top,
                }
            }
        });
    }
    bounds
}

fn match_json(text: &str, line: &str, rect: &crate::Rect) -> Value {
    json!({
        "text": text,
        "line": line,
        "x": rect.x,
        "y": rect.y,
        "width": rect.width,
        "height": rect.height,
        "center_x": rect.x + rect.width / 2.0,
        "center_y": rect.y + rect.height / 2.0
    })
}

fn capture_fields(
    window: Option<&WindowInfo>,
    buf: &ImageBuffer,
    adapter: &dyn PlatformAdapter,
) -> Result<Value, AppError> {
    let mut fields = json!({
        "format": buf.format.as_str(),
        "width": buf.width,
        "height": buf.height,
        "scale_factor": buf.scale_factor
    });
    if let Some(window) = window
        && let Some(window_bounds) = window.bounds
    {
        let capture_bounds = adapter
            .capture_bounds(window, buf.width, buf.height)?
            .unwrap_or(window_bounds);
        let token = CaptureToken::new(window, capture_bounds, buf.width, buf.height)?;
        fields["capture_id"] = json!(token.encode()?);
        fields["window_id"] = json!(window.id);
        fields["window_bounds"] = json!(window_bounds);
        fields["capture_bounds"] = json!(capture_bounds);
        fields["capture_time_ms"] = json!(token.captured_at_ms);
    }
    Ok(fields)
}

fn resolve_target(
    args: &ScreenshotArgs,
    adapter: &dyn PlatformAdapter,
    deadline: crate::Deadline,
) -> Result<ScreenshotTarget, AppError> {
    if args.screen.is_some() && (args.app.is_some() || args.window_id.is_some()) {
        return Err(AppError::invalid_input_with_suggestion(
            "--screen cannot be combined with --app or --window-id",
            "Choose exactly one display target or one app/window target.",
        ));
    }
    if let Some(screen) = args.screen {
        let displays = adapter.list_displays(deadline).map_err(AppError::from)?;
        if screen >= displays.len() {
            return Err(AppError::Adapter(
                AdapterError::new(
                    ErrorCode::InvalidArgs,
                    format!(
                        "Display index {screen} out of range; {} display(s) available",
                        displays.len()
                    ),
                )
                .with_suggestion(
                    "Run 'list-displays' to see the display indexes this session has, then use one of them",
                )
                .with_details(json!({
                    "display_count": displays.len(),
                    "display_ids": displays.iter().map(|display| display.id.clone()).collect::<Vec<_>>()
                }))
                .with_disposition(crate::DeliverySemantics::not_delivered()),
            ));
        }
        return Ok(ScreenshotTarget::Display {
            index: screen,
            expected: displays[screen].clone(),
        });
    }

    if let Some(window_id) = &args.window_id {
        let expected_app = args
            .app
            .as_deref()
            .map(|name| crate::commands::helpers::resolve_app(Some(name), adapter, deadline))
            .transpose()?;
        let filter = WindowFilter {
            focused_only: false,
            app: args.app.clone(),
        };
        let mut candidates = adapter
            .list_windows(&filter, deadline)?
            .into_iter()
            .filter(|window| &window.id == window_id)
            .collect::<Vec<_>>();
        if candidates
            .iter()
            .any(|window| window.process_instance.as_deref().is_none_or(str::is_empty))
        {
            return Err(AppError::Adapter(
                AdapterError::new(
                    ErrorCode::ActionNotSupported,
                    "Matching window has incomplete process identity",
                )
                .with_suggestion("Refresh the target with 'list-windows', then retry")
                .with_disposition(crate::DeliverySemantics::not_delivered()),
            ));
        }
        if let Some(app) = expected_app.as_ref() {
            let instance = crate::commands::helpers::process_identity(app)?.instance;
            candidates.retain(|window| {
                window.pid == app.pid
                    && window.process_instance.as_deref() == Some(instance.as_str())
            });
        }
        let win = select_unique_window(
            candidates,
            window_id,
            expected_app.as_ref(),
            adapter,
            deadline,
        )?;
        return Ok(ScreenshotTarget::ExactWindow(win));
    }

    if let Some(app_name) = &args.app {
        let app = crate::commands::helpers::resolve_app(Some(app_name), adapter, deadline)?;
        let win = crate::system::window_lookup::find_window_for_process(
            crate::commands::helpers::process_identity(&app)?,
            adapter,
            deadline,
        )?;
        return Ok(ScreenshotTarget::ExactWindow(win));
    }

    Ok(ScreenshotTarget::FullScreen)
}

fn missing_window_error(
    window_id: &str,
    expected_app: Option<&crate::AppInfo>,
    adapter: &dyn PlatformAdapter,
    deadline: Deadline,
) -> AppError {
    let stopped = expected_app.filter(|app| {
        let Ok(process) = crate::commands::helpers::process_identity(app) else {
            return false;
        };
        let probe = deadline.capped(PROCESS_STATE_PROBE_BUDGET);
        adapter.process_state(process, probe).ok() == Some(ProcessState::Unresponsive)
    });
    if let Some(app) = stopped {
        return AdapterError::app_unresponsive(&app.name).into();
    }
    AdapterError::new(
        ErrorCode::WindowNotFound,
        format!("Window '{window_id}' was not found"),
    )
    .with_suggestion("Run 'list-windows' to refresh window IDs, then retry.")
    .unsent()
    .into()
}

fn select_unique_window(
    mut candidates: Vec<crate::WindowInfo>,
    window_id: &str,
    expected_app: Option<&crate::AppInfo>,
    adapter: &dyn PlatformAdapter,
    deadline: Deadline,
) -> Result<crate::WindowInfo, AppError> {
    match candidates.len() {
        0 => Err(missing_window_error(
            window_id,
            expected_app,
            adapter,
            deadline,
        )),
        1 => Ok(candidates.swap_remove(0)),
        _ => Err(AdapterError::ambiguous_target(format!(
            "Multiple windows matched id '{window_id}'"
        ))
        .with_details(json!({ "candidate_count": candidates.len() }))
        .into()),
    }
}

#[cfg(test)]
#[path = "screenshot_deadline_tests.rs"]
mod deadline_tests;

#[cfg(test)]
#[path = "screenshot_ocr_tests.rs"]
mod ocr_tests;
