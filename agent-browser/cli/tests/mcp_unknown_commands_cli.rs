use serde_json::{json, Value};
use std::process::Command;
use tempfile::TempDir;

#[test]
fn mcp_batch_rejects_unknown_cookie_subcommands() {
    let home = TempDir::new().unwrap();
    let session = "mcp-cookie-parse-test";
    let batch_args = json!({
        "session": session,
        "commands": [["cookies", "delete", "foo"]]
    })
    .to_string();
    let mut command = Command::new(env!("CARGO_BIN_EXE_agent-browser"));
    command
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env(
            "SystemRoot",
            std::env::var_os("SystemRoot").unwrap_or_default(),
        )
        .env("WINDIR", std::env::var_os("WINDIR").unwrap_or_default())
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("AGENT_BROWSER_SOCKET_DIR", home.path().join("sockets"))
        .env("AGENT_BROWSER_IDLE_TIMEOUT_MS", "200")
        .env("NO_COLOR", "1")
        .current_dir(home.path())
        .args(["mcp", "call", "agent_browser_batch", &batch_args]);
    let output = command.output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let batch: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(batch["isError"], true, "{batch}");
    assert_eq!(batch["structuredContent"]["exitCode"], 1, "{batch}");
    let error = batch["structuredContent"]["response"][0]["error"]
        .as_str()
        .unwrap_or_else(|| panic!("missing error string: {batch}"));
    assert!(error.contains("Unknown subcommand: delete"), "{batch}");
}
