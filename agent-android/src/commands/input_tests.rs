use super::{input_text_arguments, is_ascii_input, normalize_key, scroll_points};
use crate::bounds::{Bounds, Point};

#[test]
fn encodes_spaces_for_input_text() {
    assert_eq!(input_text_arguments("hello world"), ["hello%sworld"]);
    assert_eq!(input_text_arguments("a&b;c"), ["a&b;c"]);
    assert_eq!(input_text_arguments("it's"), ["it's"]);
    assert_eq!(input_text_arguments("\"quoted\""), ["\"quoted\""]);
    assert_eq!(input_text_arguments(r"a\b"), [r"a\b"]);
    assert_eq!(input_text_arguments("50% off"), ["50%%soff"]);
    assert!(input_text_arguments("").is_empty());
}

#[test]
fn splits_a_literal_percent_s_so_the_device_does_not_read_it_as_a_space() {
    assert_eq!(input_text_arguments("a%sb"), ["a%", "sb"]);
    assert_eq!(input_text_arguments("%s"), ["%", "s"]);
    assert_eq!(input_text_arguments("x%%sy"), ["x%%", "sy"]);
    assert_eq!(input_text_arguments(" %s %s"), ["%s%", "s%s%", "s"]);
    assert_eq!(input_text_arguments("100%"), ["100%"]);
    assert_eq!(input_text_arguments("%S"), ["%S"]);
}

#[test]
fn recognizes_ascii_input() {
    assert!(is_ascii_input("hello 123"));
    assert!(is_ascii_input("line one\nline two"));
    assert!(!is_ascii_input("你好"));
    assert!(!is_ascii_input("tab\there"));
}

#[test]
fn normalizes_common_key_names() {
    assert_eq!(normalize_key("ENTER").expect("enter"), "66");
    assert_eq!(normalize_key("KEYCODE_BACK").expect("back"), "4");
    assert_eq!(normalize_key("66").expect("numeric"), "66");
    assert_eq!(normalize_key("KEYCODE_FOO").expect("name"), "KEYCODE_FOO");
    assert!(normalize_key("").is_err());
    assert!(normalize_key("A B").is_err());
}

#[test]
fn computes_scroll_points_inside_bounds() {
    let bounds = Bounds {
        left: 0,
        top: 0,
        right: 100,
        bottom: 200,
    };
    let (start, end) = scroll_points(bounds, "down", None).expect("down");
    assert_eq!(start, Point { x: 50, y: 160 });
    assert_eq!(end, Point { x: 50, y: 40 });
    let (start, end) = scroll_points(bounds, "right", None).expect("right");
    assert_eq!(start, Point { x: 80, y: 100 });
    assert_eq!(end, Point { x: 20, y: 100 });
    let (start, end) = scroll_points(bounds, "up", Some(20)).expect("up");
    assert_eq!(start, Point { x: 50, y: 90 });
    assert_eq!(end, Point { x: 50, y: 110 });
}

#[test]
fn each_direction_swipes_the_finger_the_opposite_way() {
    let bounds = Bounds {
        left: 0,
        top: 0,
        right: 100,
        bottom: 200,
    };
    let (start, end) = scroll_points(bounds, "down", None).expect("down");
    assert!(end.y < start.y && end.x == start.x);
    let (start, end) = scroll_points(bounds, "up", None).expect("up");
    assert!(end.y > start.y && end.x == start.x);
    let (start, end) = scroll_points(bounds, "right", None).expect("right");
    assert!(end.x < start.x && end.y == start.y);
    let (start, end) = scroll_points(bounds, "left", None).expect("left");
    assert!(end.x > start.x && end.y == start.y);
}

#[test]
fn rejects_empty_scroll_bounds() {
    let bounds = Bounds {
        left: 10,
        top: 10,
        right: 10,
        bottom: 10,
    };
    assert!(scroll_points(bounds, "up", None).is_err());
}
