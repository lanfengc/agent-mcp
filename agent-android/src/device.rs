use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Device {
    pub serial: String,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_id: Option<String>,
}

impl Device {
    pub fn ready(&self) -> bool {
        self.state == "device"
    }
}

pub fn parse_devices(output: &str) -> Vec<Device> {
    let mut devices = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with("List of devices")
            || line.starts_with("* daemon")
            || line.starts_with("adb server")
        {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(serial) = parts.next() else {
            continue;
        };
        let Some(state) = parts.next() else {
            continue;
        };
        let mut device = Device {
            serial: serial.to_string(),
            state: state.to_string(),
            model: None,
            product: None,
            device: None,
            transport_id: None,
        };
        if line.contains("no permissions") {
            device.state = "no permissions".to_string();
        }
        for part in parts {
            let Some((key, value)) = part.split_once(':') else {
                continue;
            };
            let value = value.to_string();
            match key {
                "model" => device.model = Some(value),
                "product" => device.product = Some(value),
                "device" => device.device = Some(value),
                "transport_id" => device.transport_id = Some(value),
                _ => {}
            }
        }
        devices.push(device);
    }
    devices
}

#[cfg(test)]
#[path = "device_tests.rs"]
mod tests;
