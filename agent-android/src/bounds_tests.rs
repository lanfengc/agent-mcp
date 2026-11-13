use super::{Bounds, Point};

#[test]
fn parses_uiautomator_bounds() {
    let bounds = Bounds::parse("[10,20][110,220]").expect("valid bounds");
    assert_eq!(
        bounds,
        Bounds {
            left: 10,
            top: 20,
            right: 110,
            bottom: 220
        }
    );
    assert_eq!(bounds.width(), 100);
    assert_eq!(bounds.height(), 200);
    assert_eq!(bounds.center(), Point { x: 60, y: 120 });
}

#[test]
fn rejects_malformed_bounds() {
    assert!(Bounds::parse("").is_none());
    assert!(Bounds::parse("[10,20][110]").is_none());
    assert!(Bounds::parse("[110,20][10,220]").is_none());
    assert!(Bounds::parse("(10,20)(110,220)").is_none());
}

#[test]
fn empty_bounds_have_no_area() {
    let bounds = Bounds::parse("[10,10][10,10]").expect("valid zero bounds");
    assert!(bounds.is_empty());
}
