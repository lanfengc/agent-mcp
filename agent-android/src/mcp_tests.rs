use super::{LEGACY_VERSIONS, respond};
use crate::adb::Adb;
use crate::commands::Ctx;
use crate::policy::Policy;
use serde_json::{Value, json};

fn call(message: Value) -> Option<Value> {
    let adb = Adb::discover(None, None);
    let policy = Policy::default();
    let home = std::env::temp_dir();
    let ctx = Ctx {
        adb: &adb,
        policy: &policy,
        home: &home,
    };
    respond(&message, &ctx)
}

fn initialize(version: &str) -> Value {
    call(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": version,
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "1.0" },
        },
    }))
    .expect("initialize response")
}

#[test]
fn negotiates_a_known_protocol_version() {
    let response = initialize("2024-11-05");
    assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(response["result"]["serverInfo"]["name"], "agent-android");
    assert!(response["result"]["capabilities"]["tools"].is_object());
    assert!(response["result"]["instructions"].is_string());
}

#[test]
fn falls_back_for_an_unknown_protocol_version() {
    let response = initialize("1900-01-01");
    assert_eq!(response["result"]["protocolVersion"], LEGACY_VERSIONS[0]);
}

#[test]
fn lists_android_tools() {
    let response = call(json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {},
    }))
    .expect("tools list");
    let tools = response["result"]["tools"].as_array().expect("tools array");
    assert!(tools.len() >= 20);
    assert!(tools.iter().any(|tool| tool["name"] == "android_tap"));
    assert!(tools.iter().any(|tool| tool["name"] == "android_snapshot"));
    assert!(
        tools
            .iter()
            .all(|tool| tool["inputSchema"]["type"] == "object")
    );
}

#[test]
fn rejects_unknown_methods() {
    let response = call(json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/unknown",
        "params": {},
    }))
    .expect("error response");
    assert_eq!(response["error"]["code"], -32601);
}

#[test]
fn unknown_tools_are_tool_errors_not_rpc_errors() {
    let response = call(json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": { "name": "android_nope", "arguments": {} },
    }))
    .expect("tool error response");
    assert_eq!(response["result"]["isError"], true);
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    assert!(text.contains("UNKNOWN_TOOL"));
}

#[test]
fn ignores_notifications() {
    let response = call(json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {},
    }));
    assert!(response.is_none());
}

#[test]
fn image_tool_results_include_an_image_block_and_text() {
    let result = super::tool_success(crate::tools::ToolResult {
        command: "screenshot",
        data: json!({ "width": 1, "height": 2, "bytes": 4 }),
        image: Some(vec![1, 2, 3, 4]),
    });
    assert_eq!(result["resultType"], "complete");
    assert_eq!(result["content"][0]["type"], "image");
    assert_eq!(result["content"][0]["mimeType"], "image/png");
    assert_eq!(result["content"][1]["type"], "text");
}
