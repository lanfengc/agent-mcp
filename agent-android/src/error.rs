use serde_json::{Value, json};
use std::fmt;

#[derive(Debug, Clone)]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
    pub suggestion: Option<String>,
    pub details: Option<Value>,
}

pub type Result<T> = std::result::Result<T, AppError>;

impl AppError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            suggestion: None,
            details: None,
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("INVALID_ARGS", message)
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn adb_not_found() -> Self {
        Self::new("ADB_NOT_FOUND", "Android Debug Bridge (adb) was not found").with_suggestion(
            "Install platform-tools with `winget install --id Google.PlatformTools -e`, \
                 or set AGENT_ANDROID_ADB to the full path of adb.exe",
        )
    }

    pub fn device_not_found() -> Self {
        Self::new(
            "DEVICE_NOT_FOUND",
            "No Android device or emulator is connected",
        )
        .with_suggestion("Enable USB debugging, connect the device, and confirm with `adb devices`")
    }

    pub fn device_failure(adb_output: &str) -> Option<Self> {
        let line = adb_output.lines().next().unwrap_or_default().trim();
        let reason = ["adb.exe: ", "adb: ", "error: "]
            .iter()
            .find_map(|prefix| line.strip_prefix(prefix))
            .unwrap_or(line);
        if reason == "no devices/emulators found" {
            return Some(Self::device_not_found());
        }
        if reason.starts_with("device '") && reason.ends_with("' not found") {
            return Some(
                Self::new("DEVICE_NOT_FOUND", format!("adb reports: {reason}")).with_suggestion(
                    "Compare --serial or ANDROID_SERIAL with the output of `adb devices`",
                ),
            );
        }
        let not_ready = [
            "device offline",
            "device unauthorized",
            "device still authorizing",
        ]
        .iter()
        .any(|state| reason.starts_with(state));
        not_ready.then(|| {
            Self::new("DEVICE_NOT_READY", format!("adb reports: {reason}")).with_suggestion(
                "Accept the USB debugging prompt on the device, or run `adb reconnect`",
            )
        })
    }

    pub fn multiple_devices(serials: &[String]) -> Self {
        Self::new(
            "MULTIPLE_DEVICES",
            "More than one Android device is ready; choose one with --serial",
        )
        .with_details(json!({ "serials": serials }))
        .with_suggestion("Pass --serial <serial> or set ANDROID_SERIAL")
    }

    pub fn timeout(seconds: u64, operation: &str) -> Self {
        Self::new(
            "TIMEOUT",
            format!("{operation} did not finish within {seconds} seconds"),
        )
        .with_suggestion("Check the USB connection and whether the device is responsive")
    }

    pub fn stale_ref(reference: &str, snapshot_id: &str) -> Self {
        Self::new(
            "STALE_REF",
            format!("Snapshot reference {reference} is not in snapshot {snapshot_id}"),
        )
        .with_suggestion("Call android_snapshot again and use a ref from the new result")
    }

    pub fn snapshot_not_found(snapshot_id: &str) -> Self {
        Self::new(
            "SNAPSHOT_NOT_FOUND",
            format!("Snapshot {snapshot_id} is not stored on this computer"),
        )
        .with_suggestion(format!(
            "Call android_snapshot again and use its snapshot_id; only the newest {} snapshots are kept",
            crate::state::MAX_SNAPSHOTS
        ))
    }

    pub fn snapshot_id_required() -> Self {
        Self::new(
            "SNAPSHOT_ID_REQUIRED",
            "A snapshot_id is required when acting on a snapshot ref",
        )
        .with_suggestion(
            "Pass the snapshot_id returned with the ref by android_snapshot, android_find or android_wait",
        )
    }

    pub fn to_value(&self) -> Value {
        let mut value = json!({
            "code": self.code,
            "message": self.message,
        });
        if let Some(suggestion) = &self.suggestion {
            value["suggestion"] = json!(suggestion);
        }
        if let Some(details) = &self.details {
            value["details"] = details.clone();
        }
        value
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
