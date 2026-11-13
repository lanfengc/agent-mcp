//! Relaunch the pending command inside the logged-in session.
//!
//! A service launched outside the user's session still receives the MCP
//! request, but every desktop call it makes lands on a desktop nobody can
//! see. Rather than act there, the command is re-run as the logged-in user on
//! `winsta0\default`: the child inherits the caller's stdout/stderr pipes, so
//! the JSON response still reaches the MCP client, while stdin is attached to
//! `NUL` so the child cannot consume the server's own JSON-RPC stream. The
//! parent waits and forwards the child's exit code.

use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::{null, null_mut};

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_NOT_ALL_ASSIGNED, GetLastError, HANDLE, INVALID_HANDLE_VALUE, LUID,
};
use windows_sys::Win32::Security::{
    AdjustTokenPrivileges, DuplicateTokenEx, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW,
    SE_PRIVILEGE_ENABLED, SECURITY_ATTRIBUTES, SecurityImpersonation, TOKEN_ADJUST_PRIVILEGES,
    TOKEN_ALL_ACCESS, TOKEN_PRIVILEGES, TOKEN_QUERY, TokenPrimary,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Console::{GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};
use windows_sys::Win32::System::Environment::{
    CreateEnvironmentBlock, DestroyEnvironmentBlock, GetCommandLineW,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::System::RemoteDesktop::WTSQueryUserToken;
use windows_sys::Win32::System::Threading::{
    CREATE_UNICODE_ENVIRONMENT, CreateProcessAsUserW, GetCurrentProcess, GetExitCodeProcess,
    INFINITE, OpenProcessToken, PROCESS_INFORMATION, STARTF_USESTDHANDLES, STARTUPINFOW,
    WaitForSingleObject,
};

const DESKTOP: &str = "winsta0\\default";
const TCB_PRIVILEGE: &str = "SeTcbPrivilege";

/// Runs the current command line in the active session and returns its exit
/// code.
pub fn relaunch_in_active_session() -> Result<u32, AdapterError> {
    let session =
        crate::system::session_probe::active_session_id().ok_or_else(no_interactive_user)?;
    enable_tcb_privilege()?;
    let user_token = OwnedHandle::new(
        query_user_token(session)?,
        "The logged-in user's token could not be queried",
    );
    let primary = OwnedHandle::new(
        primary_token(user_token.0)?,
        "The user token could not be duplicated as a primary token",
    );
    let environment = EnvironmentBlock::create(primary.0)?;
    let stdio = StdioHandles::capture()?;
    let application = current_executable()?;
    let mut command = current_command_line()?;
    let mut desktop: Vec<u16> = DESKTOP.encode_utf16().chain(std::iter::once(0)).collect();
    let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
    startup.cb = size_of::<STARTUPINFOW>() as u32;
    startup.lpDesktop = desktop.as_mut_ptr();
    startup.dwFlags = STARTF_USESTDHANDLES;
    startup.hStdInput = stdio.stdin.0;
    startup.hStdOutput = stdio.stdout.0;
    startup.hStdError = stdio.stderr.0;
    let mut process: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let created = unsafe {
        CreateProcessAsUserW(
            primary.0,
            application.as_ptr(),
            command.as_mut_ptr(),
            null(),
            null(),
            1,
            CREATE_UNICODE_ENVIRONMENT,
            environment.0,
            null(),
            &startup,
            &mut process,
        )
    };
    if created == 0 {
        return Err(last_error(
            "The command could not be relaunched in the logged-in session",
            "Run the agent as LocalSystem so it holds SeTcbPrivilege, SeAssignPrimaryTokenPrivilege and SeIncreaseQuotaPrivilege",
        ));
    }
    let process_handle = OwnedHandle(process.hProcess);
    let _thread_handle = OwnedHandle(process.hThread);
    drop(stdio);
    drop(environment);
    unsafe { WaitForSingleObject(process_handle.0, INFINITE) };
    let mut exit_code = 1u32;
    if unsafe { GetExitCodeProcess(process_handle.0, &mut exit_code) } == 0 {
        return Err(last_error(
            "The relaunched command's exit code could not be read",
            "Retry the command, or run the agent inside the logged-in session",
        ));
    }
    Ok(exit_code)
}

fn enable_tcb_privilege() -> Result<(), AdapterError> {
    let mut token = null_mut();
    if unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
    } == 0
    {
        return Err(last_error(
            "The process token could not be opened to enable SeTcbPrivilege",
            "Run the agent as LocalSystem, or launch it inside the logged-in session",
        ));
    }
    let token = OwnedHandle(token);
    let name: Vec<u16> = TCB_PRIVILEGE
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut luid = LUID {
        LowPart: 0,
        HighPart: 0,
    };
    if unsafe { LookupPrivilegeValueW(null(), name.as_ptr(), &mut luid) } == 0 {
        return Err(last_error(
            "SeTcbPrivilege could not be resolved",
            "This Windows installation does not expose SeTcbPrivilege; launch the agent inside the logged-in session",
        ));
    }
    let privileges = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: SE_PRIVILEGE_ENABLED,
        }],
    };
    if unsafe { AdjustTokenPrivileges(token.0, 0, &privileges, 0, null_mut(), null_mut()) } == 0 {
        return Err(last_error(
            "SeTcbPrivilege could not be enabled",
            "Run the agent as LocalSystem, or launch it inside the logged-in session",
        ));
    }
    if unsafe { GetLastError() } == ERROR_NOT_ALL_ASSIGNED {
        return Err(AdapterError::new(
            ErrorCode::PermDenied,
            "SeTcbPrivilege is not held, so the command cannot be relaunched into the logged-in desktop",
        )
        .with_suggestion(
            "Run the agent as LocalSystem (a service or scheduled task under SYSTEM), or launch it inside the logged-in session",
        )
        .with_disposition(DeliverySemantics::not_delivered()));
    }
    Ok(())
}

