use super::*;

#[test]
fn the_current_command_line_is_nul_terminated_and_non_empty() {
    let command = current_command_line().unwrap();

    assert!(command.len() > 1);
    assert_eq!(command.last(), Some(&0));
}

#[test]
fn the_current_executable_path_is_absolute_and_nul_terminated() {
    let executable = current_executable().unwrap();
    let text = String::from_utf16(&executable[..executable.len() - 1]).unwrap();

    assert_eq!(executable.last(), Some(&0));
    assert!(text.contains('\\'), "{text}");
}

#[test]
fn a_relaunch_without_tcb_privilege_is_refused_cleanly() {
    if enable_tcb_privilege().is_ok() {
        return;
    }
    let error = relaunch_in_active_session().unwrap_err();

    assert_eq!(error.disposition, DeliverySemantics::NotDelivered);
    assert!(
        matches!(error.code, ErrorCode::PermDenied | ErrorCode::ActionFailed),
        "{error:?}"
    );
}
