use super::*;

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
    assert!(point_to_native(Point { x: 3.0e9, y: 0.0 }).is_err());
    assert!(point_to_native(Point { x: 0.0, y: -3.0e9 }).is_err());
}