fn query_user_token(session: u32) -> Result<HANDLE, AdapterError> {
    let mut token = null_mut();
    if unsafe { WTSQueryUserToken(session, &mut token) } == 0 {
        return Err(last_error(
            "The logged-in user's token could not be queried",
            "Run the agent as LocalSystem, or launch it inside the logged-in session",
        ));
    }
    Ok(token)
}

fn primary_token(token: HANDLE) -> Result<HANDLE, AdapterError> {
    let mut primary = null_mut();
    if unsafe {
        DuplicateTokenEx(
            token,
            TOKEN_ALL_ACCESS,
            null(),
            SecurityImpersonation,
            TokenPrimary,
            &mut primary,
        )
    } == 0
    {
        return Err(last_error(
            "The user token could not be duplicated as a primary token",
            "Run the agent as LocalSystem, or launch it inside the logged-in session",
        ));
    }
    Ok(primary)
}

fn current_executable() -> Result<Vec<u16>, AdapterError> {
    let mut buffer = vec![0u16; 260];
    loop {
        let length =
            unsafe { GetModuleFileNameW(null_mut(), buffer.as_mut_ptr(), buffer.len() as u32) };
        if length == 0 {
            return Err(last_error(
                "The current executable path could not be read",
                "Run the command again from the installed agent-desktop binary",
            ));
        }
        if (length as usize) < buffer.len() {
            buffer.truncate(length as usize + 1);
            return Ok(buffer);
        }
        if buffer.len() >= 32_768 {
            return Err(AdapterError::internal(
                "The current executable path exceeds the Windows path limit",
            ));
        }
        buffer.resize(buffer.len() * 2, 0);
    }
}

fn current_command_line() -> Result<Vec<u16>, AdapterError> {
    let raw = unsafe { GetCommandLineW() };
    if raw.is_null() {
        return Err(AdapterError::internal(
            "The current command line could not be read for the relaunch",
        ));
    }
    let mut length = 0usize;
    unsafe {
        while *raw.add(length) != 0 {
            length += 1;
        }
    }
    if length == 0 {
        return Err(AdapterError::internal(
            "The current command line is empty, so there is nothing to relaunch",
        ));
    }
    let mut command = unsafe { std::slice::from_raw_parts(raw, length) }.to_vec();
    command.push(0);
    Ok(command)
}

