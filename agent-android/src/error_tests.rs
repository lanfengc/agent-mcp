use super::AppError;

#[test]
fn classifies_a_missing_device_from_adb_output() {
    for output in [
        "adb.exe: no devices/emulators found",
        "error: no devices/emulators found",
        "adb: device 'emulator-5554' not found",
        "error: device 'bogus-serial' not found",
    ] {
        let error = AppError::device_failure(output).expect("device failure");
        assert_eq!(error.code, "DEVICE_NOT_FOUND", "{output}");
        assert!(error.suggestion.is_some(), "{output}");
    }
    let error = AppError::device_failure("adb.exe: device 'bogus-serial' not found")
        .expect("device failure");
    assert!(error.message.contains("bogus-serial"));
}

#[test]
fn classifies_a_device_that_is_present_but_not_usable() {
    for output in [
        "adb.exe: device offline",
        "error: device still authorizing",
        "adb.exe: device unauthorized.\nThis adb server's $ADB_VENDOR_KEYS is not set",
    ] {
        let error = AppError::device_failure(output).expect("device failure");
        assert_eq!(error.code, "DEVICE_NOT_READY", "{output}");
    }
}

#[test]
fn leaves_other_adb_output_unclassified() {
    for output in [
        "",
        "error: closed",
        "ERROR: null root node returned by UiTestAutomationBridge.",
        "Failure [INSTALL_FAILED_ALREADY_EXISTS]",
        "note: device 'x' not found earlier",
    ] {
        assert!(AppError::device_failure(output).is_none(), "{output}");
    }
}

#[test]
fn names_the_snapshot_a_stale_ref_was_looked_up_in() {
    let error = AppError::stale_ref("@a7", "s19ab");
    assert_eq!(error.code, "STALE_REF");
    assert!(error.message.contains("@a7"));
    assert!(error.message.contains("s19ab"));
}
