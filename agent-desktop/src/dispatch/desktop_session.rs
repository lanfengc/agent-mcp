//! Keeps a command on a desktop the signed-in user can actually see.
//!
//! A process in session 0 still accepts MCP requests, but every desktop call
//! it makes lands on a desktop nobody can see. Rather than act there, the
//! command is relaunched inside the logged-in session when the caller holds
//! `SeTcbPrivilege`; a workstation whose input belongs to the secure desktop
//! is refused. Commands that never touch the desktop run in place.
//!
//! The relaunched child starts from the logged-in user's environment, so it
//! sees none of this process's execution-policy variables. The caller checks
//! the policy before it asks for a placement, and every outcome decided here
//! is written to the caller's audit log.

use std::process::ExitCode;
use std::time::Instant;

use agent_desktop_core::{AdapterError, AppError, DeliverySemantics, ErrorCode};

use crate::command_policy::safety::SafetyPolicy;

/// Returns an exit code when the command must not run here, or `None` when it
/// may proceed. Call it only after `safety` has admitted the command.
pub(crate) fn guard(cmd_name: &str, safety: &SafetyPolicy) -> Option<ExitCode> {
    use agent_desktop_windows::DesktopSession;

    if matches!(cmd_name, "version" | "skills" | "session" | "trace") {
        return None;
    }
    let started = Instant::now();
    let refused = match agent_desktop_windows::desktop_session() {
        DesktopSession::Interactive => return None,
        DesktopSession::OtherSession { .. } => {
            match agent_desktop_windows::relaunch_in_active_session() {
                Ok(code) => {
                    safety.audit_relaunched(cmd_name, code, started.elapsed());
                    return Some(child_exit_code(code));
                }
                Err(error) => error.into(),
            }
        }
        DesktopSession::Locked(desktop) => refusal(
            "The interactive desktop is locked, so the command cannot act on it",
            &format!(
                "The input desktop is '{desktop}'. Unlock the session or dismiss the UAC prompt, then retry"
            ),
        ),
        DesktopSession::NoActiveSession => refusal(
            "No session has a user at its desktop, so there is no desktop to act on",
            "Log in at the console or reconnect the remote desktop session, then retry the command",
        ),
        DesktopSession::Unreadable(code) => AdapterError::new(
            ErrorCode::PermDenied,
            "The input desktop could not be read, so the command cannot act on it",
        )
        .with_suggestion("Retry after the session switch completes")
        .with_details(serde_json::json!({ "win32_error": code }))
        .with_disposition(DeliverySemantics::not_delivered())
        .into(),
    };
    let result = Err(refused);
    safety.audit(cmd_name, &result, started.elapsed());
    Some(crate::finish(cmd_name, result))
}

fn refusal(message: &str, suggestion: &str) -> AppError {
    AdapterError::new(ErrorCode::PermDenied, message)
        .with_suggestion(suggestion)
        .with_disposition(DeliverySemantics::not_delivered())
        .into()
}

fn child_exit_code(code: u32) -> ExitCode {
    match u8::try_from(code) {
        Ok(code) => ExitCode::from(code),
        Err(_) => ExitCode::FAILURE,
    }
}
