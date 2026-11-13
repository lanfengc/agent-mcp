use super::{
    AppsArgs, ClearDataArgs, FindArgs, InstallArgs, KeyArgs, LogcatArgs, LongPressArgs,
    ScreenshotArgs, ScrollArgs, SnapshotArgs, SwipeArgs, TapArgs, TypeArgs, UninstallArgs,
    WaitArgs,
};
use serde_json::json;

fn parse<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> T {
    serde_json::from_value(value).expect("valid tool arguments")
}

#[test]
fn observation_args_accept_minimal_json() {
    let _: SnapshotArgs = parse(json!({}));
    let _: ScreenshotArgs = parse(json!({}));
    let _: LogcatArgs = parse(json!({}));
    let apps: AppsArgs = parse(json!({}));
    assert!(!apps.all);
    let find: FindArgs = parse(json!({ "text": "Sign in" }));
    assert!(!find.exact);
}

#[test]
fn interaction_args_accept_minimal_json() {
    let _: TapArgs = parse(json!({}));
    let long_press: LongPressArgs = parse(json!({}));
    assert_eq!(long_press.duration_ms, 600);
    let swipe: SwipeArgs = parse(json!({ "x1": 1, "y1": 2, "x2": 3, "y2": 4 }));
    assert_eq!(swipe.duration_ms, 300);
    let type_args: TypeArgs = parse(json!({ "text": "hello" }));
    assert!(!type_args.clear);
    let _: KeyArgs = parse(json!({ "key": "ENTER" }));
    let _: ScrollArgs = parse(json!({ "direction": "up" }));
    let wait: WaitArgs = parse(json!({ "text": "Done" }));
    assert_eq!(wait.timeout_ms, 10000);
    assert_eq!(wait.interval_ms, 300);
}

#[test]
fn app_args_accept_minimal_json() {
    let install: InstallArgs = parse(json!({ "path": "app.apk" }));
    assert!(!install.replace);
    assert_eq!(install.path.to_string_lossy(), "app.apk");
    let uninstall: UninstallArgs = parse(json!({ "package": "com.example.app" }));
    assert!(!uninstall.keep_data);
    assert!(!uninstall.confirm);
    let clear: ClearDataArgs = parse(json!({ "package": "com.example.app" }));
    assert!(!clear.confirm);
}
