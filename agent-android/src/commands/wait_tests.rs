use super::{element_contains_text, is_fatal};
use crate::bounds::{Bounds, Point};
use crate::commands::snapshot::{UiElement, UiFlags};
use crate::error::AppError;

fn element(text: Option<&str>, desc: Option<&str>) -> UiElement {
    UiElement {
        reference: "@a1".to_string(),
        role: "TextView".to_string(),
        class: "android.widget.TextView".to_string(),
        text: text.map(str::to_string),
        desc: desc.map(str::to_string),
        resource_id: None,
        hint: None,
        package: Some("com.example".to_string()),
        bounds: Bounds::default(),
        center: Point::default(),
        flags: UiFlags::default(),
    }
}

#[test]
fn finds_text_or_description_when_both_are_present() {
    let element = element(Some("Settings"), Some("Open settings"));
    assert!(element_contains_text(&element, "settings"));
    assert!(element_contains_text(&element, "open"));
    assert!(!element_contains_text(&element, "missing"));
}

#[test]
fn stops_waiting_on_fatal_errors() {
    assert!(is_fatal(&AppError::adb_not_found()));
    assert!(is_fatal(&AppError::device_not_found()));
    assert!(!is_fatal(&AppError::new(
        "UI_NOT_AVAILABLE",
        "screen is busy"
    )));
}
