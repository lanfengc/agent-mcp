use crate::adb_discovery;
use crate::device::{Device, parse_devices};
use crate::error::{AppError, Result};
use crate::shell;
use std::cell::OnceCell;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
pub const CAPTURE_TIMEOUT: Duration = Duration::from_secs(45);
pub const INSTALL_TIMEOUT: Duration = Duration::from_secs(180);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct Adb {
    exe: Option<PathBuf>,
    serial: Option<String>,
    resolved: OnceCell<String>,
    discovery_error: Option<AppError>,
}

impl Adb {
    pub fn discover(explicit: Option<PathBuf>, serial: Option<String>) -> Self {
        let mut discovery_error = None;
        let exe = match explicit {
            Some(path) if path.is_file() => Some(path),
            Some(path) => {
                discovery_error = Some(
                    AppError::new(
                        "ADB_NOT_FOUND",
                        format!("ADB executable not found: {}", path.display()),
                    )
                    .with_suggestion(
                        "Correct the path, install platform-tools, or unset AGENT_ANDROID_ADB",
                    ),
                );
                None
            }
            None => adb_discovery::find_adb(),
        };
        if exe.is_none() && discovery_error.is_none() {
            discovery_error = Some(AppError::adb_not_found());
        }
        Self {
            exe,
            serial,
            resolved: OnceCell::new(),
            discovery_error,
        }
    }

    pub fn exe_display(&self) -> Option<String> {
        self.exe.as_ref().map(|path| path.display().to_string())
    }

    pub fn discovery_error(&self) -> Option<&AppError> {
        self.discovery_error.as_ref()
    }

    pub fn ensure_ready(&self) -> Result<()> {
        self.exe().map(|_| ())
    }

    pub fn devices(&self) -> Result<Vec<Device>> {
        let output =
            self.run_checked(&["devices".to_string(), "-l".to_string()], DEFAULT_TIMEOUT)?;
        Ok(parse_devices(&output))
    }

    pub fn version(&self) -> Result<String> {
        let output = self.run_checked(&["version".to_string()], DEFAULT_TIMEOUT)?;
        output
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .ok_or_else(|| AppError::new("ADB_FAILED", "adb version returned no output"))
    }

    pub fn serial(&self) -> Result<&str> {
        if let Some(serial) = &self.serial {
            return Ok(serial.as_str());
        }
        if let Some(serial) = self.resolved.get() {
            return Ok(serial.as_str());
        }
        let selected = self.select_serial()?;
        let _ = self.resolved.set(selected);
        self.resolved
            .get()
            .map(String::as_str)
            .ok_or_else(AppError::device_not_found)
    }

    fn select_serial(&self) -> Result<String> {
        let devices = self.devices()?;
        let ready: Vec<&Device> = devices.iter().filter(|device| device.ready()).collect();
        match ready.len() {
            1 => Ok(ready[0].serial.clone()),
            0 => {
                if let Some(device) = devices.first() {
                    Err(AppError::new(
                        "DEVICE_NOT_READY",
                        format!(
                            "Android device {} is present but its state is '{}'",
                            device.serial, device.state
                        ),
                    )
                    .with_suggestion(
                        "Accept the USB debugging prompt on the device, or run `adb reconnect`",
                    ))
                } else {
                    Err(AppError::device_not_found())
                }
            }
            _ => Err(AppError::multiple_devices(
                &ready
                    .iter()
                    .map(|device| device.serial.clone())
                    .collect::<Vec<_>>(),
            )),
        }
    }

    pub fn device_args(&self, args: &[&str]) -> Result<Vec<String>> {
        let serial = self.serial()?;
        let mut full = Vec::with_capacity(args.len() + 2);
        full.push("-s".to_string());
        full.push(serial.to_string());
        full.extend(args.iter().map(|arg| (*arg).to_string()));
        Ok(full)
    }

    pub fn run_device(&self, args: &[&str], timeout: Duration) -> Result<String> {
        let full = self.device_args(args)?;
        self.run_checked(&full, timeout)
    }

    pub fn shell(&self, args: &[&str]) -> Result<String> {
        let full = self.shell_args(args)?;
        self.run_checked(&full, DEFAULT_TIMEOUT)
    }

    pub fn shell_report(&self, args: &[&str]) -> Result<Report> {
        let full = self.shell_args(args)?;
        self.report(&full, DEFAULT_TIMEOUT)
    }

