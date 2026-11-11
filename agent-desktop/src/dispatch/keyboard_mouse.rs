use agent_desktop_core::{
    AppError, PlatformAdapter,
    commands::{
        drag as drag_command, helpers, hover as hover_command, key_down as key_down_command,
        key_up as key_up_command, mouse_click as mouse_click_command,
        mouse_click_background as mouse_click_background_command, mouse_down as mouse_down_command,
        mouse_move as mouse_move_command, mouse_up as mouse_up_command,
        mouse_wheel as mouse_wheel_command, press as press_command,
    },
    context::CommandContext,
};
use serde_json::Value;

use crate::cli_args::{
    actions::{HoverArgs, KeyComboArgs, MouseClickArgs, MouseMoveArgs, MousePointArgs, PressArgs},
    drag::DragCliArgs,
    mouse_wheel::MouseWheelArgs,
};
use crate::dispatch::parse::{parse_modifiers, parse_mouse_button, parse_xy, parse_xy_opt};

const MAX_CAPTURE_AGE_ENV: &str = "AGENT_DESKTOP_MAX_CAPTURE_AGE_MS";
const DEFAULT_MAX_CAPTURE_AGE_MS: u64 = 10_000;
const MAX_CAPTURE_AGE_LIMIT_MS: u64 = 86_400_000;

pub(super) fn press(
    args: PressArgs,
    adapter: &dyn PlatformAdapter,
    context: &CommandContext,
) -> Result<Value, AppError> {
    press_command::execute(
        press_command::PressArgs {
            combo: args.combo,
            app: args.app,
            window_id: args.window_id,
            force: args.force,
        },
        adapter,
        context,
    )
}

pub(super) fn key_down(
    args: KeyComboArgs,
    adapter: &dyn PlatformAdapter,
) -> Result<Value, AppError> {
    key_down_command::execute(
        key_down_command::KeyDownArgs {
            combo: args.combo,
            force: args.force,
        },
        adapter,
    )
}

pub(super) fn key_up(args: KeyComboArgs, adapter: &dyn PlatformAdapter) -> Result<Value, AppError> {
    key_up_command::execute(
        key_up_command::KeyUpArgs {
            combo: args.combo,
            force: args.force,
        },
        adapter,
    )
}

pub(super) fn hover(
    args: HoverArgs,
    adapter: &dyn PlatformAdapter,
    context: &CommandContext,
) -> Result<Value, AppError> {
    hover_command::execute(
        hover_command::HoverArgs {
            ref_id: args.ref_id,
            xy: parse_xy_opt(args.xy.as_deref())?,
            duration_ms: args.duration,
            timeout_ms: helpers::normalize_action_timeout_ms(args.timeout_ms),
        },
        adapter,
        context,
    )
}

pub(super) fn drag(
    args: DragCliArgs,
    adapter: &dyn PlatformAdapter,
    context: &CommandContext,
) -> Result<Value, AppError> {
    drag_command::execute(
        drag_command::DragArgs {
            from: drag_command::DragEndpoint {
                ref_id: args.target.from,
                xy: parse_xy_opt(args.target.from_xy.as_deref())?,
            },
            to: drag_command::DragEndpoint {
                ref_id: args.target.to,
                xy: parse_xy_opt(args.target.to_xy.as_deref())?,
            },
            duration_ms: args.duration,
            drop_delay_ms: args.drop_delay,
            timeout_ms: helpers::normalize_action_timeout_ms(args.timeout_ms),
            wait_for_scope: args
                .wait_for_scope
                .map(crate::cli_args::drag::WaitForScopeArg::to_core)
                .unwrap_or_default(),
        },
        adapter,
        context,
    )
}

pub(super) fn mouse_move(
    args: MouseMoveArgs,
    adapter: &dyn PlatformAdapter,
    context: &CommandContext,
) -> Result<Value, AppError> {
    let (x, y) = parse_xy(&args.xy)?;
    mouse_move_command::execute(mouse_move_command::MouseMoveArgs { x, y }, adapter, context)
}

