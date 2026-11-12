use super::*;

#[test]
fn a_process_outside_the_logged_in_session_is_offered_the_relaunch() {
    assert_eq!(
        session_verdict(Some(0), Some(3)),
        Some(DesktopSession::OtherSession {
            current: 0,
            active: 3
        })
    );
}

#[test]
fn session_zero_without_an_active_user_has_no_interactive_desktop() {
    assert_eq!(
        session_verdict(Some(0), None),
        Some(DesktopSession::NoActiveSession)
    );
}

#[test]
fn a_matching_session_falls_through_to_the_desktop_probe() {
    assert_eq!(session_verdict(Some(3), Some(3)), None);
}

#[test]
fn a_default_desktop_is_interactive() {
    assert_eq!(
        desktop_verdict(Ok("Default".to_string())),
        DesktopSession::Interactive
    );
}

#[test]
fn a_winlogon_desktop_is_locked() {
    assert_eq!(
        desktop_verdict(Ok("Winlogon".to_string())),
        DesktopSession::Locked("Winlogon".to_string())
    );
}

#[test]
fn an_access_denied_probe_is_locked() {
    assert_eq!(
        desktop_verdict(Err(ERROR_ACCESS_DENIED)),
        DesktopSession::Locked("secure desktop".to_string())
    );
}

#[test]
fn any_other_probe_failure_is_reported_with_its_code() {
    assert_eq!(desktop_verdict(Err(87)), DesktopSession::Unreadable(87));
}
