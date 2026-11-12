//! Keeps a command on a desktop the signed-in user can actually see.
//!
//! A process in session 0 still accepts MCP requests, but every desktop call
//! it makes lands on a desktop nobody can see. Rather than act there, the
//! command is relaunched inside the logged-in session when the caller holds
//! `SeTcbPrivilege`; a locked or UAC-secured workstation is refused. Commands
//! that never touch the desktop run in place.

use std::process::ExitCode;

use agent_desktop_core::{AdapterError, AppError, DeliverySemantics, ErrorCode};

/// Returns an exit code when the command must not run here, or `None` when it
/// may proceed.
pub(crate) fn guard(cmd_name: &str) -> Option<ExitCode> {
    use agent_desktop_windows::DesktopSession;

    if matches!(cmd_name, "version" | "skills" | "session" | "trace") {
        return None;
    }
    match agent_desktop_windows::desktop_session() {
        DesktopSession::Interactive => None,
        DesktopSession::OtherSession { .. } => {
            Some(match agent_desktop_windows::relaunch_in_active_session() {
                Ok(code) => child_exit_code(code),
                Err(error) => crate::finish(cmd_name, Err(error.into())),
            })
        }
        DesktopSession::Locked(desktop) => Some(crate::finish(
            cmd_name,
            Err(refusal(
                "The interactive desktop is locked, so the command cannot act on it",
                &format!(
                    "The input desktop is '{desktop}'. Unlock the session or dismiss the UAC prompt, then retry"
                ),
            )),
        )),
        DesktopSession::NoActiveSession => Some(crate::finish(
            cmd_name,
            Err(refusal(
                "No user is logged on to an interactive desktop",
                "Log in to the console session, then retry the command",
            )),
        )),
        DesktopSession::Unreadable(code) => Some(crate::finish(
            cmd_name,
            Err(AdapterError::new(
                ErrorCode::PermDenied,
                "The input desktop could not be read, so the command cannot act on it",
            )
            .with_suggestion("Retry after the session switch completes")
            .with_details(serde_json::json!({ "win32_error": code }))
            .with_disposition(DeliverySemantics::not_delivered())
            .into()),
        )),
    }
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
