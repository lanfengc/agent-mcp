use super::*;

fn policy(read_only: bool, allowed: Option<Vec<&str>>) -> SafetyPolicy {
    SafetyPolicy {
        read_only,
        allowed_commands: allowed.map(|entries| entries.into_iter().map(str::to_string).collect()),
        audit_log: None,
    }
}

fn mouse_click() -> Commands {
    Commands::MouseClick(crate::cli_args::actions::MouseClickArgs {
        xy: "10,20".into(),
        button: "left".into(),
        count: 1,
        modifiers: Vec::new(),
        background: crate::cli_args::actions::BackgroundClickFlags {
            background: true,
            window_id: Some("w-1".into()),
            capture_id: None,
            restore_minimized: false,
            cursor_fallback: false,
            semantic: false,
        },
    })
}

#[test]
fn read_only_denies_a_mutating_command() {
    let error = policy(true, None).check(&mouse_click()).unwrap_err();

    assert_eq!(error.code(), "POLICY_DENIED");
    assert!(policy(true, None).check(&Commands::Version).is_ok());
}

#[test]
fn read_only_allows_a_read_command() {
    assert!(policy(true, None).check_command("snapshot", false).is_ok());
}

#[test]
fn allowlist_denies_a_command_that_is_not_listed() {
    let error = policy(false, Some(vec!["snapshot"]))
        .check_command("mouse-click", true)
        .unwrap_err();

    assert_eq!(error.code(), "POLICY_DENIED");
}

#[test]
fn allowlist_admits_a_listed_command() {
    assert!(
        policy(false, Some(vec!["snapshot", "mouse-click"]))
            .check_command("mouse-click", true)
            .is_ok()
    );
}

#[test]
fn a_restrictive_policy_refuses_batch() {
    let error = policy(true, None).check_command("batch", true).unwrap_err();

    assert_eq!(error.code(), "POLICY_DENIED");
    assert!(
        error
            .suggestion()
            .unwrap_or_default()
            .contains("individually")
    );
}

#[test]
fn an_unrestricted_policy_allows_mutation() {
    assert!(
        policy(false, None)
            .check_command("mouse-click", true)
            .is_ok()
    );
}

#[test]
fn audit_records_outcome_without_the_command_output() {
    let dir = std::env::temp_dir().join(format!(
        "agent-desktop-safety-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("audit.jsonl");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .unwrap();
    let policy = SafetyPolicy {
        read_only: false,
        allowed_commands: None,
        audit_log: Some(Mutex::new(file)),
    };
    let result: Result<Value, AppError> = Ok(json!({ "secret": "not-for-the-log" }));

    policy.audit("snapshot", &result, Duration::from_millis(3));
    drop(policy);
    let logged = std::fs::read_to_string(&path).unwrap();

    assert!(logged.contains("\"command\":\"snapshot\""));
    assert!(logged.contains("\"ok\":true"));
    assert!(!logged.contains("not-for-the-log"));
    std::fs::remove_dir_all(&dir).ok();
}
