use super::*;
use agent_desktop_core::WindowOp;

/// Wire-shape assertion for a shared condition - one core contract every
/// adapter can reach for the same real-world event.
fn assert_class_a_envelope(error: &AdapterError, code: ErrorCode, expected: DeliverySemantics) {
    assert_code_wire(error, code);
    assert_disposition_wire(error, expected);
}

/// Targets a `HostedFixture` this test alone owns, never this process's own
/// pid: the graceful close arm broadcasts `WM_CLOSE` to every top-level
/// window the target pid owns, and the test runner's own pid also owns
/// other tests' in-process `LocalFixture` windows running in parallel, so
/// targeting it here would destroy them. A fixture host that swallows
/// `WM_CLOSE` never closes, so the deadline genuinely expires.
#[test]
fn shared_timeout_after_close_delivery_pins_the_windows_envelope() {
    #[cfg(target_os = "windows")]
    {
        crate::tree::fixture::bootstrap();
        let fixture = crate::tree::fixture::HostedFixture::spawn_swallowing_wm_close()
            .expect("swallowing fixture");
        let pid = ProcessId::from(fixture.process_id());
        let token = crate::system::process_identity::token_for_pid(pid)
            .expect("token")
            .expect("live");
        let app = AppInfo {
            name: "fixture-host".into(),
            pid,
            bundle_id: None,
            process_instance: Some(token),
            presentation: None,
        };
        let error = close_app_impl(&app, false, Deadline::after(200).expect("deadline"))
            .expect_err("timeout");
        assert_class_a_envelope(
            &error,
            ErrorCode::Timeout,
            DeliverySemantics::delivered_unverified(),
        );
        assert_eq!(error_wire(&error)["disposition"]["retry"], "unsafe");
    }
}

/// `budget_exhausted(false)` is the focus-steal-retry-budget exhaustion
/// path. The pair expresses a core rule: a chain that issued no successful
/// step reports `ActionFailed`/not-delivered.
#[test]
fn shared_action_failed_before_delivery_pins_the_windows_envelope() {
    let error = budget_exhausted(false);
    assert_class_a_envelope(
        &error,
        ErrorCode::ActionFailed,
        DeliverySemantics::not_delivered(),
    );
    assert_eq!(error_wire(&error)["disposition"]["retry"], "safe");
}

/// `AdapterError::permission_denied()` is the core constructor the Windows
/// write-path classifier
/// (`crates/windows/src/actions/mutation.rs::classify_hresult`, on
/// `E_ACCESSDENIED`) calls for "the OS denied this process accessibility
/// access." No lifecycle adapter function reaches
/// that condition on Windows today - close, launch, window-op, and focus
/// never probe whole-process UI Automation access - so this test drives the
/// real Windows call site that does, instead of a hand-written literal
/// standing in for it.
#[test]
fn shared_perm_denied_before_delivery_pins_the_windows_envelope() {
    use crate::actions::mutation::classify_mutation;
    use crate::system::runtime::hresult::E_ACCESSDENIED;
    use crate::tree::automation::UiaFailure;

    let error = classify_mutation(
        "SetValue",
        "ValuePattern.SetValue",
        &UiaFailure::Hresult(E_ACCESSDENIED),
    )
    .expect_err("access-denied write must classify as an error");
    assert_class_a_envelope(
        &error,
        ErrorCode::PermDenied,
        DeliverySemantics::not_delivered(),
    );
    assert_eq!(error_wire(&error)["disposition"]["retry"], "safe");
}

/// Windows raises `StaleRef` here because `window_op_impl` has no
/// process-instance token to verify before mutating the window, so the
/// write is refused before any native call. The code choice follows this
/// crate's resolved-reference contract. This test pins the Windows wire
/// shape.
#[test]
fn adapter_window_op_without_a_process_instance_keeps_the_stale_ref_envelope() {
    use agent_desktop_core::{InteractionLease, SystemOps};

    let win = WindowInfo {
        id: "w-1".into(),
        title: String::new(),
        app: "fixture".into(),
        pid: ProcessId::from(1u32),
        process_instance: None,
        bounds: None,
        state: WindowState::default(),
    };
    let deadline = Deadline::after(1_000).expect("deadline");
    let lease = InteractionLease::guarded(deadline, ()).expect("lease");

    let error = crate::adapter::WindowsAdapter::new()
        .window_op(&win, WindowOp::Minimize, &lease)
        .expect_err("stale");

    assert_class_a_envelope(
        &error,
        ErrorCode::StaleRef,
        DeliverySemantics::not_delivered(),
    );
}
