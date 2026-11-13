use super::parse_devices;

const SAMPLE: &str = "\
List of devices attached
emulator-5554          device product:sdk_gphone64_x86_64 model:sdk_gphone64_x86_64 device:emu64xa transport_id:1
R58M12345              unauthorized usb:1-1 transport_id:2
192.168.1.20:5555      offline transport_id:3
";

#[test]
fn parses_ready_unauthorized_and_offline_devices() {
    let devices = parse_devices(SAMPLE);
    assert_eq!(devices.len(), 3);
    assert_eq!(devices[0].serial, "emulator-5554");
    assert!(devices[0].ready());
    assert_eq!(devices[0].model.as_deref(), Some("sdk_gphone64_x86_64"));
    assert_eq!(devices[1].state, "unauthorized");
    assert!(!devices[1].ready());
    assert_eq!(devices[2].state, "offline");
    assert_eq!(devices[2].transport_id.as_deref(), Some("3"));
}

#[test]
fn ignores_daemon_noise_and_blank_lines() {
    let output = "* daemon started successfully\nList of devices attached\n\n";
    assert!(parse_devices(output).is_empty());
}

#[test]
fn records_no_permissions_state() {
    let output = "List of devices attached\nabc\tno permissions (missing udev rules?) usb:1-1\n";
    let devices = parse_devices(output);
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].state, "no permissions");
}
