use super::{ScreenSize, SnapshotOutput, UiElement, UiFlags, element_matches, resolve_ref};
use crate::adb::Adb;
use crate::args::FindArgs;
use crate::bounds::{Bounds, Point};
use crate::commands::Ctx;
use crate::commands::snapshot_parse::{extract_xml, parse_ui};
use crate::policy::Policy;

const SAMPLE: &str = r#"<?xml version='1.0' encoding='UTF-8' standalone='yes' ?><hierarchy rotation="0"><node index="0" text="" resource-id="" class="android.widget.FrameLayout" package="com.example" content-desc="" clickable="false" enabled="true" bounds="[0,0][1080,2400]"><node index="0" text="OK" resource-id="com.example:id/ok" class="android.widget.Button" package="com.example" content-desc="" clickable="true" enabled="true" bounds="[10,20][110,80]" /><node index="1" text="Hello" resource-id="" class="android.widget.TextView" package="com.example" content-desc="" clickable="false" enabled="true" bounds="[0,100][500,200]" /><node index="2" text="" resource-id="com.example:id/name" class="android.widget.EditText" package="com.example" content-desc="Name" clickable="true" enabled="true" bounds="[0,200][500,300]" /><node index="3" text="" resource-id="" class="android.widget.LinearLayout" package="com.example" content-desc="" clickable="false" enabled="true" bounds="[0,300][1080,500]" /></node></hierarchy>"#;

fn find_args() -> FindArgs {
    FindArgs {
        text: None,
        resource_id: None,
        desc: None,
        class: None,
        exact: false,
    }
}

#[test]
fn parses_only_interesting_nodes_by_default() {
    let parsed = parse_ui(SAMPLE, None, false).expect("valid hierarchy");
    assert_eq!(parsed.package, "com.example");
    assert_eq!(parsed.screen.width, 1080);
    assert_eq!(parsed.screen.height, 2400);
    assert_eq!(parsed.node_count, 5);
    assert_eq!(parsed.elements.len(), 3);
    assert_eq!(parsed.elements[0].reference, "@a1");
    assert_eq!(parsed.elements[0].role, "Button");
    assert_eq!(parsed.elements[0].text.as_deref(), Some("OK"));
    assert_eq!(parsed.elements[0].center.x, 60);
    assert_eq!(parsed.elements[0].center.y, 50);
    assert!(parsed.elements[0].flags.clickable);
    assert_eq!(parsed.elements[1].reference, "@a2");
    assert_eq!(parsed.elements[1].role, "TextView");
    assert_eq!(parsed.elements[2].reference, "@a3");
    assert_eq!(parsed.elements[2].role, "EditText");
    assert_eq!(
        parsed.elements[2].resource_id.as_deref(),
        Some("com.example:id/name")
    );
}

#[test]
fn all_mode_includes_every_node() {
    let parsed = parse_ui(SAMPLE, None, true).expect("valid hierarchy");
    assert_eq!(parsed.elements.len(), 5);
    assert_eq!(parsed.elements[0].role, "FrameLayout");
    assert_eq!(parsed.elements[4].role, "LinearLayout");
}

#[test]
fn max_depth_limits_traversal() {
    let parsed = parse_ui(SAMPLE, Some(0), false).expect("valid hierarchy");
    assert_eq!(parsed.node_count, 1);
    assert!(parsed.elements.is_empty());
    let parsed = parse_ui(SAMPLE, Some(1), false).expect("valid hierarchy");
    assert_eq!(parsed.node_count, 5);
    assert_eq!(parsed.elements.len(), 3);
}

#[test]
fn extracts_xml_from_adb_noise() {
    let output =
        "warning: something\n<?xml version='1.0'?><hierarchy></hierarchy>\nUI hierarchy dumped\n";
    assert_eq!(
        extract_xml(output),
        Some("<?xml version='1.0'?><hierarchy></hierarchy>")
    );
}

