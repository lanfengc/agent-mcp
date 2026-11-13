use super::{ToolFailure, call};
use crate::adb::Adb;
use crate::commands::Ctx;
use crate::policy::Policy;
use crate::tool_definitions::definitions;
use serde_json::{Map, Value, json};
use std::path::PathBuf;

const PARSE_FAILURE: &str = "Invalid tool arguments";

fn sample(property: &Value) -> Value {
    if let Some(choices) = property["enum"].as_array() {
        return choices[0].clone();
    }
    match property["type"].as_str() {
        Some("integer") => json!(1),
        Some("boolean") => json!(true),
        _ => json!("sample"),
    }
}

fn arguments(tool: &Value, only_required: bool) -> Value {
    let schema = &tool["inputSchema"];
    let required = schema["required"].as_array().expect("required list");
    let properties = schema["properties"].as_object().expect("properties");
    let mut arguments = Map::new();
    for (name, property) in properties {
        if !only_required || required.iter().any(|entry| entry == name) {
            arguments.insert(name.clone(), sample(property));
        }
    }
    Value::Object(arguments)
}

fn failure(name: &str, arguments: Value) -> Option<ToolFailure> {
    let adb = Adb::discover(Some(PathBuf::from("no-such-adb-binary")), None);
    let policy = Policy {
        read_only: true,
        allow_destructive: false,
    };
    let home = tempfile::tempdir().expect("home");
    let ctx = Ctx {
        adb: &adb,
        policy: &policy,
        home: home.path(),
    };
    call(name, arguments, &ctx).err()
}

fn message(failure: &Option<ToolFailure>) -> &str {
    failure
        .as_ref()
        .map_or("", |failure| failure.error.message.as_str())
}

fn tool_name(tool: &Value) -> &str {
    tool["name"].as_str().expect("tool name")
}

#[test]
fn every_schema_property_is_accepted_by_its_argument_struct() {
    for tool in definitions() {
        let name = tool_name(&tool);
        let failure = failure(name, arguments(&tool, false));
        let code = failure.as_ref().map(|failure| failure.error.code);
        assert_ne!(code, Some("UNKNOWN_TOOL"), "{name}");
        let message = message(&failure);
        assert!(!message.starts_with(PARSE_FAILURE), "{name}: {message}");
    }
}

#[test]
fn the_required_list_is_enough_to_parse() {
    for tool in definitions() {
        let name = tool_name(&tool);
        let failure = failure(name, arguments(&tool, true));
        let message = message(&failure);
        assert!(!message.starts_with(PARSE_FAILURE), "{name}: {message}");
    }
}

#[test]
fn an_argument_outside_the_schema_is_refused_by_name() {
    for tool in definitions() {
        let name = tool_name(&tool);
        let mut arguments = arguments(&tool, true);
        arguments["no_such_argument"] = json!(true);
        let failure = failure(name, arguments).expect("a refusal");
        let message = failure.error.message;
        assert_eq!(failure.error.code, "INVALID_ARGS", "{name}");
        assert!(message.starts_with(PARSE_FAILURE), "{name}: {message}");
        assert!(message.contains("no_such_argument"), "{name}: {message}");
    }
}

#[test]
fn every_schema_says_that_other_arguments_are_refused() {
    for tool in definitions() {
        assert_eq!(
            tool["inputSchema"]["additionalProperties"],
            json!(false),
            "{}",
            tool_name(&tool)
        );
    }
}

#[test]
fn a_failure_names_the_command_the_cli_uses() {
    let commands = [
        ("android_list_devices", "devices"),
        ("android_device_info", "info"),
        ("android_foreground_app", "foreground"),
        ("android_list_apps", "apps"),
        ("android_launch_app", "launch"),
        ("android_terminate_app", "terminate"),
        ("android_install_app", "install"),
        ("android_uninstall_app", "uninstall"),
        ("android_clear_app_data", "clear_data"),
        ("android_long_press", "long_press"),
        ("android_open_url", "open_url"),
        ("android_tap", "tap"),
    ];
    for (tool, command) in commands {
        let failure = failure(tool, json!({ "no_such_argument": true })).expect("a refusal");
        assert_eq!(failure.command, command, "{tool}");
    }
}

#[test]
fn an_unknown_tool_is_named_in_its_failure() {
    let failure = failure("android_nope", json!({})).expect("a refusal");
    assert_eq!(failure.error.code, "UNKNOWN_TOOL");
    assert_eq!(failure.command, "nope");
}
