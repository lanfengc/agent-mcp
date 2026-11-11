//! Screenshot orchestration: target dispatch, identity sandwich, precedence.

use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, DisplayInfo, ErrorCode, ImageBuffer, Rect,
    ScreenshotTarget, WindowInfo,
};

use super::capture_backend::{CaptureSubject, capture_with_precedence};
use crate::system::display::{
    capture_selection, display_at, scale_for_bounds, verify_display_identity,
};
use crate::system::window::window_ops::parse_handle;
use crate::system::window::window_resolve::resolve_window_strict;

pub(crate) fn screenshot(
    target: ScreenshotTarget,
    deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    dispatch_target(target, deadline).map_err(not_delivered)
}

fn dispatch_target(
    target: ScreenshotTarget,
    deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    deadline.ensure_remaining()?;
    match target {
        ScreenshotTarget::Screen(index) => capture_screen(index, deadline),
        ScreenshotTarget::Display { index, expected } => {
            capture_display(index, &expected, deadline)
        }
        ScreenshotTarget::ExactWindow(window) => capture_window(&window, deadline),
        ScreenshotTarget::FullScreen => capture_screen(0, deadline),
    }
}

fn not_delivered(error: AdapterError) -> AdapterError {
    if error.disposition == DeliverySemantics::Unknown {
        error.with_disposition(DeliverySemantics::not_delivered())
    } else {
        error
    }
}

pub(crate) fn capture_screen(
    index: usize,
    deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    deadline.ensure_remaining()?;
    let expected = display_at(index, deadline)?;
    capture_display(index, &expected, deadline)
}

pub(crate) fn capture_display(
    index: usize,
    expected: &DisplayInfo,
    deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    deadline.ensure_remaining()?;
    let (raw_index, current) = capture_selection(expected, deadline)?;
    deadline.ensure_remaining()?;
    verify_display_identity(index, expected, &current)?;
    let captured = capture_with_precedence(CaptureSubject::Display { index: raw_index }, deadline)?;
    deadline.ensure_remaining()?;
    if !skip_post_identity() {
        let after = display_at(raw_index, deadline)?;
        deadline.ensure_remaining()?;
        verify_display_identity(index, &current, &after)?;
    }
    Ok(captured)
}

pub(crate) fn capture_window(
    window: &WindowInfo,
    deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    if window.process_instance.is_none() {
        return Err(AdapterError::new(
            ErrorCode::InvalidArgs,
            "Exact window screenshot requires a process instance token",
        )
        .with_suggestion("Refresh the target with 'list-windows', then retry")
        .with_disposition(DeliverySemantics::not_delivered()));
    }
    deadline.ensure_remaining()?;
    let verified = resolve_window_strict(window, deadline)?;
    let handle = parse_handle(&verified.id);
    if handle.is_null() {
        return Err(AdapterError::new(
            ErrorCode::InvalidArgs,
            "window id is not a Windows HWND-shaped id",
        )
        .with_disposition(DeliverySemantics::not_delivered()));
    }
    let scale = scale_for_bounds(verified.bounds, deadline)?;
    deadline.ensure_remaining()?;
    let captured = capture_with_precedence(
        CaptureSubject::Window {
            handle,
            scale_factor: scale,
        },
        deadline,
    )?;
    deadline.ensure_remaining()?;
    if !skip_post_identity() {
        if force_post_identity_failure() {
            return Err(AdapterError::new(
                ErrorCode::StaleRef,
                "window identity could not be re-proved after capture",
            )
            .with_suggestion("Run 'list-windows' to refresh window identifiers, then retry.")
            .with_disposition(DeliverySemantics::not_delivered()));
        }
        resolve_window_strict(&verified, deadline)?;
    }
    Ok(captured)
}

pub(crate) fn capture_bounds(
    window: &WindowInfo,
    image_width: u32,
    image_height: u32,
) -> Option<Rect> {
    let handle = parse_handle(&window.id);
    if handle.is_null() {
        return None;
    }
    if let Some(extended) = extended_frame_bounds(handle)
        && matches_image(extended, image_width, image_height)
    {
        return Some(extended);
    }
    window
        .bounds
        .filter(|bounds| matches_image(*bounds, image_width, image_height))
}

fn extended_frame_bounds(handle: crate::system::window::window_enum::WindowHandle) -> Option<Rect> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};

    let mut rect = RECT::default();
    let result = unsafe {
        DwmGetWindowAttribute(
            handle,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            (&mut rect as *mut RECT).cast(),
            size_of::<RECT>() as u32,
        )
    };
    if result != 0 {
        return None;
    }
    Some(Rect {
        x: f64::from(rect.left),
        y: f64::from(rect.top),
        width: f64::from(rect.right.saturating_sub(rect.left)),
        height: f64::from(rect.bottom.saturating_sub(rect.top)),
    })
}

fn matches_image(bounds: Rect, image_width: u32, image_height: u32) -> bool {
    bounds.width > 0.0
        && bounds.height > 0.0
        && (bounds.width - f64::from(image_width)).abs() <= 1.0
        && (bounds.height - f64::from(image_height)).abs() <= 1.0
}

#[cfg(test)]
fn skip_post_identity() -> bool {
    test_hooks::skip_post_identity()
}

#[cfg(not(test))]
fn skip_post_identity() -> bool {
    false
}

#[cfg(test)]
fn force_post_identity_failure() -> bool {
    test_hooks::force_post_identity_failure()
}

#[cfg(not(test))]
fn force_post_identity_failure() -> bool {
    false
}

#[cfg(test)]
pub(crate) mod test_hooks {
    use std::cell::Cell;

    thread_local! {
        static SKIP_POST_IDENTITY: Cell<bool> = const { Cell::new(false) };
        static FORCE_POST_IDENTITY_FAILURE: Cell<bool> = const { Cell::new(false) };
    }

    pub(crate) fn skip_post_identity() -> bool {
        SKIP_POST_IDENTITY.with(Cell::get)
    }

    pub(crate) fn force_post_identity_failure() -> bool {
        FORCE_POST_IDENTITY_FAILURE.with(Cell::get)
    }

    pub(crate) fn with_skip_post_identity<R>(run: impl FnOnce() -> R) -> R {
        crate::system::fixture::test_support::with_flag(&SKIP_POST_IDENTITY, true, run)
    }

    pub(crate) fn with_force_post_identity_failure<R>(run: impl FnOnce() -> R) -> R {
        crate::system::fixture::test_support::with_flag(&FORCE_POST_IDENTITY_FAILURE, true, run)
    }
}

#[cfg(test)]
pub(crate) fn coerce_screenshot_disposition_for_test(error: AdapterError) -> AdapterError {
    not_delivered(error)
}

#[cfg(all(test, target_os = "windows"))]
#[path = "screenshot_tests.rs"]
mod tests;
