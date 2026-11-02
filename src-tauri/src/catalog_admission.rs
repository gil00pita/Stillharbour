//! Native request lifetime barrier for catalog activation on the next launch.
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use stillharbour_core::domain::{AppError, ErrorCode, Result};
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

#[derive(Default)]
pub struct Admission {
    closing: AtomicBool,
    requests: Arc<RwLock<()>>,
}
fn closed() -> AppError {
    AppError::new(
        ErrorCode::Busy,
        "Catalog recovery has stopped this session. Existing catalogs remain retained; close and reopen Stillharbour to continue.",
    )
}
impl Admission {
    pub fn idle(&self) -> bool {
        self.requests.clone().try_write_owned().is_ok()
    }
    pub fn enter(&self) -> Result<OwnedRwLockReadGuard<()>> {
        if self.closing.load(Ordering::Acquire) {
            return Err(closed());
        }
        let guard = self
            .requests
            .clone()
            .try_read_owned()
            .map_err(|_| closed())?;
        if self.closing.load(Ordering::Acquire) {
            return Err(closed());
        }
        Ok(guard)
    }
    pub fn close_and_drain(&self) -> Result<OwnedRwLockWriteGuard<()>> {
        self.close_with_timeout(Duration::from_secs(30))
    }
    fn close_with_timeout(&self, timeout: Duration) -> Result<OwnedRwLockWriteGuard<()>> {
        if self.closing.swap(true, Ordering::AcqRel) {
            return Err(closed());
        }
        let until = Instant::now() + timeout;
        loop {
            if let Ok(guard) = self.requests.clone().try_write_owned() {
                return Ok(guard);
            }
            if Instant::now() >= until {
                return Err(closed());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closing_drains_an_owned_request_and_refuses_new_admission() {
        let gate = Arc::new(Admission::default());
        let old_request = gate.enter().unwrap();
        let owner = gate.clone();
        let worker = std::thread::spawn(move || {
            let _closed = owner.close_and_drain().unwrap();
        });
        let until = Instant::now() + Duration::from_secs(5);
        while !gate.closing.load(Ordering::Acquire) {
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        assert!(gate.enter().is_err());
        drop(old_request);
        worker.join().unwrap();
        assert!(gate.enter().is_err());
    }
    #[test]
    fn failed_drain_does_not_reopen_admission() {
        let gate = Admission::default();
        let old_request = gate.enter().unwrap();
        assert!(gate.close_with_timeout(Duration::ZERO).is_err());
        drop(old_request);
        assert!(gate.enter().is_err());
    }
}
