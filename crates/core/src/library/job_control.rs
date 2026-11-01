use crate::{
    catalog::Catalog,
    domain::{AppError, ErrorCode, Result},
};
use std::sync::{
    Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Default)]
pub(crate) struct PauseControl {
    requested: Mutex<bool>,
    changed: Condvar,
}
impl PauseControl {
    pub(super) fn request(&self, catalog: &Catalog, id: &str, pause: bool) -> Result<()> {
        let mut requested = self.requested.lock().map_err(|_| lock_error())?;
        state(
            catalog,
            id,
            if pause { "pause_requested" } else { "running" },
            if pause {
                "Pausing at the next safe boundary"
            } else {
                "Resuming work"
            },
        )?;
        *requested = pause;
        self.changed.notify_all();
        Ok(())
    }
}
fn lock_error() -> AppError {
    AppError::new(ErrorCode::WorkerFailed, "Job control is unavailable.")
}

/// Acknowledgement and requests share a lock so rapid resume/pause cannot lose a request.
/// Cancellation has priority and wakes a waiting worker within one bounded timeout.
pub(super) fn boundary(
    catalog: &Catalog,
    id: &str,
    paused: &PauseControl,
    canceled: &AtomicBool,
) -> Result<()> {
    let mut requested = paused.requested.lock().map_err(|_| lock_error())?;
    if !*requested || canceled.load(Ordering::Acquire) {
        return Ok(());
    }
    state(catalog, id, "paused", "Paused; completed work retained")?;
    while *requested && !canceled.load(Ordering::Acquire) {
        let (next, timeout) = paused
            .changed
            .wait_timeout(requested, std::time::Duration::from_millis(100))
            .map_err(|_| lock_error())?;
        requested = next;
        if !timeout.timed_out() && *requested && !canceled.load(Ordering::Acquire) {
            state(catalog, id, "paused", "Paused; completed work retained")?;
        }
    }
    Ok(())
}

pub(super) fn state(catalog: &Catalog, id: &str, state: &str, message: &str) -> Result<()> {
    let (id, state, message) = (id.to_owned(), state.to_owned(), message.to_owned());
    catalog.call(move |db| {
        db.execute("UPDATE scans SET state=?,message=? WHERE id=? AND state IN ('queued','running','pause_requested','paused')", rusqlite::params![state,message,id])?;
        Ok(())
    })
}
