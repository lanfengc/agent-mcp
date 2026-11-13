use super::{parse_dimensions, parse_wm_density, parse_wm_size, prop};

#[test]
fn prefers_override_screen_size() {
    let output = "Physical size: 1080x2400\nOverride size: 720x1600\n";
    assert_eq!(parse_wm_size(output), Some((720, 1600)));
}

#[test]
fn reads_physical_screen_size() {
    let output = "Physical size: 1440x3120\n";
    assert_eq!(parse_wm_size(output), Some((1440, 3120)));
}

#[test]
fn rejects_unknown_screen_size() {
    assert_eq!(parse_wm_size("Wm size: unknown\n"), None);
    assert_eq!(parse_dimensions("1080"), None);
}

#[test]
fn reads_density_from_last_line() {
    let output = "Physical density: 420\nOverride density: 560\n";
    assert_eq!(parse_wm_density(output), Some(560));
}

#[test]
fn reads_getprop_values() {
    let output = "[ro.product.model]: [Pixel 7]\n[ro.build.version.sdk]: [34]\n";
    assert_eq!(prop(output, "ro.product.model").as_deref(), Some("Pixel 7"));
    assert_eq!(prop(output, "ro.build.version.sdk").as_deref(), Some("34"));
    assert_eq!(prop(output, "ro.missing"), None);
}