    pub fn device_report(&self, args: &[&str], timeout: Duration) -> Result<Report> {
        let full = self.device_args(args)?;
        self.report(&full, timeout)
    }

    pub fn host_report(&self, args: &[&str], timeout: Duration) -> Result<Report> {
        let full = args
            .iter()
            .map(|arg| (*arg).to_string())
            .collect::<Vec<_>>();
        self.report(&full, timeout)
    }

    pub fn exec_out(&self, args: &[&str]) -> Result<Vec<u8>> {
        let output = self.exec_out_output(args, CAPTURE_TIMEOUT)?;
        if !output.status.success() {
            return Err(adb_failed(&output));
        }
        Ok(output.stdout)
    }

    pub fn exec_out_output(&self, args: &[&str], timeout: Duration) -> Result<Output> {
        let mut full = Vec::with_capacity(args.len() + 1);
        full.push("exec-out");
        full.extend_from_slice(args);
        let full = self.device_args(&full)?;
        self.run_args(&full, timeout)
    }

    pub fn run_args(&self, args: &[String], timeout: Duration) -> Result<Output> {
        let exe = self.exe()?;
        run_process(exe, args, timeout)
    }

    fn shell_args(&self, args: &[&str]) -> Result<Vec<String>> {
        let mut full = self.device_args(&["shell"])?;
        full.extend(args.iter().map(|arg| shell::quote(arg)));
        Ok(full)
    }

    fn report(&self, args: &[String], timeout: Duration) -> Result<Report> {
        Report::from_output(&self.run_args(args, timeout)?)
    }

    fn run_checked(&self, args: &[String], timeout: Duration) -> Result<String> {
        let output = self.run_args(args, timeout)?;
        if !output.status.success() {
            return Err(adb_failed(&output));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn exe(&self) -> Result<&Path> {
        if let Some(exe) = &self.exe {
            return Ok(exe.as_path());
        }
        if let Some(error) = &self.discovery_error {
            return Err(error.clone());
        }
        Err(AppError::adb_not_found())
    }
}

pub struct Report {
    pub text: String,
    failure: Option<AppError>,
}

impl Report {
    fn from_output(output: &Output) -> Result<Self> {
        let failure = (!output.status.success()).then(|| adb_failed(output));
        if let Some(error) = &failure
            && error.code != "ADB_FAILED"
        {
            return Err(error.clone());
        }
        Ok(Self {
            text: combined_text(output),
            failure,
        })
    }

    pub fn check(self) -> Result<String> {
        match self.failure {
            Some(error) => Err(error),
            None => Ok(self.text),
        }
    }
}

fn run_process(exe: &Path, args: &[String], timeout: Duration) -> Result<Output> {
    let mut command = Command::new(exe);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_window(&mut command);
    let mut child = command.spawn().map_err(|error| {
        AppError::new("ADB_NOT_FOUND", format!("Could not start adb: {error}"))
            .with_suggestion("Install Android platform-tools or set AGENT_ANDROID_ADB")
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::new("ADB_FAILED", "adb stdout was unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::new("ADB_FAILED", "adb stderr was unavailable"))?;
    let stdout_thread = thread::spawn(move || read_all(stdout));
    let stderr_thread = thread::spawn(move || read_all(stderr));

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = stdout_thread.join().unwrap_or_default();
                let stderr = stderr_thread.join().unwrap_or_default();
                return Ok(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AppError::new(
                    "ADB_FAILED",
                    format!("Could not wait for adb: {error}"),
                ));
            }
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            return Err(AppError::timeout(timeout.as_secs().max(1), "adb"));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn read_all(mut reader: impl Read) -> Vec<u8> {
    let mut buffer = Vec::new();
    let _ = reader.read_to_end(&mut buffer);
    buffer
}

fn adb_failed(output: &Output) -> AppError {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let message = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("adb exited with status {}", output.status)
    };
    AppError::device_failure(&message).unwrap_or_else(|| AppError::new("ADB_FAILED", message))
}

fn combined_text(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    [stdout.trim(), stderr.trim()]
        .into_iter()
        .filter(|stream| !stream.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .replace("\r\n", "\n")
}

fn hide_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = command;
}

#[cfg(test)]
#[path = "adb_tests.rs"]
mod tests;
