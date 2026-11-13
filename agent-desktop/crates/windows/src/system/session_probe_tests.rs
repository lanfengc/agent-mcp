use super::*;

#[test]
fn a_process_outside_the_logged_in_session_is_offered_the_relaunch() {
    assert_eq!(
        session_verdict(Some(0), &[3]),
        Some(DesktopSession::OtherSession {
            current: 0,
            active: 3
        })
    );
}

#[test]
fn a_process_outside_several_active_sessions_is_sent_to_the_first() {
    assert_eq!(
        session_verdict(Some(0), &[2, 5]),
        Some(DesktopSession::OtherSession {
            current: 0,
            active: 2
        })
    );
}

#[test]
fn a_process_in_any_active_session_acts_on_its_own_desktop() {
    assert_eq!(session_verdict(Some(5), &[2, 5]), None);
}

#[test]
fn a_process_in_a_disconnected_session_is_offered_the_relaunch() {
    assert_eq!(
        session_verdict(Some(4), &[2]),
        Some(DesktopSession::OtherSession {
            current: 4,
            active: 2
        })
    );
}

#[test]
fn an_unreadable_session_id_falls_through_to_the_desktop_probe() {
    assert_eq!(session_verdict(None, &[2]), None);
}

#[test]
fn session_zero_without_an_active_user_has_no_interactive_desktop() {
    assert_eq!(
        session_verdict(Some(0), &[]),
        Some(DesktopSession::NoActiveSession)
    );
}

#[test]
fn no_active_session_at_all_has_no_interactive_desktop() {
    assert_eq!(
        session_verdict(Some(3), &[]),
        Some(DesktopSession::NoActiveSession)
    );
    assert_eq!(
        session_verdict(None, &[]),
        Some(DesktopSession::NoActiveSession)
    );
}

#[test]
fn a_matching_session_falls_through_to_the_desktop_probe() {
    assert_eq!(session_verdict(Some(3), &[3]), None);
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
