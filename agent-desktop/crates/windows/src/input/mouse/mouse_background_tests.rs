use super::*;
use crate::system::fixture::live_identity::window_info_for_current_process;
use crate::system::fixture::test_time::deadline;
use crate::system::window::window_ops::window_id;
use crate::tree::fixture::{LocalFixture, bootstrap, fixture_window};
use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;

#[test]
fn packs_a_native_point_into_the_lparam_layout() {
    let packed = pack_point(POINT {
        x: 0x1234,
        y: 0x5678,
    }) as usize;

    assert_eq!(packed, 0x5678_1234);
}

#[test]
fn packs_negative_coordinates_as_unsigned_words() {
    let packed = pack_point(POINT { x: -1, y: -1 }) as usize;

    assert_eq!(packed, 0xFFFF_FFFF);
}

#[test]
fn maps_each_button_to_its_down_and_up_messages() {
    assert_eq!(
        button_messages(&MouseButton::Left),
        (WM_LBUTTONDOWN, WM_LBUTTONUP, MK_LBUTTON)
    );
    assert_eq!(
        button_messages(&MouseButton::Right),
        (WM_RBUTTONDOWN, WM_RBUTTONUP, MK_RBUTTON)
    );
    assert_eq!(
        button_messages(&MouseButton::Middle),
        (WM_MBUTTONDOWN, WM_MBUTTONUP, MK_MBUTTON)
    );
}

#[test]
fn rejects_coordinates_outside_the_native_range() {
    assert!(point_to_native(&Point { x: 3.0e9, y: 0.0 }).is_err());
    assert!(point_to_native(&Point { x: 0.0, y: -3.0e9 }).is_err());
}

#[test]
fn a_click_over_a_child_control_names_the_child_that_received_it() {
    bootstrap();
    let fixture = LocalFixture::create().expect("fixture window");
    let button = fixture_window::find_button(fixture.handle());
    let mut frame = RECT::default();
    assert_ne!(unsafe { GetWindowRect(button, &mut frame) }, 0);
    let mut center = POINT {
        x: frame.left + (frame.right - frame.left) / 2,
        y: frame.top + (frame.bottom - frame.top) / 2,
    };
    let screen_point = Point {
        x: f64::from(center.x),
        y: f64::from(center.y),
    };
    let window = window_info_for_current_process(fixture.handle());
    let options = BackgroundClickOptions {
        button: MouseButton::Left,
        count: 1,
        restore_minimized: false,
        cursor_fallback: false,
        semantic: false,
    };

    let report = post_background_click(&window, screen_point, options, deadline(5_000))
        .expect("the click is posted");

    assert_ne!(unsafe { ScreenToClient(button, &mut center) }, 0);
    let details = report
        .result
        .details
        .expect("a posted click carries details");
    assert_eq!(details["target_hwnd"], window_id(button));
    assert_ne!(details["target_hwnd"], window.id.as_str());
    assert_eq!(details["client_x"], center.x);
    assert_eq!(details["client_y"], center.y);
}
