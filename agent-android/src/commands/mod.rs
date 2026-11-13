pub mod apps;
pub mod device;
pub mod doctor;
pub mod input;
pub mod screen;
pub mod snapshot;
pub(crate) mod snapshot_parse;
pub mod wait;

use crate::adb::Adb;
use crate::error::Result;
use crate::policy::Policy;
use crate::state::SnapshotStore;
use std::path::Path;

pub struct Ctx<'a> {
    pub adb: &'a Adb,
    pub policy: &'a Policy,
    pub home: &'a Path,
}

impl Ctx<'_> {
    pub fn check_write(&self, action: &str) -> Result<()> {
        self.policy.ensure_write(action)
    }

    pub fn check_destructive(&self, action: &str, confirm: bool) -> Result<()> {
        self.policy.ensure_destructive(action, confirm)
    }

    pub fn store(&self) -> SnapshotStore {
        SnapshotStore::new(self.home)
    }
}
