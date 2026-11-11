use super::*;

fn window() -> WindowInfo {
    WindowInfo {
        id: "w-1".to_string(),
        title: "Target".to_string(),
        app: "fixture".to_string(),
        pid: crate::ProcessId::new(1),
        process_instance: Some("test-instance".to_string()),
        bounds: Some(Rect {
            x: 100.0,
            y: 200.0,
            width: 400.0,
            height: 300.0,
        }),
        state: Default::default(),
    }
}

fn token() -> CaptureToken {
    CaptureToken::new(
        &window(),
        Rect {
            x: 100.0,
            y: 200.0,
            width: 400.0,
            height: 300.0,
        },
        800,
        600,
    )
    .expect("valid capture metadata")
}

#[test]
fn round_trips_through_the_opaque_id() {
    let original = token();
    let encoded = original.encode().expect("encode");
    let decoded = CaptureToken::decode(&encoded).expect("decode");

    assert_eq!(decoded.window_id, original.window_id);
    assert_eq!(decoded.process_instance, original.process_instance);
    assert_eq!(decoded.window_bounds, original.window_bounds);
    assert_eq!(decoded.capture_bounds, original.capture_bounds);
    assert_eq!(decoded.image_width, original.image_width);
    assert_eq!(decoded.image_height, original.image_height);
    assert_eq!(decoded.captured_at_ms, original.captured_at_ms);
}

#[test]
fn maps_image_center_to_window_center() {
    let point = token().image_to_screen(400.0, 300.0).expect("map point");

    assert_eq!(point.x, 300.0);
    assert_eq!(point.y, 350.0);
}

#[test]
fn maps_image_pixels_from_the_capture_origin() {
    let token = CaptureToken::new(
        &window(),
        Rect {
            x: 107.0,
            y: 200.0,
            width: 362.0,
            height: 252.0,
        },
        362,
        252,
    )
    .expect("valid capture metadata");

    let point = token.image_to_screen(181.0, 126.0).expect("map point");

    assert_eq!(point.x, 288.0);
    assert_eq!(point.y, 326.0);
}

#[test]
fn rejects_points_outside_the_image() {
    assert!(token().image_to_screen(800.0, 0.0).is_err());
    assert!(token().image_to_screen(0.0, 600.0).is_err());
    assert!(token().image_to_screen(801.0, 0.0).is_err());
    assert!(token().image_to_screen(0.0, 601.0).is_err());
    assert!(token().image_to_screen(-1.0, 0.0).is_err());
}

#[test]
fn rejects_malformed_ids() {
    assert!(CaptureToken::decode("not-a-capture").is_err());
}
