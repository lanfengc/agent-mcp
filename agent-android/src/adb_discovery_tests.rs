use super::{adb_name, find_in_path};
use std::path::PathBuf;

#[test]
fn finds_adb_in_the_first_path_directory_that_holds_it() {
    let empty = tempfile::tempdir().expect("empty dir");
    let first = tempfile::tempdir().expect("first dir");
    let second = tempfile::tempdir().expect("second dir");
    for directory in [&first, &second] {
        std::fs::write(directory.path().join(adb_name()), b"").expect("fake adb");
    }
    let path = std::env::join_paths([empty.path(), first.path(), second.path()]).expect("path");
    assert_eq!(
        find_in_path(&path),
        Some(first.path().join(adb_name())),
        "the result is the full path of the first match"
    );
}

#[test]
fn ignores_relative_and_adb_less_path_entries() {
    let empty = tempfile::tempdir().expect("empty dir");
    let holder = tempfile::tempdir().expect("holder dir");
    std::fs::create_dir(holder.path().join(adb_name())).expect("directory named like adb");
    let path = std::env::join_paths([
        PathBuf::from("."),
        PathBuf::from("relative-tools"),
        empty.path().to_path_buf(),
        holder.path().to_path_buf(),
    ])
    .expect("path");
    assert_eq!(find_in_path(&path), None);
    assert_eq!(find_in_path(std::ffi::OsStr::new("")), None);
}