pub(super) fn mouse_click(
    args: MouseClickArgs,
    adapter: &dyn PlatformAdapter,
    context: &CommandContext,
) -> Result<Value, AppError> {
    let (x, y) = parse_xy(&args.xy)?;
    if args.background {
        return background_click(args, x, y, adapter, context);
    }
    if args.window_id.is_some() || args.capture_id.is_some() {
        return Err(AppError::invalid_input_with_suggestion(
            "--window-id and --capture-id require --background",
            "Add --background to post the click without moving the cursor.",
        ));
    }
    mouse_click_command::execute(
        mouse_click_command::MouseClickArgs {
            x,
            y,
            button: parse_mouse_button(&args.button)?,
            count: args.count,
            modifiers: parse_modifiers(&args.modifiers)?,
        },
        adapter,
        context,
    )
}

fn background_click(
    args: MouseClickArgs,
    x: f64,
    y: f64,
    adapter: &dyn PlatformAdapter,
    context: &CommandContext,
) -> Result<Value, AppError> {
    if !args.modifiers.is_empty() {
        return Err(AppError::invalid_input_with_suggestion(
            "--background does not support --modifiers",
            "Use a foreground mouse-click when the click needs held modifier keys.",
        ));
    }
    let Some(window_id) = args.window_id else {
        return Err(AppError::invalid_input_with_suggestion(
            "--background requires --window-id",
            "Run 'list-windows' and pass the target window id.",
        ));
    };
    let space = if args.capture_id.is_some() {
        "image"
    } else {
        "screen"
    };
    mouse_click_background_command::execute(
        mouse_click_background_command::BackgroundClickArgs {
            window_id,
            capture_id: args.capture_id,
            space: space.to_string(),
            max_capture_age_ms: max_capture_age_ms()?,
        },
        x,
        y,
        parse_mouse_button(&args.button)?,
        args.count,
        adapter,
        context,
    )
}

fn max_capture_age_ms() -> Result<u64, AppError> {
    let Some(raw) = std::env::var_os(MAX_CAPTURE_AGE_ENV) else {
        return Ok(DEFAULT_MAX_CAPTURE_AGE_MS);
    };
    parse_max_capture_age(&raw.to_string_lossy())
}

fn parse_max_capture_age(raw: &str) -> Result<u64, AppError> {
    let value = raw.trim().parse::<u64>().map_err(|_| {
        AppError::invalid_input_with_suggestion(
            format!("{MAX_CAPTURE_AGE_ENV} must be an integer millisecond value"),
            "Use a positive value such as 120000, or unset the variable for 10000.",
        )
    })?;
    if !(1..=MAX_CAPTURE_AGE_LIMIT_MS).contains(&value) {
        return Err(AppError::invalid_input_with_suggestion(
            format!(
                "{MAX_CAPTURE_AGE_ENV} must be between 1 and {MAX_CAPTURE_AGE_LIMIT_MS} milliseconds"
            ),
            "Use a positive value such as 120000, or unset the variable for 10000.",
        ));
    }
    Ok(value)
}

pub(super) fn mouse_down(
    args: MousePointArgs,
    adapter: &dyn PlatformAdapter,
    context: &CommandContext,
) -> Result<Value, AppError> {
    let (x, y) = parse_xy(&args.xy)?;
    mouse_down_command::execute(
        mouse_down_command::MouseDownArgs {
            x,
            y,
            button: parse_mouse_button(&args.button)?,
            modifiers: parse_modifiers(&args.modifiers)?,
        },
        adapter,
        context,
    )
}

pub(super) fn mouse_up(
    args: MousePointArgs,
    adapter: &dyn PlatformAdapter,
    context: &CommandContext,
) -> Result<Value, AppError> {
    let (x, y) = parse_xy(&args.xy)?;
    mouse_up_command::execute(
        mouse_up_command::MouseUpArgs {
            x,
            y,
            button: parse_mouse_button(&args.button)?,
            modifiers: parse_modifiers(&args.modifiers)?,
        },
        adapter,
        context,
    )
}

pub(super) fn mouse_wheel(
    args: MouseWheelArgs,
    adapter: &dyn PlatformAdapter,
    context: &CommandContext,
) -> Result<Value, AppError> {
    mouse_wheel_command::execute(
        mouse_wheel_command::MouseWheelArgs {
            x: args.x,
            y: args.y,
            dy: args.vertical_lines(),
            dx: args.dx,
            modifiers: parse_modifiers(&args.modifiers)?,
        },
        adapter,
        context,
    )
}

#[cfg(test)]
#[path = "keyboard_mouse_tests.rs"]
mod tests;
