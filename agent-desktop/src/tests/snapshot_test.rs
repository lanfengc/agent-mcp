#[cfg(test)]
mod tests {
    use std::process::Command;

    fn run(args: &[&str]) -> serde_json::Value {
        let output = Command::new(env!("CARGO_BIN_EXE_agent-desktop"))
            .args(args)
            .output()
            .expect("failed to run agent-desktop");
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "agent-desktop output is not JSON: {error}; stderr={}",
                String::from_utf8_lossy(&output.stderr)
            )
        })
    }

    #[test]
    fn snapshot_invalid_root_ref_format_returns_invalid_args() {
        let json = run(&["snapshot", "--app", "notepad.exe", "--root", "bad-ref"]);
        assert_eq!(json["ok"], false);
        assert_eq!(json["error"]["code"], "INVALID_ARGS");
    }

    #[test]
    fn list_apps_inventory_matches_the_platform() {
        let json = run(&["list-apps"]);
        assert_eq!(json["ok"], true, "list-apps is live on Windows: {json}");
        assert!(
            json["data"]["apps"].is_array(),
            "list-apps returns an apps array on Windows: {json}"
        );
    }
}
