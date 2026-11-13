//! Which desktop session this process can actually act on.
//!
//! `OpenInputDesktop` alone cannot tell a session-0 service desktop from the
//! logged-in one: both are named `Default`. The process is placed first by its
//! WTS session id and only then judged by the input desktop's name, so a
//! service is offered the relaunch path while a workstation whose input
//! belongs to the secure desktop is refused instead of silently acting on the
//! wrong desktop. A lock screen that still runs on `Default` is not told apart
//! from an unlocked session.
//!
//! A host can have several active sessions at once, one per connected user.
//! A process that runs in any of them acts on its own desktop; only a process
//! outside all of them is sent to the first.

use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, GetLastError};
use windows_sys::Win32::System::RemoteDesktop::{
    ProcessIdToSessionId, WTS_CURRENT_SERVER_HANDLE, WTS_SESSION_INFOW, WTSActive,
    WTSEnumerateSessionsW, WTSFreeMemory, WTSGetActiveConsoleSessionId,
};
use windows_sys::Win32::System::StationsAndDesktops::{
    CloseDesktop, DESKTOP_READOBJECTS, GetUserObjectInformationW, HDESK, OpenInputDesktop, UOI_NAME,
};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;

/// The desktop this process can reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopSession {
    /// The process runs in an active session on the `Default` desktop.
    Interactive,
    /// The process runs in an active session, but another desktop owns input;
    /// the value names it (`Winlogon` for the sign-in screen or a UAC prompt).
    Locked(String),
    /// The process runs outside every active session; `active` is the first
    /// one Windows lists.
    OtherSession { current: u32, active: u32 },
    /// No session is active: nobody is logged on, or every logged-on session
    /// is disconnected.
    NoActiveSession,
    /// The input desktop exists but could not be read; the value is the Win32
    /// error code.
    Unreadable(u32),
}

/// Classifies the desktop this process can act on.
pub fn desktop_session() -> DesktopSession {
    let active = active_session_ids();
    let current = current_session_id();
    session_verdict(current, &active).unwrap_or_else(|| desktop_verdict(input_desktop_name()))
}

fn session_verdict(current: Option<u32>, active: &[u32]) -> Option<DesktopSession> {
    let Some(&first_active) = active.first() else {
        return Some(DesktopSession::NoActiveSession);
    };
    match current {
        Some(current) if !active.contains(&current) => Some(DesktopSession::OtherSession {
            current,
            active: first_active,
        }),
        _ => None,
    }
}

fn desktop_verdict(desktop: Result<String, u32>) -> DesktopSession {
    match desktop {
        Ok(name) if name.eq_ignore_ascii_case("Default") => DesktopSession::Interactive,
        Ok(name) => DesktopSession::Locked(name),
        Err(ERROR_ACCESS_DENIED) => DesktopSession::Locked("secure desktop".to_string()),
        Err(code) => DesktopSession::Unreadable(code),
    }
}

/// The session a command outside every active session is relaunched into:
/// the first one Windows lists.
pub(crate) fn active_session_id() -> Option<u32> {
    active_session_ids().first().copied()
}

/// Every session with a user at its desktop right now, in the order Windows
/// lists them. Session 0 hosts services and is never one of them.
fn active_session_ids() -> Vec<u32> {
    let mut sessions: *mut WTS_SESSION_INFOW = std::ptr::null_mut();
    let mut count = 0u32;
    if unsafe { WTSEnumerateSessionsW(WTS_CURRENT_SERVER_HANDLE, 0, 1, &mut sessions, &mut count) }
        == 0
    {
        return console_session_id().into_iter().collect();
    }
    let active = (0..count as usize)
        .map(|index| unsafe { &*sessions.add(index) })
        .filter(|session| session.State == WTSActive && session.SessionId != 0)
        .map(|session| session.SessionId)
        .collect();
    unsafe { WTSFreeMemory(sessions as *mut core::ffi::c_void) };
    active
}

fn console_session_id() -> Option<u32> {
    let session = unsafe { WTSGetActiveConsoleSessionId() };
    (session != u32::MAX && session != 0).then_some(session)
}

fn current_session_id() -> Option<u32> {
    let mut session = 0u32;
    if unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) } == 0 {
        return None;
    }
    Some(session)
}

fn input_desktop_name() -> Result<String, u32> {
    let desktop = unsafe { OpenInputDesktop(0, 0, DESKTOP_READOBJECTS) };
    if desktop.is_null() {
        return Err(unsafe { GetLastError() });
    }
    let name = desktop_name(desktop);
    unsafe { CloseDesktop(desktop) };
    name
}

fn desktop_name(desktop: HDESK) -> Result<String, u32> {
    let mut needed = 0u32;
    unsafe {
        GetUserObjectInformationW(desktop, UOI_NAME, std::ptr::null_mut(), 0, &mut needed);
    }
    if needed == 0 {
        return Err(unsafe { GetLastError() });
    }
    let mut buffer = vec![0u16; (needed as usize).div_ceil(2)];
    if unsafe {
        GetUserObjectInformationW(
            desktop,
            UOI_NAME,
            buffer.as_mut_ptr().cast(),
            needed,
            &mut needed,
        )
    } == 0
    {
        return Err(unsafe { GetLastError() });
    }
    let length = buffer
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(buffer.len());
    Ok(String::from_utf16_lossy(&buffer[..length]))
}

#[cfg(test)]
#[path = "session_probe_tests.rs"]
mod tests;
