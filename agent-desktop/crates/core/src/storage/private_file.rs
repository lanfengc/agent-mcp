use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) fn open_private_lock(path: &Path, create: bool) -> std::io::Result<File> {
    crate::storage::private_file_ops::with_active_ops(|ops| ops.open_private_lock(path, create))
}

pub(crate) fn open_private_lock_portable(path: &Path, create: bool) -> std::io::Result<File> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid_input("private file path has no parent"))?;
    crate::storage::private_file_parent::ensure_private_portable(parent)?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(create);
    let file = options.open(path)?;
    validate_private_regular(&file)?;
    Ok(file)
}

pub(crate) fn open_private_append(path: &Path) -> std::io::Result<File> {
    crate::storage::private_file_ops::with_active_ops(|ops| ops.open_private_append(path))
}

/// Read access is requested alongside append because callers lock the returned
/// handle, and Windows `LockFileEx` fails with `ERROR_ACCESS_DENIED` on a handle
/// opened for append only.
pub(crate) fn open_private_append_portable(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).create(true).append(true);
    let file = options.open(path)?;
    validate_private_regular(&file)?;
    Ok(file)
}

pub(crate) fn read_private_bounded(path: &Path, max_bytes: u64) -> std::io::Result<Vec<u8>> {
    crate::storage::private_file_ops::with_active_ops(|ops| {
        ops.read_private_bounded(path, max_bytes)
    })
}

pub(crate) fn read_private_bounded_portable(
    path: &Path,
    max_bytes: u64,
) -> std::io::Result<Vec<u8>> {
    let file = open_private_read(path)?;
    crate::storage::private_file_ops::bounded_read(file, max_bytes)
}

pub(crate) fn read_regular_bounded(path: &Path, max_bytes: u64) -> std::io::Result<Vec<u8>> {
    let file = {
        let mut options = OpenOptions::new();
        options.read(true);
        options.open(path)?
    };
    validate_regular(&file)?;
    crate::storage::private_file_ops::bounded_read(file, max_bytes)
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    crate::storage::private_file_ops::with_active_ops(|ops| ops.write_atomic(path, bytes))
}

pub(crate) fn write_atomic_portable(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_atomic_with(
        path,
        bytes,
        crate::storage::private_file_parent::ensure_private_portable,
        sync_directory,
        validate_private_destination,
    )
}

/// Writes user-requested output, deliberately bypassing the `PrivateFileOps`
/// seam that `write_atomic` routes through.
///
/// Its callers — `commands::screenshot`, `commands::clipboard_get` and the
/// trace HTML export in `trace_read::html`, via `crate::refs::write_user_file` —
/// write to a destination the user named, so
/// it must tolerate network shares, reparse-traversing paths such as OneDrive
/// or mapped folders, and directories owned by another principal. A platform
/// implementation of the seam refuses all three, which is right for private
/// artifacts and a regression here. User output deliberately accepts the
/// group-readable destination and the system temporary directory that the
/// private path rejects. The cost is that user
/// output keeps the portable rename-based replace on Windows, which unlike
/// `ReplaceFileW` fails over a destination another process holds open — a
/// limitation of user output, not of private artifacts.
pub(crate) fn write_user_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    write_atomic_with(
        path,
        bytes,
        crate::storage::private_file_parent::ensure_user,
        sync_user_directory,
        validate_user_destination,
    )
}

/// `Path::parent` yields `Some("")` for a bare relative filename, and both
/// `std::fs::metadata("")` and opening `""` fail with `NotFound`, so the empty
/// parent is normalized to the current directory before `ensure_parent` and
/// `sync_parent` see it; without that, a legitimate relative output path such
/// as `--out report.html` is rejected.
fn write_atomic_with(
    path: &Path,
    bytes: &[u8],
    ensure_parent: fn(&Path) -> std::io::Result<()>,
    sync_parent: fn(&Path) -> std::io::Result<()>,
    validate_destination: fn(&Path) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid_input("private file path has no parent"))?;
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    ensure_parent(parent)?;
    validate_destination(path)?;
    let (temporary, mut file) = create_temporary(path)?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    let result = written.and_then(|()| {
        #[cfg(test)]
        crash_before_rename_if_requested(path);
        replace_atomic(&temporary, path)?;
        validate_private_regular(&open_private_read(path)?)?;
        sync_parent(parent)
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn validate_private_destination(path: &Path) -> std::io::Result<()> {
    match open_private_read(path) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn validate_user_destination(path: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(invalid_input("user output path is a symlink"))
        }
        Ok(metadata) if !metadata.is_file() => {
            Err(invalid_input("user output path is not a regular file"))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub(crate) fn validate_private_regular(file: &File) -> std::io::Result<std::fs::Metadata> {
    let metadata = validate_regular(file)?;
    Ok(metadata)
}

pub(crate) fn validate_regular(file: &File) -> std::io::Result<std::fs::Metadata> {
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(permission_denied("path is not a regular file"));
    }
    Ok(metadata)
}

fn open_private_read(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    let file = options.open(path)?;
    validate_private_regular(&file)?;
    Ok(file)
}

fn create_temporary(path: &Path) -> std::io::Result<(PathBuf, File)> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid_input("private file path has an invalid filename"))?;
    for _ in 0..32 {
        let temporary = path.with_file_name(crate::storage::private_file_ops::temporary_file_name(
            std::ffi::OsStr::new(file_name),
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        match options.open(&temporary) {
            Ok(file) => {
                validate_private_regular(&file)?;
                return Ok((temporary, file));
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not allocate a private temporary file",
    ))
}

fn sync_directory(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

fn sync_user_directory(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

fn replace_atomic(source: &Path, destination: &Path) -> std::io::Result<()> {
    std::fs::rename(source, destination)
}

#[cfg(test)]
fn crash_before_rename_if_requested(path: &Path) {
    if std::env::var_os("AGENT_DESKTOP_TEST_CRASH_BEFORE_RENAME")
        .is_some_and(|value| Path::new(&value) == path)
    {
        std::process::abort();
    }
}

pub(crate) fn invalid_input(message: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

pub(crate) fn permission_denied(message: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::PermissionDenied, message)
}

#[cfg(test)]
#[path = "private_file_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "private_file_lock_tests.rs"]
mod lock_tests;
