//! UI Automation invocation used by `--semantic` background clicks.

use agent_desktop_core::{
    ActionResult, ActionStep, AdapterError, Deadline, OperationReport, Point, WindowInfo,
};

use crate::input::mouse::mouse_background::{CLICK_GAP, point_to_native};
use crate::system::window::window_ops::parse_handle;

const SEMANTIC_MAX_DEPTH: u8 = 16;
const SEMANTIC_MAX_ANCESTOR: usize = 8;

/// Invokes the UI Automation element under `point` instead of posting mouse
/// messages. The element is found inside the target window's own UIA subtree,
/// never with the global `ElementFromPoint`, so another window covering the
/// target cannot redirect the click and the target need not hold focus.
/// Returns `false` when nothing under the point offers `InvokePattern`, so
/// the caller can fall back to a message click.
pub(super) fn semantic_invoke(
    window: &WindowInfo,
    point: &Point,
    count: u32,
    deadline: Deadline,
) -> Result<bool, AdapterError> {
    let client = crate::tree::automation::automation_client()?;
    let walker = client
        .get_control_view_walker()
        .map_err(|error| crate::tree::automation::uia_error(&error, "read the UIA control view"))?;
    let native = point_to_native(point)?;
    let native_point = (native.x, native.y);
    let handle = parse_handle(&window.id);
    let root = match crate::tree::automation::root_from_hwnd(handle as isize, deadline) {
        Ok(root) => root.0,
        Err(error) => {
            tracing::debug!(error = %error, "semantic click: the target UIA root could not be read");
            return Ok(false);
        }
    };
    let Some(element) = deepest_element_at(&walker, &root, native_point, 0, deadline) else {
        tracing::debug!("semantic click: no element bounds contain the point");
        return Ok(false);
    };
    let Some(pattern) = invokable_ancestor(&walker, element, native_point, deadline) else {
        tracing::debug!("semantic click: no element at the point offers InvokePattern");
        return Ok(false);
    };
    for click_index in 0..count {
        deadline.ensure_remaining()?;
        pattern.invoke().map_err(|error| {
            crate::tree::automation::uia_error(
                &error,
                "InvokePattern.Invoke failed for a semantic click",
            )
        })?;
        if click_index + 1 < count {
            crate::input::mouse::sleep_bounded(deadline, CLICK_GAP)?;
        }
    }
    Ok(true)
}

fn deepest_element_at(
    walker: &uiautomation::UITreeWalker,
    element: &uiautomation::UIElement,
    point: (i32, i32),
    depth: u8,
    deadline: Deadline,
) -> Option<uiautomation::UIElement> {
    if depth >= SEMANTIC_MAX_DEPTH || deadline.is_expired() {
        return Some(element.clone());
    }
    let Some(children) = walker.get_children(element) else {
        return Some(element.clone());
    };
    for child in children {
        if deadline.is_expired() {
            return Some(element.clone());
        }
        let Ok(rect) = child.get_bounding_rectangle() else {
            continue;
        };
        if point.0 < rect.get_left()
            || point.1 < rect.get_top()
            || point.0 >= rect.get_right()
            || point.1 >= rect.get_bottom()
        {
            continue;
        }
        return deepest_element_at(walker, &child, point, depth + 1, deadline);
    }
    Some(element.clone())
}

fn invokable_ancestor(
    walker: &uiautomation::UITreeWalker,
    mut element: uiautomation::UIElement,
    point: (i32, i32),
    deadline: Deadline,
) -> Option<uiautomation::patterns::UIInvokePattern> {
    for _ in 0..=SEMANTIC_MAX_ANCESTOR {
        deadline.ensure_remaining().ok()?;
        if let Ok(pattern) = element.get_pattern::<uiautomation::patterns::UIInvokePattern>() {
            return Some(pattern);
        }
        let parent = walker.get_parent(&element).ok()?;
        let rect = parent.get_bounding_rectangle().ok()?;
        if point.0 < rect.get_left()
            || point.1 < rect.get_top()
            || point.0 >= rect.get_right()
            || point.1 >= rect.get_bottom()
        {
            return None;
        }
        element = parent;
    }
    None
}

pub(super) fn semantic_report(
    window: &WindowInfo,
    count: u32,
) -> Result<OperationReport, AdapterError> {
    let target = agent_desktop_core::OperationTarget::for_window(window)?;
    let step = ActionStep::succeeded("InvokePattern.Invoke").with_verified(false);
    Ok(OperationReport::new(
        ActionResult::delivered_unverified("mouse-click")
            .with_steps(vec![step])
            .with_details(serde_json::json!({
                "method": "UIA.Invoke",
                "background": true,
                "count": count,
                "semantic": true,
            })),
        target,
    ))
}
