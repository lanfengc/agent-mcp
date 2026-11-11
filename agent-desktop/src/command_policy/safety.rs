//! Environment-driven execution policy and audit trail.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use agent_desktop_core::{AdapterError, AppError, DeliverySemantics, ErrorCode};
use serde_json::{Value, json};

use crate::cli::Commands;

const READ_ONLY_ENV: &str = "AGENT_DESKTOP_READ_ONLY";
const ALLOWED_COMMANDS_ENV: &str = "AGENT_DESKTOP_ALLOWED_COMMANDS";
const AUDIT_LOG_ENV: &str = "AGENT_DESKTOP_AUDIT_LOG";

pub(crate) struct SafetyPolicy {
    read_only: bool,
    allowed_commands: Option<Vec<String>>,
    audit_log: Option<Mutex<File>>,
}

impl SafetyPolicy {
    pub(crate) fn from_env() -> Result<Self, AppError> {
        let read_only = env_flag(READ_ONLY_ENV)?;
        let allowed_commands = allowed_commands()?;
        let audit_log = audit_log()?;
        Ok(Self {
            read_only,
            allowed_commands,
            audit_log,
        })
    }

    pub(crate) fn check(&self, cmd: &Commands) -> Result<(), AppError> {
        self.check_command(cmd.name(), cmd.is_mutating())
    }

    pub(crate) fn audit(&self, command: &str, result: &Result<Value, AppError>, elapsed: Duration) {
        let Some(log) = self.audit_log.as_ref() else {
            return;
        };
        let record = json!({
            "ts_ms": now_ms(),
            "command": command,
            "ok": result.is_ok(),
            "error": result.as_ref().err().map(AppError::code),
            "duration_ms": u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
        });
        let Ok(mut file) = log.lock() else {
            return;
        };
        let _ = writeln!(file, "{record}");
        let _ = file.flush();
    }

    fn check_command(&self, command: &str, mutating: bool) -> Result<(), AppError> {
        if self.restrictive() && command == "batch" {
            return Err(denied(
                "batch is disabled while a safety policy is active",
                "Run the allowed commands individually, or unset the safety policy.",
            ));
        }
        if let Some(allowed) = self.allowed_commands.as_ref()
            && !allowed.iter().any(|entry| entry == command)
        {
            return Err(denied(
                format!("Command '{command}' is not in {ALLOWED_COMMANDS_ENV}"),
                "Add the command name to the allowlist, or unset it.",
            ));
        }
        if self.read_only && mutating {
            return Err(denied(
                format!("Command '{command}' changes application state in read-only mode"),
                "Unset AGENT_DESKTOP_READ_ONLY only for the interactive phase.",
            ));
        }
        Ok(())
    }

    fn restrictive(&self) -> bool {
        self.read_only || self.allowed_commands.is_some()
    }
}

fn env_flag(name: &str) -> Result<bool, AppError> {
    let Some(raw) = std::env::var_os(name) else {
        return Ok(false);
    };
    match raw.to_string_lossy().trim().to_ascii_lowercase().as_str() {
        "" | "0" | "false" | "no" | "off" => Ok(false),
        "1" | "true" | "yes" | "on" => Ok(true),
        _ => Err(AppError::invalid_input_with_suggestion(
            format!("{name} must be one of 0, 1, true, false, yes, no, on, off"),
            "Unset the variable or give it a supported boolean value.",
        )),
    }
}

fn allowed_commands() -> Result<Option<Vec<String>>, AppError> {
    let Some(raw) = std::env::var_os(ALLOWED_COMMANDS_ENV) else {
        return Ok(None);
    };
    let commands = raw
        .to_string_lossy()
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    if commands.is_empty() {
        return Err(AppError::invalid_input_with_suggestion(
            format!("{ALLOWED_COMMANDS_ENV} must name at least one command"),
            "List command names separated by commas, or unset the variable.",
        ));
    }
    Ok(Some(commands))
}

fn audit_log() -> Result<Option<Mutex<File>>, AppError> {
    let Some(raw) = std::env::var_os(AUDIT_LOG_ENV) else {
        return Ok(None);
    };
    let path = PathBuf::from(raw);
    if !path.is_absolute() {
        return Err(AppError::invalid_input_with_suggestion(
            format!("{AUDIT_LOG_ENV} must be an absolute path"),
            "Use an absolute path to a writable audit file.",
        ));
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| {
            AppError::invalid_input_with_suggestion(
                format!("{AUDIT_LOG_ENV} could not be opened: {error}"),
                "Choose a writable absolute path for the audit log.",
            )
        })?;
    Ok(Some(Mutex::new(file)))
}

fn denied(message: impl Into<String>, suggestion: impl Into<String>) -> AppError {
    AdapterError::new(ErrorCode::PolicyDenied, message)
        .with_suggestion(suggestion)
        .with_disposition(DeliverySemantics::not_delivered())
        .into()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "safety_tests.rs"]
mod tests;
