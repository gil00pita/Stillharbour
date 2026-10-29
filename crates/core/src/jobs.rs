//! Bounded admission independent of any UI queue. Blocking jobs use worker threads.
pub mod resources;

use crate::domain::{AppError, ErrorCode, Result};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Clone)]
pub struct JobScheduler {
    slots: Arc<Semaphore>,
}

pub struct JobPermit {
    _slot: OwnedSemaphorePermit,
    canceled: Arc<AtomicBool>,
}

impl JobScheduler {
    pub fn new(concurrency: usize) -> Result<Self> {
        if concurrency == 0 || concurrency > 8 {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Worker concurrency must be between one and eight.",
            ));
        }
        Ok(Self {
            slots: Arc::new(Semaphore::new(concurrency)),
        })
    }

    pub fn try_start(&self) -> Result<JobPermit> {
        let slot = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| AppError::new(ErrorCode::Busy, "All worker slots are occupied."))?;
        Ok(JobPermit {
            _slot: slot,
            canceled: Arc::new(AtomicBool::new(false)),
        })
    }
}

impl JobPermit {
    pub fn cancellation(&self) -> Arc<AtomicBool> {
        self.canceled.clone()
    }
    pub fn check_boundary(&self) -> Result<()> {
        if self.canceled.load(Ordering::Relaxed) {
            Err(AppError::new(
                ErrorCode::Busy,
                "The job was canceled at a safe boundary.",
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn admission_is_bounded_and_cancellation_is_explicit() {
        let scheduler = JobScheduler::new(1).unwrap();
        let permit = scheduler.try_start().unwrap();
        assert!(scheduler.try_start().is_err());
        permit.cancellation().store(true, Ordering::Relaxed);
        assert!(permit.check_boundary().is_err());
        drop(permit);
        assert!(scheduler.try_start().is_ok());
    }
}
