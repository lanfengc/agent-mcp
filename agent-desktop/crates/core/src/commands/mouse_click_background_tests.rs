use super::*;
use crate::adapter::{ActionOps, InputOps, ObservationOps, SystemOps};
use crate::{AdapterError, BackgroundClickOptions, OperationReport, OperationTarget};
use crate::{Point, WindowInfo};
use std::sync::Mutex;

struct BackgroundAdapter {
    windows: Vec<WindowInfo>,
    clicked: Mutex<Option<Clicked>>,
}

struct Clicked {
    point: Point,
    options: BackgroundClickOptions,
}

impl ObservationOps for BackgroundAdapter {
    fn list_windows(
        &self,
        _filter: &WindowFilter,
        _deadline: Deadline,
    ) -> Result<Vec<WindowInfo>, AdapterError> {
        Ok(self.windows.clone())
    }
}

impl ActionOps for BackgroundAdapter {}

impl InputOps for BackgroundAdapter {
    fn background_click(
        &self,
        window: &WindowInfo,
        point: Point,
        options: BackgroundClickOptions,
        _lease: &crate::InteractionLease,
    ) -> Result<OperationReport, AdapterError> {
        *self.clicked.lock().unwrap() = Some(Clicked { point, options });
        Ok(OperationReport::delivered_unverified(
            "mouse-click",
            OperationTarget::for_window(window)?,
        ))
    }
}

impl SystemOps for BackgroundAdapter {
    crate::adapter::guarded_interaction_lease!();
}

fn window(bounds: crate::Rect) -> WindowInfo {
    WindowInfo {
        id: "w-42".into(),
        title: "Target".into(),
        app: "fixture".into(),
        pid: crate::ProcessId::new(42),
        process_instance: Some("instance".into()),
        bounds: Some(bounds),
        state: Default::default(),
    }
}

fn adapter(bounds: crate::Rect) -> BackgroundAdapter {
    BackgroundAdapter {
        windows: vec![window(bounds)],
        clicked: Mutex::new(None),
    }
}

fn capture_id(bounds: crate::Rect) -> String {
    crate::system::capture_token::CaptureToken::new(&window(bounds), bounds, 800, 600)
        .unwrap()
        .encode()
        .unwrap()
}

fn other_window_capture_id(bounds: crate::Rect) -> String {
    let mut other = window(bounds);
    other.id = "w-99".into();
    crate::system::capture_token::CaptureToken::new(&other, bounds, 800, 600)
        .unwrap()
        .encode()
        .unwrap()
}

fn execute_click(
    adapter: &BackgroundAdapter,
    capture_id: Option<String>,
    x: f64,
    y: f64,
) -> Result<serde_json::Value, AppError> {
    execute(
        BackgroundClickArgs {
            window_id: "w-42".into(),
            capture_id,
            x,
            y,
            max_capture_age_ms: 10_000,
            options: BackgroundClickOptions {
                button: crate::MouseButton::Left,
                count: 1,
                restore_minimized: false,
                cursor_fallback: false,
                semantic: false,
            },
        },
        adapter,
    )
}

#[test]
fn screen_space_keeps_the_requested_point() {
    let bounds = crate::Rect {
        x: 100.0,
        y: 200.0,
        width: 400.0,
        height: 300.0,
    };
    let adapter = adapter(bounds);

    let response = execute_click(&adapter, None, 150.0, 260.0).unwrap();

    assert_eq!(response["space"], "screen");
    assert!(response.get("capture_id").is_none());
    assert!(response.get("image_x").is_none());
    let clicked = adapter.clicked.lock().unwrap();
    let clicked = clicked.as_ref().unwrap();
    assert_eq!(clicked.point, Point { x: 150.0, y: 260.0 });
    assert!(matches!(clicked.options.button, crate::MouseButton::Left));
    assert_eq!(clicked.options.count, 1);
    assert!(!clicked.options.restore_minimized);
    assert!(!clicked.options.cursor_fallback);
    assert!(!clicked.options.semantic);
}

