use std::ffi::OsStr;
use std::path::PathBuf;

pub fn find_adb() -> Option<PathBuf> {
    find_beside_executable()
        .or_else(find_on_path)
        .or_else(find_known_path)
}

fn adb_name() -> &'static str {
    if cfg!(windows) { "adb.exe" } else { "adb" }
}

fn find_beside_executable() -> Option<PathBuf> {
    let candidate = std::env::current_exe().ok()?.parent()?.join(adb_name());
    candidate.is_file().then_some(candidate)
}

fn find_on_path() -> Option<PathBuf> {
    find_in_path(&std::env::var_os("PATH")?)
}

fn find_in_path(path: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(adb_name()))
        .find(|candidate| candidate.is_file())
}

fn find_known_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    #[cfg(windows)]
    {
        for key in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
            if let Some(root) = std::env::var_os(key) {
                candidates.push(PathBuf::from(root).join("platform-tools").join(adb_name()));
            }
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(
                PathBuf::from(local)
                    .join("Android")
                    .join("Sdk")
                    .join("platform-tools")
                    .join(adb_name()),
            );
        }
        if let Some(home) = std::env::var_os("USERPROFILE") {
            candidates.push(
                PathBuf::from(home)
                    .join("AppData")
                    .join("Local")
                    .join("Android")
                    .join("Sdk")
                    .join("platform-tools")
                    .join(adb_name()),
            );
        }
    }
    #[cfg(not(windows))]
    {
        candidates.push(PathBuf::from("/usr/local/bin/adb"));
        candidates.push(PathBuf::from("/usr/bin/adb"));
    }
    for candidate in candidates {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    #[cfg(windows)]
    if let Some(path) = find_winget_path() {
        return Some(path);
    }
    None
}

#[cfg(windows)]
fn find_winget_path() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    let packages = PathBuf::from(local)
        .join("Microsoft")
        .join("WinGet")
        .join("Packages");
    for entry in std::fs::read_dir(packages).ok()?.flatten() {
        let candidate = entry.path().join("platform-tools").join("adb.exe");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
#[path = "adb_discovery_tests.rs"]
mod tests;