#[test]
fn rejects_invalid_xml() {
    let error = parse_ui("<hierarchy>", None, false).expect_err("invalid xml");
    assert_eq!(error.code, "UI_PARSE_FAILED");
}

#[test]
fn snapshot_output_round_trips_through_json() {
    let parsed = parse_ui(SAMPLE, None, false).expect("valid hierarchy");
    let output = SnapshotOutput {
        snapshot_id: "s1".to_string(),
        package: parsed.package,
        screen: parsed.screen,
        elements: parsed.elements,
        node_count: parsed.node_count,
    };
    let value = serde_json::to_value(&output).expect("serialize");
    assert_eq!(value["elements"][0]["ref"], "@a1");
    assert_eq!(value["elements"][0]["clickable"], true);
    let loaded: SnapshotOutput = serde_json::from_value(value).expect("deserialize");
    assert_eq!(loaded.snapshot_id, "s1");
    assert_eq!(loaded.elements[0].text.as_deref(), Some("OK"));
    assert!(loaded.elements[0].flags.clickable);
}

#[test]
fn matches_text_descriptions_and_ids() {
    let parsed = parse_ui(SAMPLE, None, false).expect("valid hierarchy");
    let mut args = find_args();
    args.text = Some("ok".to_string());
    assert!(element_matches(&parsed.elements[0], &args));
    args.exact = true;
    assert!(!element_matches(&parsed.elements[0], &args));

    let mut element = parsed.elements[0].clone();
    element.text = Some("Settings".to_string());
    element.desc = Some("Open settings".to_string());
    let mut args = find_args();
    args.text = Some("open".to_string());
    assert!(element_matches(&element, &args));

    let mut args = find_args();
    args.resource_id = Some("id/name".to_string());
    assert!(element_matches(&parsed.elements[2], &args));

    let mut args = find_args();
    args.desc = Some("name".to_string());
    assert!(element_matches(&parsed.elements[2], &args));

    let mut args = find_args();
    args.class = Some("Button".to_string());
    assert!(element_matches(&parsed.elements[0], &args));
}

#[test]
fn refs_require_a_snapshot_id_and_nonempty_bounds() {
    let directory = tempfile::tempdir().expect("temp dir");
    let adb = Adb::discover(None, None);
    let policy = Policy::default();
    let ctx = Ctx {
        adb: &adb,
        policy: &policy,
        home: directory.path(),
    };
    let element = UiElement {
        reference: "@a1".to_string(),
        role: "Button".to_string(),
        class: "android.widget.Button".to_string(),
        text: Some("OK".to_string()),
        desc: None,
        resource_id: None,
        hint: None,
        package: Some("com.example".to_string()),
        bounds: Bounds::default(),
        center: Point::default(),
        flags: UiFlags::default(),
    };
    let snapshot = SnapshotOutput {
        snapshot_id: "s1".to_string(),
        package: "com.example".to_string(),
        screen: ScreenSize::default(),
        elements: vec![element],
        node_count: 1,
    };
    ctx.store().save(&snapshot).expect("save snapshot");

    let error = resolve_ref(&ctx, "@a1", None).expect_err("snapshot id required");
    assert_eq!(error.code, "SNAPSHOT_ID_REQUIRED");
    let error = resolve_ref(&ctx, "@a1", Some("s1")).expect_err("empty bounds");
    assert_eq!(error.code, "INVALID_ARGS");

    let mut snapshot = snapshot;
    snapshot.elements[0].bounds = Bounds {
        left: 10,
        top: 20,
        right: 30,
        bottom: 60,
    };
    snapshot.elements[0].center = Point { x: 20, y: 40 };
    ctx.store().save(&snapshot).expect("save snapshot");
    let (point, snapshot_id) = resolve_ref(&ctx, "@a1", Some("s1")).expect("resolve ref");
    assert_eq!(point, Point { x: 20, y: 40 });
    assert_eq!(snapshot_id, "s1");
}