#[test]
fn image_space_maps_pixels_to_the_window_bounds() {
    let bounds = crate::Rect {
        x: 100.0,
        y: 200.0,
        width: 400.0,
        height: 300.0,
    };
    let adapter = adapter(bounds);

    let token = capture_id(bounds);
    let response = execute_click(&adapter, Some(token.clone()), 400.0, 300.0).unwrap();

    assert_eq!(response["space"], "image");
    assert_eq!(response["capture_id"], token.as_str());
    assert_eq!(response["image_x"], 400.0);
    assert_eq!(response["screen_x"], 300.0);
    let clicked = adapter.clicked.lock().unwrap();
    let clicked = clicked.as_ref().unwrap();
    assert_eq!(clicked.point, Point { x: 300.0, y: 350.0 });
}

#[test]
fn a_capture_from_moved_window_is_rejected_as_stale() {
    let captured = crate::Rect {
        x: 100.0,
        y: 200.0,
        width: 400.0,
        height: 300.0,
    };
    let live = crate::Rect {
        x: 120.0,
        y: 200.0,
        width: 400.0,
        height: 300.0,
    };

    let error = execute_click(&adapter(live), Some(capture_id(captured)), 1.0, 1.0).unwrap_err();

    assert_eq!(error.code(), "STALE_REF");
}

#[test]
fn a_capture_from_another_window_is_rejected_as_stale() {
    let bounds = crate::Rect {
        x: 100.0,
        y: 200.0,
        width: 400.0,
        height: 300.0,
    };
    let token = other_window_capture_id(bounds);

    let error = execute_click(&adapter(bounds), Some(token), 1.0, 1.0).unwrap_err();

    assert_eq!(error.code(), "STALE_REF");
}

#[test]
fn an_unknown_window_is_rejected_before_dispatch() {
    let adapter = BackgroundAdapter {
        windows: Vec::new(),
        clicked: Mutex::new(None),
    };

    let error = execute_click(&adapter, None, 1.0, 1.0).unwrap_err();

    assert_eq!(error.code(), "WINDOW_NOT_FOUND");
    assert!(adapter.clicked.lock().unwrap().is_none());
}

fn fixture_adapter() -> BackgroundAdapter {
    adapter(crate::Rect {
        x: 100.0,
        y: 200.0,
        width: 400.0,
        height: 300.0,
    })
}

fn click_with(
    adapter: &BackgroundAdapter,
    options: BackgroundClickOptions,
) -> Result<serde_json::Value, AppError> {
    execute(
        BackgroundClickArgs {
            window_id: "w-42".into(),
            capture_id: None,
            x: 150.0,
            y: 260.0,
            max_capture_age_ms: 10_000,
            options,
        },
        adapter,
    )
}

#[test]
fn button_count_minimized_and_cursor_options_reach_the_adapter() {
    let adapter = fixture_adapter();

    click_with(
        &adapter,
        BackgroundClickOptions {
            button: crate::MouseButton::Right,
            count: 2,
            restore_minimized: true,
            cursor_fallback: true,
            semantic: false,
        },
    )
    .unwrap();

    let clicked = adapter.clicked.lock().unwrap();
    let clicked = clicked.as_ref().unwrap();
    assert!(clicked.options.restore_minimized);
    assert!(clicked.options.cursor_fallback);
    assert!(!clicked.options.semantic);
    assert_eq!(clicked.options.count, 2);
    assert!(matches!(clicked.options.button, crate::MouseButton::Right));
}

#[test]
fn a_semantic_left_click_reaches_the_adapter() {
    let adapter = fixture_adapter();

    let response = click_with(
        &adapter,
        BackgroundClickOptions {
            button: crate::MouseButton::Left,
            count: 1,
            restore_minimized: false,
            cursor_fallback: false,
            semantic: true,
        },
    )
    .unwrap();

    assert_eq!(response["semantic"], true);
    let clicked = adapter.clicked.lock().unwrap();
    assert!(clicked.as_ref().unwrap().options.semantic);
}

#[test]
fn a_semantic_click_with_another_button_is_refused_before_dispatch() {
    for button in [crate::MouseButton::Right, crate::MouseButton::Middle] {
        let adapter = fixture_adapter();

        let error = click_with(
            &adapter,
            BackgroundClickOptions {
                button,
                count: 1,
                restore_minimized: false,
                cursor_fallback: false,
                semantic: true,
            },
        )
        .unwrap_err();

        assert_eq!(error.code(), "INVALID_ARGS");
        assert!(adapter.clicked.lock().unwrap().is_none());
    }
}
