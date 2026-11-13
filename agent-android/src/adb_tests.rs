use super::{Adb, Report, adb_failed};
use std::cell::OnceCell;
use std::path::PathBuf;

fn adb_with_serial(serial: &str) -> Adb {
    Adb {
        exe: Some(PathBuf::from("adb")),
        serial: Some(serial.to_string()),
        resolved: OnceCell::new(),
        discovery_error: None,
    }
}

#[test]
fn puts_the_serial_before_device_commands() {
    let adb = adb_with_serial("emulator-5554");
    let args = adb
        .device_args(&["shell", "input", "tap", "10", "20"])
        .expect("device args");
    assert_eq!(
        args,
        vec!["-s", "emulator-5554", "shell", "input", "tap", "10", "20"]
    );
}

#[test]
fn reports_a_missing_explicit_binary() {
    let adb = Adb::discover(Some(PathBuf::from(r"Z:\definitely-missing\adb.exe")), None);
    let error = adb.discovery_error().expect("discovery error");
    assert_eq!(error.code, "ADB_NOT_FOUND");
    assert!(adb.exe_display().is_none());
}

#[test]
fn classifies_a_failed_adb_run_by_its_output() {
    let missing = adb_failed(&failed_output(
        "",
        "adb.exe: device 'bogus-serial' not found",
    ));
    assert_eq!(missing.code, "DEVICE_NOT_FOUND");
    let offline = adb_failed(&failed_output("", "error: device offline"));
    assert_eq!(offline.code, "DEVICE_NOT_READY");
    let other = adb_failed(&failed_output("", "error: closed"));
    assert_eq!(other.code, "ADB_FAILED");
    assert_eq!(other.message, "error: closed");
}

#[test]
fn a_report_keeps_both_streams_of_a_failed_command() {
    let output = failed_output(
        "  bash arg: -p\r\n** No activities found to run, monkey aborted.\r\n",
        "args: [-p, com.example.missing]\r\n",
    );
    let report = Report::from_output(&output).expect("a command failure is still a report");
    assert_eq!(
        report.text,
        "bash arg: -p\n** No activities found to run, monkey aborted.\nargs: [-p, com.example.missing]"
    );
    let error = report.check().expect_err("the exit status is kept");
    assert_eq!(error.code, "ADB_FAILED");
}

#[test]
fn a_report_of_a_successful_command_checks_out_as_its_text() {
    let output = std::process::Output {
        status: std::process::ExitStatus::default(),
        stdout: b"Success\r\n".to_vec(),
        stderr: Vec::new(),
    };
    let report = Report::from_output(&output).expect("report");
    assert_eq!(report.check().expect("success"), "Success");
}

#[test]
fn a_device_failure_is_an_error_not_a_report() {
    let output = failed_output("", "adb.exe: device 'bogus-serial' not found");
    let error = Report::from_output(&output).err().expect("device failure");
    assert_eq!(error.code, "DEVICE_NOT_FOUND");
}

fn failed_output(stdout: &str, stderr: &str) -> std::process::Output {
    let status = failing_status();
    assert!(!status.success());
    std::process::Output {
        status,
        stdout: stdout.as_bytes().to_vec(),
        stderr: stderr.as_bytes().to_vec(),
    }
}

#[cfg(windows)]
fn failing_status() -> std::process::ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(1)
}

#[cfg(not(windows))]
fn failing_status() -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(1 << 8)
}
