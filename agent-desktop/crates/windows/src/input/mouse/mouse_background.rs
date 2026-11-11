//! Background coordinate clicks delivered as window messages.

use std::time::Duration;

use agent_desktop_core::{
    ActionResult, ActionStep, AdapterError, Deadline, DeliverySemantics, ErrorCode, MouseButton,
    OperationReport, OperationTarget, Point, WindowInfo, validate_mouse_click_count,
};

use crate::system::window::window_ops::parse_handle;
use crate::system::window::window_resolve::resolve_window_strict;

const CLICK_GAP: Duration = Duration::from_millis(100);
const MK_LBUTTON: usize = 0x0001;
const MK_RBUTTON: usize = 0x0002;
const MK_MBUTTON: usize = 0x0010;

pub(crate) fn post_background_click(
    window: &WindowInfo,
    point: Point,
    button: MouseButton,
    count: u32,
    deadline: Deadline,
) -> Result<OperationReport, AdapterError> {
    deadline.ensure_remaining()?;
    point.validate()?;
    validate_mouse_click_count(count)?;
    let verified = resolve_window_strict(window, deadline)?;
    let parent = parse_handle(&verified.id);
    if parent.is_null() {
        return Err(invalid_target(
            "The window id is not a Windows HWND-shaped id",
        ));
    }
    if unsafe { IsIconic(parent) } != 0 {
        return Err(invalid_target("Cannot post a click to a minimized window"));
    }
    let (target, client) = target_and_client_point(parent, point)?;
    let (down, up, down_state) = button_messages(&button);
    for click_index in 0..count {
        deadline.ensure_remaining()?;
        post_message(target, WM_MOUSEMOVE, 0, pack_point(client))?;
        post_message(target, down, down_state, pack_point(client))?;
        post_message(target, up, 0, pack_point(client))?;
        if click_index + 1 < count {
            crate::input::mouse::sleep_bounded(deadline, CLICK_GAP)?;
        }
    }
    let target = OperationTarget::for_window(&verified)?;
    let step = ActionStep::succeeded("post_message_click").with_verified(false);
    Ok(OperationReport::new(
        ActionResult::delivered_unverified("mouse-click")
            .with_steps(vec![step])
            .with_details(serde_json::json!({
                "method": "PostMessage",
                "background": true,
                "target_hwnd": verified.id,
                "client_x": client.x,
                "client_y": client.y,
            })),
        target,
    ))
}

fn target_and_client_point(
    parent: crate::system::window::window_enum::WindowHandle,
    point: Point,
) -> Result<(crate::system::window::window_enum::WindowHandle, POINT), AdapterError> {
    let screen = point_to_native(point)?;
    let mut parent_client = screen;
    if unsafe { ScreenToClient(parent, &mut parent_client) } == 0 {
        return Err(native_failure(
            "ScreenToClient failed for the target window",
        ));
    }
    let child = unsafe {
        ChildWindowFromPointEx(
            parent,
            parent_client,
            CWP_SKIPINVISIBLE | CWP_SKIPDISABLED | CWP_SKIPTRANSPARENT,
        )
    };
    let target = if child.is_null() { parent } else { child };
    let client = if target == parent {
        parent_client
    } else {
        let mut child_client = screen;
        if unsafe { ScreenToClient(target, &mut child_client) } == 0 {
            return Err(native_failure("ScreenToClient failed for the child window"));
        }
        child_client
    };
    ensure_client_point(target, client)?;
    Ok((target, client))
}

fn ensure_client_point(
    target: crate::system::window::window_enum::WindowHandle,
    client: POINT,
) -> Result<(), AdapterError> {
    let mut rect = RECT::default();
    if unsafe { GetClientRect(target, &mut rect) } == 0 {
        return Err(native_failure("GetClientRect failed for the target window"));
    }
    if client.x < 0 || client.y < 0 || client.x >= rect.right || client.y >= rect.bottom {
        return Err(invalid_target(
            "The click point is outside the target's client area",
        ));
    }
    Ok(())
}

fn point_to_native(point: Point) -> Result<POINT, AdapterError> {
    let x = point.x.round();
    let y = point.y.round();
    if x < f64::from(i32::MIN) || x > f64::from(i32::MAX) {
        return Err(invalid_target(
            "The click x coordinate is outside the native range",
        ));
    }
    if y < f64::from(i32::MIN) || y > f64::from(i32::MAX) {
        return Err(invalid_target(
            "The click y coordinate is outside the native range",
        ));
    }
    Ok(POINT {
        x: x as i32,
        y: y as i32,
    })
}

fn pack_point(point: POINT) -> isize {
    let x = (point.x as u16) as u32;
    let y = (point.y as u16) as u32;
    ((y << 16) | x) as isize
}

fn post_message(
    target: crate::system::window::window_enum::WindowHandle,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> Result<(), AdapterError> {
    let posted = unsafe { PostMessageW(target, message, wparam, lparam) };
    if posted != 0 {
        return Ok(());
    }
    let code = unsafe { GetLastError() };
    if code == ERROR_ACCESS_DENIED {
        return Err(AdapterError::new(
            ErrorCode::PermDenied,
            "The target window refused a background click",
        )
        .with_suggestion(
            "Run at the same integrity level as the target, or use an explicit foreground click",
        )
        .with_details(serde_json::json!({ "win32_error": code }))
        .with_disposition(DeliverySemantics::not_delivered()));
    }
    Err(native_failure("PostMessageW failed for the target window")
        .with_details(serde_json::json!({ "win32_error": code })))
}

fn button_messages(button: &MouseButton) -> (u32, u32, usize) {
    match button {
        MouseButton::Left => (WM_LBUTTONDOWN, WM_LBUTTONUP, MK_LBUTTON),
        MouseButton::Right => (WM_RBUTTONDOWN, WM_RBUTTONUP, MK_RBUTTON),
        MouseButton::Middle => (WM_MBUTTONDOWN, WM_MBUTTONUP, MK_MBUTTON),
    }
}

fn invalid_target(message: &str) -> AdapterError {
    AdapterError::new(ErrorCode::InvalidArgs, message)
        .with_disposition(DeliverySemantics::not_delivered())
}

fn native_failure(message: &str) -> AdapterError {
    AdapterError::new(ErrorCode::ActionFailed, message)
        .with_disposition(DeliverySemantics::not_delivered())
}

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, GetLastError, POINT, RECT};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CWP_SKIPDISABLED, CWP_SKIPINVISIBLE, CWP_SKIPTRANSPARENT, ChildWindowFromPointEx,
    GetClientRect, IsIconic, PostMessageW, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

#[cfg(test)]
#[path = "mouse_background_tests.rs"]
mod tests;