fn no_interactive_user() -> AdapterError {
    AdapterError::new(
        ErrorCode::PermDenied,
        "No session has a user at its desktop, so there is no desktop to relaunch into",
    )
    .with_suggestion(
        "Log in at the console or reconnect the remote desktop session, then retry the command",
    )
    .with_disposition(DeliverySemantics::not_delivered())
}

fn last_error(message: &str, suggestion: &str) -> AdapterError {
    let code = unsafe { GetLastError() };
    AdapterError::new(ErrorCode::ActionFailed, message.to_owned())
        .with_suggestion(suggestion)
        .with_details(serde_json::json!({ "win32_error": code }))
        .with_disposition(DeliverySemantics::not_delivered())
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn new(handle: HANDLE, message: &str) -> Self {
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            tracing::warn!(reason = message, "the relaunch received an invalid handle");
        }
        Self(handle)
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            unsafe { CloseHandle(self.0) };
        }
    }
}

struct EnvironmentBlock(*mut c_void);

impl EnvironmentBlock {
    fn create(token: HANDLE) -> Result<Self, AdapterError> {
        let mut block = null_mut();
        if unsafe { CreateEnvironmentBlock(&mut block, token, 0) } == 0 {
            return Err(last_error(
                "The logged-in user's environment block could not be built",
                "Run the agent as LocalSystem, or launch it inside the logged-in session",
            ));
        }
        Ok(Self(block))
    }
}

impl Drop for EnvironmentBlock {
    fn drop(&mut self) {
        unsafe { DestroyEnvironmentBlock(self.0) };
    }
}

struct StdioHandles {
    stdin: OwnedHandle,
    stdout: OwnedHandle,
    stderr: OwnedHandle,
}

impl StdioHandles {
    fn capture() -> Result<Self, AdapterError> {
        Ok(Self {
            stdin: OwnedHandle::new(open_nul()?, "The NUL device handle was invalid"),
            stdout: OwnedHandle::new(
                inheritable_duplicate(STD_OUTPUT_HANDLE)?,
                "The stdout duplicate was invalid",
            ),
            stderr: OwnedHandle::new(
                inheritable_duplicate(STD_ERROR_HANDLE)?,
                "The stderr duplicate was invalid",
            ),
        })
    }
}

fn open_nul() -> Result<HANDLE, AdapterError> {
    let name: Vec<u16> = "NUL".encode_utf16().chain(std::iter::once(0)).collect();
    let attributes = inheritable_attributes();
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            windows_sys::Win32::Foundation::GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &attributes,
            OPEN_EXISTING,
            0,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(last_error(
            "The NUL device could not be opened for the relaunched command's stdin",
            "Retry the command, or run the agent inside the logged-in session",
        ));
    }
    Ok(handle)
}

fn inheritable_duplicate(
    std_handle: windows_sys::Win32::System::Console::STD_HANDLE,
) -> Result<HANDLE, AdapterError> {
    let source = unsafe { GetStdHandle(std_handle) };
    if source.is_null() || source == INVALID_HANDLE_VALUE {
        return Err(AdapterError::internal(
            "The command's stdout or stderr handle is not available for the relaunch",
        ));
    }
    let mut duplicate = null_mut();
    if unsafe {
        windows_sys::Win32::Foundation::DuplicateHandle(
            GetCurrentProcess(),
            source,
            GetCurrentProcess(),
            &mut duplicate,
            0,
            1,
            windows_sys::Win32::Foundation::DUPLICATE_SAME_ACCESS,
        )
    } == 0
    {
        return Err(last_error(
            "The command's output handle could not be duplicated for the relaunch",
            "Retry the command, or run the agent inside the logged-in session",
        ));
    }
    Ok(duplicate)
}

fn inheritable_attributes() -> SECURITY_ATTRIBUTES {
    SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    }
}

#[cfg(test)]
#[path = "session_relaunch_tests.rs"]
mod tests;
