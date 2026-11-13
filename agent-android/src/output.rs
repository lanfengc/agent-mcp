use crate::error::AppError;
use serde_json::{Value, json};

pub fn success(command: &str, data: Value) -> Value {
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "ok": true,
        "command": command,
        "data": data,
    })
}

pub fn failure(command: &str, error: &AppError) -> Value {
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "ok": false,
        "command": command,
        "error": error.to_value(),
    })
}
