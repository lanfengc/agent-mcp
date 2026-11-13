use crate::commands::snapshot::SnapshotOutput;
use crate::error::{AppError, Result};
use std::path::{Path, PathBuf};

pub const MAX_SNAPSHOTS: usize = 50;

pub struct SnapshotStore {
    root: PathBuf,
}

impl SnapshotStore {
    pub fn new(home: &Path) -> Self {
        Self {
            root: home.join("snapshots"),
        }
    }

    pub fn save(&self, snapshot: &SnapshotOutput) -> Result<()> {
        std::fs::create_dir_all(&self.root).map_err(|error| {
            AppError::new(
                "STATE_WRITE_FAILED",
                format!("Could not create snapshot directory: {error}"),
            )
        })?;
        let path = self.root.join(format!("{}.json", snapshot.snapshot_id));
        let bytes = serde_json::to_vec(snapshot).map_err(|error| {
            AppError::new(
                "STATE_WRITE_FAILED",
                format!("Could not serialize snapshot: {error}"),
            )
        })?;
        std::fs::write(&path, bytes).map_err(|error| {
            AppError::new(
                "STATE_WRITE_FAILED",
                format!("Could not write {}: {error}", path.display()),
            )
        })?;
        self.prune(&path);
        Ok(())
    }

    pub fn load(&self, snapshot_id: &str) -> Result<SnapshotOutput> {
        if !valid_id(snapshot_id) {
            return Err(AppError::invalid(
                "Snapshot id may only contain letters, digits, hyphens and underscores",
            ));
        }
        let path = self.root.join(format!("{snapshot_id}.json"));
        let bytes = std::fs::read(&path).map_err(|_| AppError::snapshot_not_found(snapshot_id))?;
        serde_json::from_slice(&bytes).map_err(|error| {
            AppError::new(
                "STATE_READ_FAILED",
                format!("Could not parse snapshot {snapshot_id}: {error}"),
            )
        })
    }

    fn prune(&self, newest: &Path) {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return;
        };
        let mut older = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path != newest && path.extension().is_some_and(|ext| ext == "json"))
            .filter_map(|path| {
                let modified = std::fs::metadata(&path).ok()?.modified().ok()?;
                Some((modified, path))
            })
            .collect::<Vec<_>>();
        let keep = MAX_SNAPSHOTS - 1;
        if older.len() <= keep {
            return;
        }
        older.sort();
        let excess = older.len() - keep;
        for (_, path) in older.into_iter().take(excess) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
}

pub fn default_home() -> PathBuf {
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        return PathBuf::from(home).join(".agent-android");
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".agent-android")
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
