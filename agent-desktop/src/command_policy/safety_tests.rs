use super::*;
use clap::Parser;

fn policy(read_only: bool, allowed: Option<Vec<&str>>) -> SafetyPolicy {
    SafetyPolicy {
        read_only,
        allowed_commands: allowed.map(|entries| entries.into_iter().map(str::to_string).collect()),
        audit_log: None,
    }
}

fn command(arguments: &[&str]) -> Commands {
    let mut argv = vec!["agent-desktop"];
    argv.extend_from_slice(arguments);
    crate::cli::Cli::try_parse_from(argv)
        .expect("command parses")
        .command
        .expect("subcommand is present")
}

fn audited_policy(test: &str) -> (SafetyPolicy, PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "agent-desktop-safety-{test}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("audit.jsonl"))
        .unwrap();
    let policy = SafetyPolicy {
        read_only: false,
        allowed_commands: None,
        audit_log: Some(Mutex::new(file)),
    };
    (policy, dir)
}

fn logged_record(policy: SafetyPolicy, dir: &std::path::Path) -> (String, Value) {
    drop(policy);
    let logged = std::fs::read_to_string(dir.join("audit.jsonl")).unwrap();
    std::fs::remove_dir_all(dir).ok();
    let record = serde_json::from_str(logged.trim()).unwrap();
    (logged, record)
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
fn read_only_allows_a_command_that_only_writes_a_file_the_caller_named() {
    let read_only = policy(true, None);

    for arguments in [
        &["screenshot", "--window-id", "w-123", "shot.png"][..],
        &["clipboard-get", "--out", "clipboard.png"],
        &["trace", "export"],
    ] {
        assert!(
            read_only.check(&command(arguments)).is_ok(),
            "{arguments:?}"
        );
    }
}

#[test]
fn read_only_still_denies_every_command_that_acts_on_the_desktop() {
    let read_only = policy(true, None);

    for arguments in [
        &["click", "@e1"][..],
        &["clipboard-set", "text"],
        &["session", "start"],
        &["permissions", "--request"],
    ] {
        let error = read_only.check(&command(arguments)).unwrap_err();
        assert_eq!(error.code(), "POLICY_DENIED", "{arguments:?}");
    }
}

#[test]
fn no_refusal_suggests_removing_the_policy() {
    let refusals = [
        policy(true, None).check_command("batch", true),
        policy(false, Some(vec!["snapshot"])).check_command("mouse-click", true),
        policy(true, None).check_command("mouse-click", true),
    ];

    for refusal in refusals {
        let error = refusal.unwrap_err();
        let suggestion = error.suggestion().unwrap_or_default().to_ascii_lowercase();
        assert!(!suggestion.is_empty());
        assert!(!suggestion.contains("unset"), "{suggestion}");
    }
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
    let (policy, dir) = audited_policy("direct");
    let result: Result<Value, AppError> = Ok(json!({ "secret": "not-for-the-log" }));

    policy.audit("snapshot", &result, Duration::from_millis(3));
    let (logged, record) = logged_record(policy, &dir);

    assert_eq!(record["command"], "snapshot");
    assert_eq!(record["ok"], true);
    assert!(!logged.contains("not-for-the-log"));
}

#[test]
fn a_relaunched_command_is_audited_with_its_exit_code() {
    let (policy, dir) = audited_policy("relaunched");

    policy.audit_relaunched("mouse-click", 1, Duration::from_millis(7));
    let (_, record) = logged_record(policy, &dir);

    assert_eq!(record["command"], "mouse-click");
    assert_eq!(record["ok"], false);
    assert_eq!(record["relaunched"], true);
    assert_eq!(record["exit_code"], 1);
}
