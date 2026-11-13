//! Tool calls in flight, keyed by JSON-RPC request id, so `notifications/cancelled` can stop one.

use serde_json::Value;
use std::collections::{HashMap, hash_map::Entry};
use std::io;
use std::process::{Child, ChildStderr, ChildStdout, Command};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Output pipes of a started call.
pub(crate) type Pipes = (Option<ChildStdout>, Option<ChildStderr>);

/// Where an admitted call is. A cancelled id stays occupied until its own call finishes.
enum Slot {
    Admitted,
    Running(Child),
    Cancelled,
}

/// Each admitted request id with the state of its call.
#[derive(Default)]
pub(crate) struct Calls(Mutex<HashMap<String, Slot>>);

impl Calls {
    /// Admits `id` before its call starts; false when a call with that id is already in flight.
    pub(crate) fn admit(&self, id: &Value) -> bool {
        match self.lock().entry(id.to_string()) {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                entry.insert(Slot::Admitted);
                true
            }
        }
    }

    /// Spawns `command` for `id` and returns its pipes, or `None` when `id` is not admitted or was cancelled.
    /// The registry stays locked while the child spawns, so a cancel lands either before it, and nothing
    /// starts, or after it, and the child is killed.
    pub(crate) fn start(&self, id: &Value, command: &mut Command) -> Option<io::Result<Pipes>> {
        let mut calls = self.lock();
        let slot = calls.get_mut(&id.to_string())?;
        if matches!(slot, Slot::Cancelled) {
            return None;
        }
        Some(command.spawn().map(|mut child| {
            let pipes = (child.stdout.take(), child.stderr.take());
            *slot = Slot::Running(child);
            pipes
        }))
    }

    /// Takes back the child of `id` after its output closed, or `None` when `id` was cancelled.
    pub(crate) fn take(&self, id: &Value) -> Option<Child> {
        let mut calls = self.lock();
        let slot = calls.get_mut(&id.to_string())?;
        match std::mem::replace(slot, Slot::Admitted) {
            Slot::Running(child) => Some(child),
            other => {
                *slot = other;
                None
            }
        }
    }

    /// Cancels `id`; an unknown id is ignored. A started child is stopped on its own thread by `terminate`.
    pub(crate) fn cancel(&self, id: &Value) {
        let slot = self
            .lock()
            .get_mut(&id.to_string())
            .map(|slot| std::mem::replace(slot, Slot::Cancelled));
        if let Some(Slot::Running(child)) = slot {
            std::thread::spawn(move || terminate(child));
        }
    }

    /// Releases `id` once its call has returned; false when it was cancelled, so no response is due.
    pub(crate) fn finish(&self, id: &Value) -> bool {
        self.lock()
            .remove(&id.to_string())
            .is_some_and(|slot| !matches!(slot, Slot::Cancelled))
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<String, Slot>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Kills a cancelled child process.
fn terminate(mut child: Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
#[path = "calls_tests.rs"]
mod tests;
