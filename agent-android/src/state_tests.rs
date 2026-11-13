use crate::bounds::{Bounds, Point};
use crate::commands::snapshot::{ScreenSize, SnapshotOutput, UiElement};
use crate::state::{MAX_SNAPSHOTS, SnapshotStore};

fn sample_snapshot(id: &str) -> SnapshotOutput {
    SnapshotOutput {
        snapshot_id: id.to_string(),
        package: "com.example".to_string(),
        screen: ScreenSize {
            width: 1080,
            height: 2400,
        },
        node_count: 1,
        elements: vec![UiElement {
            reference: "@a1".to_string(),
            role: "Button".to_string(),
            class: "android.widget.Button".to_string(),
            text: Some("OK".to_string()),
            desc: None,
            resource_id: Some("com.example:id/ok".to_string()),
            hint: None,
            package: Some("com.example".to_string()),
            bounds: Bounds {
                left: 10,
                top: 20,
                right: 110,
                bottom: 80,
            },
            center: Point { x: 60, y: 50 },
            flags: Default::default(),
        }],
    }
}

#[test]
fn saves_and_loads_a_snapshot_by_id() {
    let temp = tempfile::tempdir().expect("temp dir");
    let store = SnapshotStore::new(temp.path());
    store.save(&sample_snapshot("s1")).expect("save one");
    store.save(&sample_snapshot("s2")).expect("save two");
    let loaded = store.load("s1").expect("load named");
    assert_eq!(loaded.snapshot_id, "s1");
    assert_eq!(loaded.elements[0].reference, "@a1");
    assert_eq!(store.load("s2").expect("load second").snapshot_id, "s2");
}

#[test]
fn writes_nothing_but_snapshot_files() {
    let temp = tempfile::tempdir().expect("temp dir");
    let store = SnapshotStore::new(temp.path());
    store.save(&sample_snapshot("s1")).expect("save");
    let entries = std::fs::read_dir(temp.path())
        .expect("home")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(entries, vec!["snapshots".to_string()]);
}

#[test]
fn missing_snapshot_is_a_typed_error() {
    let temp = tempfile::tempdir().expect("temp dir");
    let store = SnapshotStore::new(temp.path());
    let error = store.load("s404").expect_err("missing snapshot");
    assert_eq!(error.code, "SNAPSHOT_NOT_FOUND");
    assert!(error.message.contains("s404"));
}

#[test]
fn rejects_snapshot_paths_outside_the_store() {
    let temp = tempfile::tempdir().expect("temp dir");
    let store = SnapshotStore::new(temp.path());
    for id in ["../escape", "", "a/b", r"a\b", "s1.json"] {
        let error = store.load(id).expect_err("invalid id");
        assert_eq!(error.code, "INVALID_ARGS", "{id:?}");
    }
}

#[test]
fn keeps_only_the_newest_snapshots() {
    let temp = tempfile::tempdir().expect("temp dir");
    let store = SnapshotStore::new(temp.path());
    let directory = temp.path().join("snapshots");
    std::fs::create_dir_all(&directory).expect("snapshot dir");
    let unrelated = directory.join("notes.txt");
    std::fs::write(&unrelated, b"keep").expect("unrelated file");
    let total = MAX_SNAPSHOTS + 5;
    for index in 0..total {
        let id = format!("s{index:04}");
        store.save(&sample_snapshot(&id)).expect("save");
        let stamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000 + index as u64);
        std::fs::File::options()
            .write(true)
            .open(directory.join(format!("{id}.json")))
            .and_then(|file| file.set_modified(stamp))
            .expect("set modified time");
    }
    store
        .save(&sample_snapshot("s9999"))
        .expect("save the newest");

    let stored = std::fs::read_dir(&directory)
        .expect("snapshot dir")
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .count();
    assert_eq!(stored, MAX_SNAPSHOTS);
    assert!(store.load("s9999").is_ok(), "the snapshot just saved stays");
    assert!(store.load(&format!("s{:04}", total - 1)).is_ok());
    assert_eq!(
        store.load("s0000").expect_err("pruned").code,
        "SNAPSHOT_NOT_FOUND"
    );
    assert!(unrelated.is_file(), "only snapshot files are pruned");
}
