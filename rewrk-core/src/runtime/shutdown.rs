//! Shared shutdown signal for coordinating worker shutdown.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Default, Clone)]
pub(crate) struct ShutdownHandle {
    /// A signal flag telling all workers to shutdown.
    should_stop: Arc<AtomicBool>,
}

impl ShutdownHandle {
    /// Checks if the worker should abort processing.
    pub(crate) fn should_abort(&self) -> bool {
        self.should_stop.load(Ordering::Relaxed)
    }

    /// Sets the abort flag across workers.
    pub(crate) fn set_abort(&self) {
        self.should_stop.store(true, Ordering::Relaxed);
    }
}
