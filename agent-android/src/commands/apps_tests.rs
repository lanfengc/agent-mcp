use super::{
    Foreground, parse_foreground, parse_packages, validate_apk, validate_package, validate_url,
};

#[test]
fn prefers_top_resumed_activity() {
    let output = "\
  mResumedActivity: ActivityRecord{111 u0 com.example/.MainActivity t42}
  topResumedActivity=ActivityRecord{222 u0 com.android.settings/.Settings t43}
";
    let foreground = parse_foreground(output).expect("foreground");
    assert_eq!(
        foreground,
        Foreground {
            package: "com.android.settings".to_string(),
            activity: Some("com.android.settings.Settings".to_string()),
        }
    );
}

#[test]
fn falls_back_to_current_focus() {
    let output = "  mCurrentFocus=Window{abc u0 com.example/com.example.Dialog}\n";
    let foreground = parse_foreground(output).expect("foreground");
    assert_eq!(foreground.package, "com.example");
    assert_eq!(foreground.activity.as_deref(), Some("com.example.Dialog"));
}

#[test]
fn ignores_lines_without_an_activity() {
    assert!(parse_foreground("no activity here").is_none());
}

#[test]
fn parses_and_sorts_package_list() {
    let output = "package:com.zeta\npackage:com.alpha\npackage:com.alpha\n";
    assert_eq!(parse_packages(output), vec!["com.alpha", "com.zeta"]);
}

#[test]
fn validates_package_names() {
    assert!(validate_package("com.example.app").is_ok());
    assert!(validate_package("com.example_app").is_ok());
    assert!(validate_package("example").is_err());
    assert!(validate_package(".com.example").is_err());
    assert!(validate_package("com.example.").is_err());
    assert!(validate_package("com.exa mple").is_err());
}

#[test]
fn allows_only_http_urls() {
    assert!(validate_url("https://example.com/a?b=1&c=2").is_ok());
    assert!(validate_url("http://example.com").is_ok());
    assert!(validate_url("file:///sdcard/secret").is_err());
    assert!(validate_url("https://example.com/a b").is_err());
}

#[test]
fn accepts_only_apk_files() {
    let directory = tempfile::tempdir().expect("temp dir");
    let apk = directory.path().join("app.APK");
    std::fs::write(&apk, b"PK\x03\x04rest").expect("write apk");
    assert!(validate_apk(&apk).is_ok());

    let text = directory.path().join("secret.txt");
    std::fs::write(&text, b"secret").expect("write text");
    assert!(validate_apk(&text).is_err());

    let fake = directory.path().join("fake.apk");
    std::fs::write(&fake, b"secret").expect("write fake");
    assert!(validate_apk(&fake).is_err());
}
