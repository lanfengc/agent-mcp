use serde_json::{Value, json};

pub fn definitions() -> Vec<Value> {
    vec![
        tool(
            "android_list_devices",
            "List Android devices and emulators known to adb. Call this before other tools when the device is uncertain.",
            json!({}),
            &[],
        ),
        tool(
            "android_device_info",
            "Show the selected device's model, manufacturer, Android version, SDK, ABI and screen size.",
            json!({}),
            &[],
        ),
        tool(
            "android_doctor",
            "Diagnose adb discovery, device readiness and the active execution policy.",
            json!({}),
            &[],
        ),
        tool(
            "android_connect",
            "Connect adb to a wireless device. Pass host:port, or host plus port. For first-time pairing, pass pair_code and the pairing endpoint as host:port; that call only pairs, so call again without pair_code and with the connection port, which differs from the pairing port.",
            json!({
                "host": string_prop("Device host or host:port"),
                "port": integer_prop("Port when host has no port; defaults to 5555"),
                "pair_code": string_prop("Six-digit wireless debugging pairing code"),
            }),
            &["host"],
        ),
        tool(
            "android_screenshot",
            "Capture a PNG screenshot. Without path, the image is returned as an MCP image content block. Screenshots can contain personal data; keep them local.",
            json!({
                "path": string_prop("Optional local path to write the PNG instead of returning it inline"),
            }),
            &[],
        ),
        tool(
            "android_snapshot",
            "Dump the accessibility hierarchy and return actionable elements with refs such as @a12. Refs are positional within this snapshot; pass its snapshot_id when acting.",
            json!({
                "all": boolean_prop("Include every node, including containers with no label"),
                "max_depth": integer_prop("Limit traversal depth; 0 keeps only the root node"),
            }),
            &[],
        ),
        tool(
            "android_find",
            "Take a fresh UI snapshot and return elements matching all supplied criteria. Each result includes a snapshot-bound ref.",
            json!({
                "text": string_prop("Match visible text or content description"),
                "resource_id": string_prop("Match resource id, for example com.example:id/button"),
                "desc": string_prop("Match content description only"),
                "class": string_prop("Match widget class or short role"),
                "exact": boolean_prop("Require an exact, case-sensitive match"),
            }),
            &[],
        ),
        tool(
            "android_foreground_app",
            "Return the foreground package and activity.",
            json!({}),
            &[],
        ),
        tool(
            "android_list_apps",
            "List installed package names. By default only third-party apps are returned.",
            json!({
                "all": boolean_prop("Include system packages"),
            }),
            &[],
        ),
        tool(
            "android_logcat",
            "Read recent logcat lines. Logs can contain personal data; keep them local.",
            json!({
                "lines": integer_prop("Number of lines, 1 to 5000; defaults to 200"),
                "filter": string_prop("Only return lines containing this text"),
            }),
            &[],
        ),
        tool(
            "android_tap",
            "Tap by snapshot ref or device pixel coordinates. Prefer a ref from android_snapshot; coordinates are a fallback.",
            json!({
                "x": integer_prop("X coordinate in device pixels"),
                "y": integer_prop("Y coordinate in device pixels"),
                "ref": string_prop("Element ref from a snapshot; pass snapshot_id too"),
                "snapshot_id": string_prop("Snapshot id that produced the ref; required with ref"),
            }),
            &[],
        ),
        tool(
            "android_long_press",
            "Long-press by snapshot ref or coordinates.",
            json!({
                "x": integer_prop("X coordinate in device pixels"),
                "y": integer_prop("Y coordinate in device pixels"),
                "ref": string_prop("Element ref from a snapshot; pass snapshot_id too"),
                "snapshot_id": string_prop("Snapshot id that produced the ref; required with ref"),
                "duration_ms": integer_prop("Press duration in milliseconds; defaults to 600, at least 50"),
            }),
            &[],
        ),
        tool(
            "android_swipe",
            "Swipe between two device pixel coordinates.",
            json!({
                "x1": integer_prop("Start X"),
                "y1": integer_prop("Start Y"),
                "x2": integer_prop("End X"),
                "y2": integer_prop("End Y"),
                "duration_ms": integer_prop("Swipe duration in milliseconds; defaults to 300"),
            }),
            &["x1", "y1", "x2", "y2"],
        ),
        tool(
            "android_type",
            "Type into the focused field, optionally tapping a snapshot ref first. ASCII only unless ADBKeyboard is the selected input method.",
            json!({
                "text": string_prop("Text to type"),
                "ref": string_prop("Tap this snapshot ref before typing; pass snapshot_id too"),
                "snapshot_id": string_prop("Snapshot id that produced the ref; required with ref"),
                "clear": boolean_prop("Clear the focused field before typing"),
            }),
            &["text"],
        ),
        tool(
            "android_key",
            "Send an Android key event by name or number, for example ENTER, BACK, HOME or 66. A name outside the built-in list is passed to the device, which ignores a name it does not know; the result then carries a notice.",
            json!({
                "key": string_prop("Android key name or number"),
            }),
            &["key"],
        ),
        tool(
            "android_scroll",
            "Scroll the screen or a scrollable snapshot ref in one direction.",
            json!({
                "direction": json!({
                    "type": "string",
                    "enum": ["up", "down", "left", "right"],
                    "description": "Direction the view moves; down reveals what is below, as a scroll wheel and agent-desktop scroll do",
                }),
                "distance": integer_prop("Optional swipe distance in pixels"),
                "ref": string_prop("Scroll inside this ref instead of the whole screen; pass snapshot_id too"),
                "snapshot_id": string_prop("Snapshot id that produced the ref; required with ref"),
            }),
            &["direction"],
        ),
        tool(
            "android_wait",
            "Poll until text appears, a ref exists or a package is in the foreground.",
            json!({
                "text": string_prop("Wait until an element contains this text"),
                "ref": string_prop("Wait until this positional ref exists in a fresh snapshot; prefer text"),
                "foreground": string_prop("Wait until this package is in the foreground"),
                "timeout_ms": integer_prop("Timeout in milliseconds; defaults to 10000"),
                "interval_ms": integer_prop("Poll interval in milliseconds; defaults to 300, at least 50"),
            }),
            &[],
        ),
        tool(
            "android_launch_app",
            "Launch an app by package name through its launcher activity.",
            json!({
                "package": string_prop("Android package name"),
            }),
            &["package"],
        ),
        tool(
            "android_terminate_app",
            "Force-stop an app by package name.",
            json!({
                "package": string_prop("Android package name"),
            }),
            &["package"],
        ),
        tool(
            "android_install_app",
            "Install an APK from this computer. The path must end in .apk and contain a ZIP/APK header. Recent Android versions replace an existing package either way; older ones need replace=true.",
            json!({
                "path": string_prop("Path to an .apk file on this computer"),
                "replace": boolean_prop("Pass -r, which older Android versions need to install over an existing package"),
            }),
            &["path"],
        ),
        tool(
            "android_uninstall_app",
            "Uninstall a package. Destructive: requires confirm=true unless AGENT_ANDROID_ALLOW_DESTRUCTIVE=1.",
            json!({
                "package": string_prop("Android package name"),
                "keep_data": boolean_prop("Keep application data and cache"),
                "confirm": boolean_prop("Confirm the destructive action"),
            }),
            &["package"],
        ),
        tool(
            "android_clear_app_data",
            "Clear application data. Destructive: requires confirm=true unless AGENT_ANDROID_ALLOW_DESTRUCTIVE=1.",
            json!({
                "package": string_prop("Android package name"),
                "confirm": boolean_prop("Confirm the destructive action"),
            }),
            &["package"],
        ),
        tool(
            "android_open_url",
            "Open an http or https URL in the device's default handler.",
            json!({
                "url": string_prop("URL to open"),
            }),
            &["url"],
        ),
    ]
}

fn tool(name: &str, description: &str, properties: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        },
    })
}

fn string_prop(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

fn integer_prop(description: &str) -> Value {
    json!({ "type": "integer", "description": description })
}

fn boolean_prop(description: &str) -> Value {
    json!({ "type": "boolean", "description": description })
}
