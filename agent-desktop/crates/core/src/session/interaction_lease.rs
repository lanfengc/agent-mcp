use std::sync::Arc;

use crate::{AdapterError, Deadline};

pub struct InteractionLease {
    _guard: Option<Arc<dyn Send + Sync>>,
    deadline: Deadline,
    contention_count: u64,
}

impl InteractionLease {
    pub fn guarded(
        deadline: Deadline,
        guard: impl Send + Sync + 'static,
    ) -> Result<Self, AdapterError> {
        Ok(Self {
            _guard: Some(Arc::new(guard)),
            deadline,
            contention_count: 0,
        })
    }

    pub fn deadline(&self) -> Deadline {
        self.deadline
    }

    /// Shares the held interaction ownership with a fresh recovery deadline.
    pub fn with_deadline(&self, deadline: Deadline) -> Self {
        Self {
            _guard: self._guard.clone(),
            deadline,
            contention_count: self.contention_count,
        }
    }

    pub fn contention_count(&self) -> u64 {
        self.contention_count
    }

    /// A guarded lease that reports the contention it earned rather than a
    /// hardcoded zero, so a platform lease that queued behind another holder
    /// before it acquired reports that queueing in the success-path envelope
    /// (`ref_action_poll_state.rs`'s `lease_contention_count`) instead of
    /// understating it.
    pub fn guarded_with_contention(
        deadline: Deadline,
        guard: impl Send + Sync + 'static,
        contention_count: u64,
    ) -> Self {
        Self {
            _guard: Some(Arc::new(guard)),
            deadline,
            contention_count,
        }
    }
}

#[cfg(test)]
#[path = "interaction_lease_tests.rs"]
mod tests;
